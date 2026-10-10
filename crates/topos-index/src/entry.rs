//! What the index keeps about one file: its references, compactly, and for EPUBs the details
//! that are only needed when a reference is shown (its CFI and the text around it)

use serde::{Deserialize, Serialize};
use topos_bible::{matcher::BibleMatcher, segments::Passage};

/// How offsets and columns are counted
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    /// UTF-8 bytes (Rust, C)
    #[default]
    Byte,
    /// Unicode scalar values (Python)
    Char,
    /// UTF-16 code units (JavaScript, Kotlin, Swift's `utf16` view)
    Utf16,
}

/// Which version of a file an entry describes
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub size: u64,
    /// Modification time in milliseconds since 1970. Syncing can change it on another device,
    /// so a different time with the same size is checked with `hash` (see [`crate::Index::check`])
    pub mtime: u64,
    /// [`text_hash`] of a text file's contents ([`None`] for files that aren't read as text, like
    /// EPUBs)
    pub hash: Option<u64>,
}

/// FNV-1a over the text's UTF-8 bytes: the same in every build and on every platform
pub fn text_hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// One reference in a file
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    pub passage: Passage,
    /// Where it starts in the file's text, and its length, in the index's [`Unit`] (UTF-16 for
    /// EPUBs, through the book's text: each content document's, a blank line apart)
    pub start: u32,
    pub len: u32,
    /// 1-based line and column of its start
    pub line: u32,
    pub column: u32,
    /// For EPUBs: which content document it's in
    pub section: Option<Section>,
}

impl Reference {
    pub fn end(&self) -> u32 {
        self.start.saturating_add(self.len)
    }
}

/// A content document of an EPUB
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// Its position in the spine
    pub spine: u32,
    /// Its chapter title, as an index into [`FileEntry::chapters`]
    pub chapter: Option<u32>,
}

/// Everything the index keeps about one version of a file
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub stamp: Stamp,
    /// When the entry was made, in milliseconds since 1970 (the newest wins when two devices
    /// indexed the same version)
    pub written: u64,
    /// EPUB chapter titles, which [`Section::chapter`] points into (each is kept once)
    pub chapters: Vec<String>,
    /// In the order they appear in the file
    #[serde(with = "crate::codec")]
    pub refs: Vec<Reference>,
    /// For EPUBs: the name of its [`Detail`], stored separately and loaded when needed
    pub detail: Option<String>,
}

impl FileEntry {
    pub fn new(stamp: Stamp, written: u64, refs: Vec<Reference>) -> Self {
        Self {
            stamp,
            written,
            chapters: vec![],
            refs,
            detail: None,
        }
    }

    pub fn chapter(&self, reference: &Reference) -> Option<&str> {
        let index = reference.section?.chapter?;
        self.chapters.get(index as usize).map(String::as_str)
    }
}

/// What an EPUB's references need only when they're shown or opened, in the order of
/// [`FileEntry::refs`]
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Detail {
    pub refs: Vec<RefDetail>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefDetail {
    /// Where it is, as a range CFI
    pub cfi: String,
    /// About [`SNIPPET_CHARS`] characters of its paragraph around it, with `…` where it was cut
    pub context: String,
    /// Where the reference starts in `context`, in UTF-16 code units
    pub at: u32,
}

/// Roughly how many characters of context a [`RefDetail`] keeps on each side of a reference
pub const SNIPPET_CHARS: usize = 100;

/**
The text around a reference in a paragraph, and where the reference starts in it (both in UTF-16
code units): up to [`SNIPPET_CHARS`] characters on each side, cut at a space when one is near,
with `…` where text was left out
*/
pub fn snippet(paragraph: &str, start_utf16: u32, len_utf16: u32) -> (String, u32) {
    let chars: Vec<char> = paragraph.chars().collect();
    // Character indexes of the reference
    let (mut start, mut end) = (chars.len(), chars.len());
    let mut utf16 = 0u32;
    for (i, c) in chars.iter().enumerate() {
        if utf16 >= start_utf16 && start == chars.len() {
            start = i;
        }
        if utf16 >= start_utf16.saturating_add(len_utf16) {
            end = i;
            break;
        }
        utf16 += c.len_utf16() as u32;
    }
    let end = end.max(start);
    let is_space = |i: usize| chars.get(i).is_some_and(|c| c.is_whitespace());
    // Up to SNIPPET_CHARS on each side, moved in to a word boundary when one is within 20
    let mut from = start.saturating_sub(SNIPPET_CHARS);
    if from > 0
        && let Some(space) = (from..(from + 20).min(start)).find(|&i| is_space(i))
    {
        from = space + 1;
    }
    let mut to = (end + SNIPPET_CHARS).min(chars.len());
    if to < chars.len()
        && let Some(space) = (end.max(to.saturating_sub(20))..to)
            .rev()
            .find(|&i| is_space(i))
    {
        to = space;
    }
    let mut text = String::new();
    if from > 0 {
        text.push('…');
    }
    let before: String = chars[from..start].iter().collect();
    text.push_str(before.trim_start());
    let at = text.encode_utf16().count() as u32;
    let rest: String = chars[start..to].iter().collect();
    text.push_str(rest.trim_end());
    if to < chars.len() {
        text.push('…');
    }
    (text, at)
}

/// Converts increasing byte offsets in one text to another unit, counting each byte once
pub(crate) struct Offsets<'a> {
    text: &'a str,
    unit: Unit,
    byte: usize,
    at: usize,
}

