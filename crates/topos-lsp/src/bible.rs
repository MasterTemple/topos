/*!
A Bible's text: a folder of USX files (one book each, like `JHN.usx`), the same as the Obsidian
plugin reads. Passages render as plain text for hover, and whole books as read-only documents
(`topos-bible://BSB/JHN`) that go-to-definition opens.

A rendered book knows which verse each part of its text is, so references from it can find
every reference to that verse, and where its verse numbers, chapter labels, and words of Jesus
are, so the editor can color them (semantic tokens).
*/

use std::{
    collections::HashMap,
    ops::Range,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use topos_bible::{
    data::{bible_data::BibleData, books::BookId},
    segments::Passage,
};
use topos_bible_markup::{
    Filter, Flattener, Kind, Markup, Node, NodePath, TextOptions,
    flatten::flatten,
    parse,
    verses::{VerseSpan, passage_ranges, slice, verses},
};

use crate::settings::MarkupSettings;

/// The scheme of whole-book documents: `topos-bible://BSB/JHN`
pub const SCHEME: &str = "topos-bible";

/// Hover text is cut after this many bytes (a whole psalm is plenty)
const MAX_HOVER_TEXT: usize = 2000;

/// What colors a part of a rendered book (the semantic token types, in the legend's order)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    VerseNumber,
    ChapterNumber,
    WordsOfJesus,
    Heading,
    Note,
}

impl Token {
    /// The semantic token types, indexed by `Token as u32`
    pub const LEGEND: [&str; 5] = [
        "verseNumber",
        "chapterNumber",
        "wordsOfJesus",
        "heading",
        "note",
    ];
}

/// What a part of a rendered book is
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Verse {
        chapter: u8,
        verse: u8,
        /// The last verse of a bridge (else `verse`)
        last: u8,
    },
    /// A chapter's label
    Chapter(u8),
}

/// A whole book as text
#[derive(Debug)]
pub struct Rendered {
    pub text: String,
    /**
    Each verse and chapter label in order: from its number or label to the next one (or a
    heading), without the whitespace after it
    - Headings and the book's title are between parts
    */
    pub parts: Vec<(Range<usize>, Part)>,
    /// What to color, in order
    pub tokens: Vec<(Range<usize>, Token)>,
}

impl Rendered {
    /// The part at a byte, or the next one after it (a heading goes with the verse after it)
    pub fn part_at(&self, byte: usize) -> Option<Part> {
        let i = self.parts.partition_point(|(range, _)| range.start <= byte);
        if let Some((range, part)) = i.checked_sub(1).and_then(|i| self.parts.get(i))
            && byte <= range.end
        {
            return Some(*part);
        }
        self.parts.get(i).map(|(_, part)| *part)
    }

    /// Where each part of a passage is: from its first verse's number to its last verse's end
    pub fn passage_ranges(&self, passage: &Passage) -> Vec<Range<usize>> {
        // Each verse's range, first (chapter, verse), and last
        type Verse<'a> = (&'a Range<usize>, (u8, u8), (u8, u8));
        let verses: Vec<Verse> = self
            .parts
            .iter()
            .filter_map(|(range, part)| match *part {
                Part::Verse {
                    chapter,
                    verse,
                    last,
                } => Some((range, (chapter, verse), (chapter, last))),
                Part::Chapter(_) => None,
            })
            .collect();
        passage
            .ranges(None)
            .into_iter()
            .filter_map(|r| {
                let (start, end) = (
                    (r.start.chapter, r.start.verse),
                    (r.end.chapter, r.end.verse),
                );
                let within = |v: &&Verse| v.2 >= start && v.1 <= end;
                let first = verses.iter().find(within)?;
                let last = verses.iter().rfind(within)?;
                Some(first.0.start..last.0.end)
            })
            .collect()
    }
}

/// A book's parsed USX and where its verses are
struct Book {
    nodes: Vec<Node>,
    spans: Vec<VerseSpan>,
}

/// A folder of USX files, read as needed
pub struct Bible {
    /// The folder's name, which names it in URIs: `BSB`
    pub name: String,
    /// The folder (canonical), whose USX files workspace searches leave out
    pub folder: PathBuf,
    /// Each book's file
    files: HashMap<BookId, PathBuf>,
    data: BibleData,
    markup: MarkupSettings,
    books: Mutex<HashMap<BookId, Arc<Book>>>,
    rendered: Mutex<HashMap<BookId, Arc<Rendered>>>,
}