impl<'a> Offsets<'a> {
    pub(crate) fn new(text: &'a str, unit: Unit) -> Self {
        Self {
            text,
            unit,
            byte: 0,
            at: 0,
        }
    }

    pub(crate) fn of(&mut self, byte: usize) -> u32 {
        let byte = byte.min(self.text.len());
        if byte < self.byte {
            self.byte = 0;
            self.at = 0;
        }
        let part = self.text.get(self.byte..byte).unwrap_or_default();
        self.at += match self.unit {
            Unit::Byte => part.len(),
            Unit::Char => part.chars().count(),
            Unit::Utf16 => part.encode_utf16().count(),
        };
        self.byte = byte;
        self.at as u32
    }
}

/// The references in a text, with offsets in `unit`
pub fn text_refs(matcher: &BibleMatcher, text: &str, unit: Unit) -> Vec<Reference> {
    let mut offsets = Offsets::new(text, unit);
    let mut lines = Offsets::new(text, unit);
    matcher
        .search(text)
        .into_iter()
        .map(|m| {
            let bytes = m.location.bytes;
            let line_start = text[..bytes.start].rfind('\n').map_or(0, |i| i + 1);
            let start = offsets.of(bytes.start);
            let end = offsets.of(bytes.end);
            Reference {
                passage: m.psg,
                start,
                len: end - start,
                line: m.location.start.line as u32,
                column: start - lines.of(line_start) + 1,
                section: None,
            }
        })
        .collect()
}

/// An entry for a text file: its references and its [`text_hash`]
pub fn text_entry(
    matcher: &BibleMatcher,
    text: &str,
    unit: Unit,
    size: u64,
    mtime: u64,
    written: u64,
) -> FileEntry {
    let stamp = Stamp {
        size,
        mtime,
        hash: Some(text_hash(text)),
    };
    FileEntry::new(stamp, written, text_refs(matcher, text, unit))
}

/// Builds an EPUB's (or a PDF's) entry and details one reference at a time
#[derive(Debug, Default)]
pub struct EpubEntryBuilder {
    chapters: Vec<String>,
    refs: Vec<Reference>,
    details: Vec<RefDetail>,
}

/// One reference in an EPUB, as [`EpubEntryBuilder::push`] takes it
#[derive(Clone, Debug)]
pub struct EpubRef<'a> {
    pub passage: Passage,
    /// Its UTF-16 range in the book's text
    pub start: u32,
    pub end: u32,
    /// 1-based line in the book's text, and UTF-16 column in that line
    pub line: u32,
    pub column: u32,
    pub spine: u32,
    pub chapter: Option<&'a str>,
    pub cfi: String,
    /// The line it starts on (its paragraph)
    pub line_text: &'a str,
}

impl EpubEntryBuilder {
    pub fn push(&mut self, r: EpubRef<'_>) {
        let chapter = r.chapter.map(|title| {
            let index = self.chapters.iter().position(|c| c == title);
            index.unwrap_or_else(|| {
                self.chapters.push(title.to_string());
                self.chapters.len() - 1
            }) as u32
        });
        let len = r.end.saturating_sub(r.start);
        let (context, at) = snippet(r.line_text, r.column.saturating_sub(1), len);
        self.refs.push(Reference {
            passage: r.passage,
            start: r.start,
            len,
            line: r.line,
            column: r.column,
            section: Some(Section {
                spine: r.spine,
                chapter,
            }),
        });
        self.details.push(RefDetail {
            cfi: r.cfi,
            context,
            at,
        });
    }

    /// The entry (named `detail` for its details) and its details
    pub fn finish(self, stamp: Stamp, written: u64, detail: String) -> (FileEntry, Detail) {
        let entry = FileEntry {
            stamp,
            written,
            chapters: self.chapters,
            refs: self.refs,
            detail: Some(detail),
        };
        (entry, Detail { refs: self.details })
    }
}

/// An EPUB's entry and details from its references (the offsets are UTF-16)
#[cfg(feature = "epub")]
pub fn epub_entry(
    matches: Vec<topos_bible::matcher::BibleMatch<topos_bible_formats::epub::CfiLocation>>,
    stamp: Stamp,
    written: u64,
    detail: String,
) -> (FileEntry, Detail) {
    let mut builder = EpubEntryBuilder::default();
    for m in matches {
        let l = m.location;
        builder.push(EpubRef {
            passage: m.psg,
            start: l.text_utf16.start as u32,
            end: l.text_utf16.end as u32,
            line: l.line as u32,
            column: l.utf16_column as u32,
            spine: l.spine_index as u32,
            chapter: l.chapter.as_deref(),
            cfi: l.cfi,
            line_text: &l.line_text,
        });
    }
    builder.finish(stamp, written, detail)
}

/**
An EPUB's entry and details from the book itself (its bytes, read anywhere: WebAssembly can't
open files), named for its path; the error says why it couldn't be read
*/
#[cfg(feature = "epub")]
pub fn epub_file_entry(
    matcher: &BibleMatcher,
    path: &str,
    bytes: &[u8],
    stamp: Stamp,
    written: u64,
) -> Result<(FileEntry, Option<Detail>), String> {
    use topos_bible_formats::epub::{CfiOptions, search_epub_reader};
    let matches = search_epub_reader(matcher, std::io::Cursor::new(bytes), CfiOptions::default())
        .map_err(|e| e.to_string())?;
    let (entry, detail) = epub_entry(matches, stamp, written, detail_name(path, &stamp));
    // A book without references needs no details
    Ok(if entry.refs.is_empty() {
        (
            FileEntry {
                detail: None,
                ..entry
            },
            None,
        )
    } else {
        (entry, Some(detail))
    })
}

/**
A PDF's entry and details from its bytes (with MuPDF, so not in WebAssembly), named for its path;
the error says why it couldn't be read
- Like an EPUB's: offsets are UTF-16 through the document's text (each page's), a reference's
  [`Section::spine`] is its page's index (page 1 is 0), and its detail's `cfi` is where it is
  as PDF++ links write it: `page=3&rect=72,708,126,720` (the first line it covers, in PDF
  coordinates from the bottom left)
*/
#[cfg(feature = "pdf")]
pub fn pdf_file_entry(
    matcher: &BibleMatcher,
    path: &str,
    bytes: &[u8],
    stamp: Stamp,
    written: u64,
) -> Result<(FileEntry, Option<Detail>), String> {
    use topos_bible_formats::{SearchFormat as _, pdf::PDFLocation};
    let doc = mupdf::Document::from_bytes(bytes, "application/pdf").map_err(|e| e.to_string())?;
    let matches = matcher
        .search_format::<PDFLocation>(&doc)
        .map_err(|e| e.to_string())?;
    let mut builder = EpubEntryBuilder::default();
    for m in matches {
        let l = m.location;
        builder.push(EpubRef {
            passage: m.psg,
            start: l.text_utf16.start as u32,
            end: l.text_utf16.end as u32,
            line: l.line as u32,
            column: l.utf16_column as u32,
            spine: l.page.saturating_sub(1) as u32,
            chapter: None,
            cfi: pdf_locator(l.page, l.rects.first()),
            line_text: &l.line_text,
        });
    }
    let (entry, detail) = builder.finish(stamp, written, detail_name(path, &stamp));
    Ok(if entry.refs.is_empty() {
        (
            FileEntry {
                detail: None,
                ..entry
            },
            None,
        )
    } else {
        (entry, Some(detail))
    })
}