/// A book's USFM code from its file name: `JHN.usx`, `jhn.usx`, `44JHNBSB.usx` (Paratext's), or a
/// name with the code as a separate word (`B04___01_John_JHN.usx`)
fn code_from_name(stem: &str, data: &BibleData) -> Option<BookId> {
    let name = stem.to_ascii_uppercase();
    let books = data.books();
    books
        .search_usfm(&name)
        .or_else(|| {
            let digits = name.get(..2)?.bytes().all(|b| b.is_ascii_digit());
            digits.then(|| books.search_usfm(name.get(2..5)?)).flatten()
        })
        .or_else(|| {
            name.split(|c: char| !c.is_ascii_alphanumeric())
                .find_map(|word| books.search_usfm(word))
        })
}

/// The code a USX file declares near its start: `<book code="JHN" ...>`
fn code_from_usx(path: &Path, data: &BibleData) -> Option<BookId> {
    use std::io::Read;
    let mut head = vec![0; 4096];
    let read = std::fs::File::open(path).ok()?.read(&mut head).ok()?;
    let head = String::from_utf8_lossy(&head[..read]);
    let start = head.find("<book")?;
    let rest = &head[start..];
    let code = rest.split("code=\"").nth(1)?.split('"').next()?;
    data.books().search_usfm(&code.to_ascii_uppercase())
}

impl Bible {
    /// The USX files in a folder (not its subfolders)
    pub fn load(folder: &Path, data: BibleData, markup: MarkupSettings) -> Result<Bible, String> {
        let entries =
            std::fs::read_dir(folder).map_err(|e| format!("bible: {}: {e}", folder.display()))?;
        let mut files = HashMap::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let is_usx = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("usx"));
            if !is_usx {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if let Some(id) = code_from_name(stem, &data).or_else(|| code_from_usx(&path, &data)) {
                files.entry(id).or_insert(path);
            }
        }
        if files.is_empty() {
            return Err(format!("bible: no USX books in {}", folder.display()));
        }
        let name = folder
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Bible")
            .to_string();
        Ok(Bible {
            name,
            folder: folder
                .canonicalize()
                .unwrap_or_else(|_| folder.to_path_buf()),
            files,
            data,
            markup,
            books: Mutex::default(),
            rendered: Mutex::default(),
        })
    }

    fn book(&self, id: BookId) -> Option<Arc<Book>> {
        if let Some(book) = self.books.lock().ok()?.get(&id) {
            return Some(book.clone());
        }
        let text = std::fs::read_to_string(self.files.get(&id)?).ok()?;
        let nodes = parse(Markup::Usx, text.trim_start_matches('\u{feff}'));
        let spans = verses(&nodes);
        let book = Arc::new(Book { nodes, spans });
        self.books.lock().ok()?.insert(id, book.clone());
        Some(book)
    }

    /// How text is written: the markup settings, with `{name}` as the book's name
    fn text_options(&self, book: BookId) -> TextOptions {
        let markup = &self.markup;
        let name = self.data.books().get_name(book).map_or("", String::as_str);
        let label = |element: &crate::settings::MarkupElement, default: &str| {
            element.shown(true).then(|| {
                element
                    .display
                    .as_deref()
                    .unwrap_or(default)
                    .replace("{name}", name)
            })
        };
        let mut filter = Filter::all().drop(Kind::Metadata);
        if !markup.headings.shown(true) {
            filter = filter.drop(Kind::Title);
        }
        if !markup.notes.shown(false) {
            filter = filter.drop(Kind::Note);
        }
        TextOptions {
            filter,
            verse_label: label(&markup.verse_numbers, "[{verse}]"),
            chapter_label: label(&markup.chapter_numbers, "{name} {chapter}"),
            ..TextOptions::default()
        }
    }

    /// A passage as text, its parts separated by blank lines, or [`None`] if the Bible doesn't
    /// have it
    pub fn passage_text(&self, passage: &Passage) -> Option<String> {
        let book = self.book(passage.book)?;
        let mut flat = Flattener::new(self.text_options(passage.book));
        let parts = passage_ranges(&book.spans, passage);
        for r in &parts {
            let (first, last) = (&book.spans[r.start], &book.spans[r.end - 1]);
            flat.push(&slice(&book.nodes, &first.start, Some(&last.end)));
        }
        let flat = flat.finish();
        let text: Vec<&str> = flat
            .entries
            .iter()
            .map(|r| &flat.text[r.clone()])
            .filter(|t| !t.is_empty())
            .collect();
        let mut text = text.join("\n\n");
        if text.is_empty() {
            return None;
        }
        if text.len() > MAX_HOVER_TEXT {
            let mut cut = MAX_HOVER_TEXT;
            while !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
            text.push_str(" …");
        }
        Some(text)
    }

    /// The whole book as text, rendered once (until the settings change)
    pub fn render(&self, id: BookId) -> Option<Arc<Rendered>> {
        if let Some(rendered) = self.rendered.lock().ok()?.get(&id) {
            return Some(rendered.clone());
        }
        let book = self.book(id)?;
        let rendered = Arc::new(self.render_book(id, &book));
        self.rendered.lock().ok()?.insert(id, rendered.clone());
        Some(rendered)
    }

    /// The whole tree at once (a verse can start in one paragraph and end in the next), with
    /// each verse found by its marker
    fn render_book(&self, id: BookId, book: &Book) -> Rendered {
        let flat = flatten(&book.nodes, &self.text_options(id));
        let verses: HashMap<&NodePath, &VerseSpan> =
            book.spans.iter().map(|s| (&s.marker, s)).collect();
        // Each verse's and chapter's start, and where headings start (which end the part before)
        let mut starts: Vec<(usize, Option<Part>)> = vec![];
        for span in &flat.spans {
            let start = span.range.start;
            match span.kind {
                Kind::Verse => {
                    if let Some(v) = verses.get(&span.path) {
                        let part = Part::Verse {
                            chapter: v.chapter,
                            verse: v.verse,
                            last: v.last,
                        };
                        starts.push((start, Some(part)));
                    }
                }
                // Its number is the next verse's chapter
                Kind::Chapter if !span.range.is_empty() => {
                    starts.push((start, Some(Part::Chapter(0))))
                }
                Kind::Title => starts.push((start, None)),
                _ => {}
            }
        }
        starts.sort_by_key(|(start, _)| *start);
        let mut parts: Vec<(Range<usize>, Part)> = vec![];
        for (i, &(start, part)) in starts.iter().enumerate() {
            let Some(part) = part else { continue };
            let end = starts.get(i + 1).map_or(flat.text.len(), |(next, _)| *next);
            let end = start + flat.text[start..end].trim_end().len();
            parts.push((start..end, part));
        }
        let mut chapter = None;
        for (_, part) in parts.iter_mut().rev() {
            match part {
                Part::Verse { chapter: c, .. } => chapter = Some(*c),
                Part::Chapter(c) => *c = chapter.unwrap_or_default(),
            }
        }
        let woj = self.markup.words_of_jesus.shown(true);
        let mut tokens: Vec<(Range<usize>, Token)> = flat
            .spans
            .iter()
            .filter(|s| !s.range.is_empty())
            .filter_map(|s| {
                let token = match s.kind {
                    Kind::Verse => Token::VerseNumber,
                    Kind::Chapter => Token::ChapterNumber,
                    Kind::WordsOfJesus if woj => Token::WordsOfJesus,
                    Kind::Title => Token::Heading,
                    Kind::Note => Token::Note,
                    _ => return None,
                };
                Some((s.range.clone(), token))
            })
            .collect();
        tokens.sort_by_key(|(range, _)| (range.start, range.end));
        Rendered {
            text: flat.text,
            parts,
            tokens,
        }
    }

    /// A book's document: `topos-bible://BSB/JHN`
    pub fn uri(&self, book: BookId) -> Option<String> {
        let code = self.data.books().get_usfm(book)?;
        Some(format!("{SCHEME}://{}/{code}", encode(&self.name)))
    }

    /// The book a document is, if it is one of this Bible's
    pub fn book_of(&self, uri: &str) -> Option<BookId> {
        let rest = uri.strip_prefix(SCHEME)?.strip_prefix("://")?;
        let (name, code) = rest.split_once('/')?;
        let code = code.split(['#', '?']).next()?;
        (decode(name) == self.name)
            .then(|| self.data.books().search_usfm(&code.to_ascii_uppercase()))
            .flatten()
    }

    /// A book's name: `John`
    pub fn book_name(&self, book: BookId) -> Option<&str> {
        self.data.books().get_name(book).map(String::as_str)
    }
}

/// Whether a URI is a whole-book document
pub fn is_book_uri(uri: &str) -> bool {
    uri.starts_with(SCHEME) && uri[SCHEME.len()..].starts_with("://")
}

fn encode(name: &str) -> String {
    let mut encoded = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(byte) = text
                .get(i + 1..i + 3)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            decoded.push(byte);
            i += 3;
            continue;
        }
        decoded.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}