/// Where a reference in a PDF is, as PDF++ links write it: `page=3&rect=72,708,126,720`
#[cfg(feature = "pdf")]
pub fn pdf_locator(page: usize, rect: Option<&topos_bible_formats::pdf::PDFRect>) -> String {
    match rect {
        Some(r) => format!(
            "page={page}&rect={:.0},{:.0},{:.0},{:.0}",
            r.x,
            r.y,
            r.x + r.w,
            r.y + r.h
        ),
        None => format!("page={page}"),
    }
}

/// A name for an EPUB's details, from its path and version
pub fn detail_name(path: &str, stamp: &Stamp) -> String {
    let key = format!("{path}\0{}\0{}", stamp.size, stamp.mtime);
    format!("{:016x}", text_hash(&key))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-page PDF with a line of text per entry
    #[cfg(feature = "pdf")]
    fn pdf(lines: &[&str]) -> Vec<u8> {
        let mut content = String::from("BT /F1 12 Tf 72 720 Td 14 TL\n");
        for line in lines {
            content.push_str(&format!("({line}) Tj T*\n"));
        }
        content.push_str("ET");
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        ];
        let mut out = String::from("%PDF-1.4\n");
        let mut offsets = vec![];
        for (i, object) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.push_str(&format!("{} 0 obj\n{object}\nendobj\n", i + 1));
        }
        let xref = out.len();
        out.push_str(&format!(
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len() + 1
        ));
        for offset in offsets {
            out.push_str(&format!("{offset:010} 00000 n \n"));
        }
        out.push_str(&format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        ));
        out.into_bytes()
    }

    #[cfg(feature = "pdf")]
    #[test]
    fn pdf_entries_have_pages_and_locators() {
        let matcher = BibleMatcher::default();
        let stamp = Stamp {
            size: 1,
            mtime: 2,
            hash: None,
        };
        let bytes = pdf(&["Notes", "Read John 3:16 today."]);
        let (entry, detail) = pdf_file_entry(&matcher, "a.pdf", &bytes, stamp, 3).unwrap();
        let detail = detail.unwrap();
        assert_eq!(entry.refs.len(), 1);
        let r = &entry.refs[0];
        assert_eq!(r.section.map(|s| s.spine), Some(0));
        assert_eq!((r.line, r.column, r.len), (2, 6, 9));
        assert!(detail.refs[0].cfi.starts_with("page=1&rect="), "{detail:?}");
        assert!(detail.refs[0].context.contains("John 3:16"));
        // Not a PDF
        assert!(pdf_file_entry(&matcher, "b.pdf", b"text", stamp, 3).is_err());
    }

    #[test]
    fn snippets_keep_the_text_around_a_reference() {
        let (text, at) = snippet("See John 3:16 here.", 4, 9);
        assert_eq!((text.as_str(), at), ("See John 3:16 here.", 4));

        let long = format!("{} John 3:16 {}", "word ".repeat(60), "more ".repeat(60));
        let start = long.find("John").unwrap() as u32;
        let (text, at) = snippet(&long, start, 9);
        assert!(text.starts_with('…') && text.ends_with('…'), "{text}");
        let chars: Vec<u16> = text.encode_utf16().collect();
        assert_eq!(
            String::from_utf16(&chars[at as usize..at as usize + 9]).unwrap(),
            "John 3:16"
        );
        // Cut at spaces, not inside words
        assert!(
            text.starts_with("…word ") && text.ends_with(" more…"),
            "{text}"
        );
        assert!(text.chars().count() < 2 * SNIPPET_CHARS + 20);
    }

    #[test]
    fn snippet_offsets_are_utf16() {
        let paragraph = "📖📖 Jn 3:16";
        let start = "📖📖 ".encode_utf16().count() as u32;
        let (text, at) = snippet(paragraph, start, 7);
        assert_eq!((text.as_str(), at), (paragraph, start));
    }

    #[test]
    fn text_references_in_each_unit() {
        let matcher = BibleMatcher::default();
        let text = "é\n📖 Jn 3:16";
        let one = |unit| {
            let r = &text_refs(&matcher, text, unit)[0];
            (r.start, r.len, r.line, r.column)
        };
        assert_eq!(one(Unit::Byte), (8, 7, 2, 6));
        assert_eq!(one(Unit::Char), (4, 7, 2, 3));
        assert_eq!(one(Unit::Utf16), (5, 7, 2, 4));
    }
}
