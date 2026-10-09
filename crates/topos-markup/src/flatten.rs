//! Plain text with a map back to the tree: what plain-text rendering, search, and editor
//! highlighting are built on.

use std::ops::Range;

use crate::{Element, Kind, Milestone, Node, NodePath, TextOptions, filter::Action};

/// A position in a tree: a byte offset in the text node at `path`, in entry `entry`
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Point {
    /// Which pushed entry (0 for the first), see [`Flattener::push`]
    pub entry: usize,
    /// The text node
    pub path: NodePath,
    /// A byte offset into the text node's text
    pub offset: usize,
}

/// Where an element's content is in the flattened text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Byte range in [`Flat::text`], without surrounding whitespace; milestones (verse and
    /// chapter markers) are empty ranges where they are
    pub range: Range<usize>,
    pub kind: Kind,
    pub entry: usize,
    /// The element; for words of Jesus between milestones, the start milestone (or the first
    /// entry's top level when it started in an earlier entry)
    pub path: NodePath,
}

/// A run of [`Flat::text`] copied from a text node
#[derive(Debug, Clone, PartialEq, Eq)]
struct Source {
    /// Where the run is in the flattened text
    start: usize,
    end: usize,
    entry: usize,
    path: NodePath,
    /// Where the run starts in the text node
    offset: usize,
}

impl Source {
    fn point(&self, byte: usize) -> Point {
        Point {
            entry: self.entry,
            path: self.path.clone(),
            offset: self.offset + byte.saturating_sub(self.start),
        }
    }
}

/**
Flattened text
- Runs of whitespace are one space, and line breaks are kept (at most one blank line); the text
  has no leading or trailing whitespace
- Text that isn't in the tree (note brackets, Strong's numbers) has no source
*/
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Flat {
    pub text: String,
    /// Every kept element and milestone, in document order
    pub spans: Vec<Span>,
    /// Where each pushed entry's text is, without the whitespace around it (an empty range where
    /// it would be, for an entry with no text)
    pub entries: Vec<Range<usize>>,
    /// The runs copied from text nodes, in text order
    sources: Vec<Source>,
}

impl Flat {
    /// Where a byte of [`Flat::text`] came from, or [`None`] for added text and whitespace
    /// between runs
    pub fn source(&self, byte: usize) -> Option<Point> {
        let index = self.sources.partition_point(|s| s.end <= byte);
        let source = self.sources.get(index)?;
        (source.start <= byte && byte < source.end).then(|| source.point(byte))
    }

    /**
    Where a range of [`Flat::text`] came from: the start of its first source text and the end of
    its last, or [`None`] if none of it is from the tree
    - The end is exclusive, like the range
    */
    pub fn source_range(&self, range: Range<usize>) -> Option<(Point, Point)> {
        let first_index = self.sources.partition_point(|s| s.end <= range.start);
        let overlapping: Vec<&Source> = self
            .sources
            .iter()
            .skip(first_index)
            .take_while(|s| s.start < range.end)
            .filter(|s| s.end > range.start)
            .collect();
        let first = overlapping.first()?;
        let last = overlapping.last()?;
        Some((
            first.point(range.start.max(first.start)),
            last.point(range.end.min(last.end)),
        ))
    }

    /// The spans that contain a byte range, outermost first
    pub fn spans_at(&self, range: Range<usize>) -> impl Iterator<Item = &Span> {
        self.spans
            .iter()
            .filter(move |s| s.range.start <= range.start && range.end <= s.range.end)
    }

    /// The last milestone or element of a kind that starts at or before a byte, like the
    /// [`Kind::Verse`] a search match is in
    pub fn last_before(&self, kind: Kind, byte: usize) -> Option<&Span> {
        let mut best: Option<&Span> = None;
        for span in &self.spans {
            if span.kind == kind
                && span.range.start <= byte
                && best.is_none_or(|b| span.range.start >= b.range.start)
            {
                best = Some(span);
            }
        }
        best
    }
}

/// The separator waiting to be written before the next piece
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Separator {
    None,
    Space,
    Breaks(usize),
}

/// Words of Jesus that are not one element: between milestones, or continuing from an unclosed
/// element or an earlier entry
#[derive(Debug, Clone)]
struct OpenWords {
    start: usize,
    entry: usize,
    path: NodePath,
    id: Option<String>,
}

/// Where a pushed entry's text is so far
#[derive(Debug, Clone)]
struct EntryText {
    start: Option<usize>,
    end: usize,
}

/**
Flattens entries one after another, carrying words of Jesus that continue from one entry into the
next (with `sID`/`eID` milestones or an unclosed `<q>`)

```
use topos_bible_markup::{Flattener, Kind, Markup, TextOptions, parse};

let mut flat = Flattener::new(TextOptions::default());
flat.push(&parse(Markup::Osis, r#"He said, <q who="Jesus">Follow"#));
flat.push(&parse(Markup::Osis, "me.</q> And they did."));
let flat = flat.finish();
assert_eq!(flat.text, "He said, Follow me. And they did.");
let red: Vec<&str> = flat.spans.iter()
    .filter(|s| s.kind == Kind::WordsOfJesus)
    .map(|s| &flat.text[s.range.clone()])
    .collect();
assert_eq!(red, ["Follow", "me."]);
```
*/
#[derive(Debug, Clone)]
pub struct Flattener {
    opts: TextOptions,
    text: String,
    separator: Separator,
    /// Space requests are ignored until the next piece (after a template's opening text)
    glued: bool,
    spans: Vec<Span>,
    entries: Vec<EntryText>,
    sources: Vec<Source>,
    words: Option<OpenWords>,
    /// The book and chapter of the last chapter marker, for labels
    book: Option<String>,
    chapter: Option<String>,
}

/// A marker's id read as book, chapter, and verse
#[derive(Default)]
struct MarkerId<'a> {
    book: Option<&'a str>,
    chapter: Option<&'a str>,
    verse: Option<&'a str>,
}

fn non_empty(s: &str) -> Option<&str> {
    (!s.is_empty()).then_some(s)
}

impl<'a> MarkerId<'a> {
    fn of(e: &'a Element) -> Self {
        let Some(id) = e.attr("sID").or_else(|| e.attr("osisID")) else {
            return MarkerId::default();
        };
        if let Some((book, rest)) = id.split_once(' ')
            && !rest.contains('.')
        {
            let (chapter, verse) = match rest.split_once(':') {
                Some((c, v)) => (c, Some(v)),
                None => (rest, None),
            };
            return MarkerId {
                book: non_empty(book),
                chapter: non_empty(chapter),
                verse: verse.and_then(non_empty),
            };
        }
        let word = id.split_whitespace().next().unwrap_or("");
        let mut parts = word.splitn(3, '.');
        MarkerId {
            book: parts.next().and_then(non_empty),
            chapter: parts.next().and_then(non_empty),
            verse: parts.next().and_then(non_empty),
        }
    }
}

/// A marker's number as written
fn marker_number(e: &Element) -> Option<&str> {
    e.attr("number").or_else(|| e.attr("n"))
}

/// Replaces `{book}`, `{chapter}`, and `{verse}` in one pass
fn fill_label(template: &str, book: &str, chapter: &str, verse: &str) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let (before, from) = rest.split_at(open);
        out.push_str(before);
        let replaced = [("{book}", book), ("{chapter}", chapter), ("{verse}", verse)]
            .iter()
            .find(|(p, _)| from.starts_with(p))
            .map(|(p, v)| (p.len(), *v));
        match replaced {
            Some((len, value)) => {
                out.push_str(value);
                rest = from.get(len..).unwrap_or("");
            }
            None => {
                out.push('{');
                rest = from.get(1..).unwrap_or("");
            }
        }
    }
    out.push_str(rest);
    out
}

/// A template split at its first `{text}`
fn split_template(template: &str) -> (&str, &str) {
    template.split_once("{text}").unwrap_or((template, ""))
}

impl Flattener {
    pub fn new(opts: TextOptions) -> Self {
        Flattener {
            opts,
            text: String::new(),
            separator: Separator::None,
            glued: false,
            spans: Vec::new(),
            entries: Vec::new(),
            sources: Vec::new(),
            words: None,
            book: None,
            chapter: None,
        }
    }

    /// Adds an entry (a verse, say), separated from the previous one by a space
    pub fn push(&mut self, nodes: &[Node]) -> &mut Self {
        if !self.entries.is_empty() {
            self.space();
            let entry = self.entries.len();
            if let Some(words) = self.words.take() {
                self.spans.push(Span {
                    range: words.start..self.text.len(),
                    kind: Kind::WordsOfJesus,
                    entry: words.entry,
                    path: words.path,
                });
                self.words = Some(OpenWords {
                    start: self.text.len(),
                    entry,
                    path: NodePath::default(),
                    id: words.id,
                });
            }
        }
        self.entries.push(EntryText {
            start: None,
            end: 0,
        });
        let mut path = Vec::new();
        self.nodes(nodes, &mut path);
        self
    }

    pub fn finish(mut self) -> Flat {
        if let Some(words) = self.words.take() {
            let (_, after) = self.template(Kind::WordsOfJesus);
            let after = after.to_string();
            self.write_after(&after);
            self.spans.push(Span {
                range: words.start..self.text.len(),
                kind: Kind::WordsOfJesus,
                entry: words.entry,
                path: words.path,
            });
        }
        let len = self.text.len();
        let entries = self
            .entries
            .iter()
            .map(|e| match e.start {
                Some(start) => start..e.end,
                None => len..len,
            })
            .collect();
        let text = self.text;
        let spans = self
            .spans
            .into_iter()
            .map(|mut span| {
                span.range = trim(&text, span.range);
                span
            })
            .collect();
        Flat {
            text,
            spans,
            entries,
            sources: self.sources,
        }
    }

    fn entry(&self) -> usize {
        self.entries.len().saturating_sub(1)
    }

    fn space(&mut self) {
        if !self.glued && self.separator == Separator::None {
            self.separator = Separator::Space;
        }
    }

    fn line_break(&mut self) {
        if !self.opts.line_breaks {
            self.space();
            return;
        }
        self.separator = match self.separator {
            Separator::None | Separator::Space => Separator::Breaks(1),
            Separator::Breaks(n) => Separator::Breaks(n.saturating_add(1)),
        };
    }

    /// Records that text from `start` to the end was written in the current entry
    fn wrote(&mut self, start: usize) {
        let end = self.text.len();
        let current = self.entries.len().saturating_sub(1);
        for (i, entry) in self.entries.iter_mut().enumerate() {
            if i == current {
                entry.start.get_or_insert(start);
                entry.end = end;
            } else if entry.start.is_none() {
                entry.start = Some(start);
                entry.end = start;
            }
        }
    }

    /// Writes a piece, with the separator before it
    fn piece(&mut self, piece: &str, source: Option<(&[usize], usize)>) {
        if piece.is_empty() {
            return;
        }
        if !self.text.is_empty() {
            match self.separator {
                Separator::None => {}
                Separator::Space => self.text.push(' '),
                Separator::Breaks(1 | 2) => self.text.push('\n'),
                Separator::Breaks(_) => self.text.push_str("\n\n"),
            }
        }
        self.separator = Separator::None;
        self.glued = false;
        let start = self.text.len();
        self.text.push_str(piece);
        self.wrote(start);
        if let Some((path, offset)) = source {
            self.sources.push(Source {
                start,
                end: self.text.len(),
                entry: self.entry(),
                path: NodePath(path.to_vec()),
                offset,
            });
        }
    }

    /// Writes a template's closing text right after what was written, keeping the separator
    /// pending; like any piece, it ends the glue of an opening text
    fn write_after(&mut self, after: &str) {
        if after.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(after);
        self.glued = false;
        self.wrote(start);
    }

    /// Writes a template's opening text and glues what follows to it
    fn write_before(&mut self, before: &str) {
        if before.is_empty() {
            return;
        }
        self.piece(before, None);
        self.glued = true;
    }

    fn template(&self, kind: Kind) -> (&str, &str) {
        match self.opts.templates.get(&kind) {
            Some(t) => split_template(t),
            None if kind == Kind::Note => ("[", "]"),
            None => ("", ""),
        }
    }

    fn nodes(&mut self, nodes: &[Node], path: &mut Vec<usize>) {
        for (i, node) in nodes.iter().enumerate() {
            path.push(i);
            match node {
                Node::Text(t) => self.text_node(t, path),
                Node::Element(e) => self.element(e, path),
            }
            path.pop();
        }
    }

    fn text_node(&mut self, text: &str, path: &[usize]) {
        let mut run_start: Option<usize> = None;
        for (i, c) in text.char_indices() {
            if c.is_whitespace() {
                if let Some(start) = run_start.take() {
                    self.piece(text.get(start..i).unwrap_or(""), Some((path, start)));
                }
                self.space();
            } else if run_start.is_none() {
                run_start = Some(i);
            }
        }
        if let Some(start) = run_start {
            self.piece(text.get(start..).unwrap_or(""), Some((path, start)));
        }
    }

    fn element(&mut self, e: &Element, path: &mut Vec<usize>) {
        if e.kind == Kind::Chapter && e.milestone != Some(Milestone::End) {
            self.chapter_marker(e);
        }
        match self.opts.filter.action(e) {
            Action::Drop => {}
            Action::Unwrap => self.nodes(&e.children, path),
            Action::Keep => {
                let start = self.text.len();
                if self.open(e, path) {
                    self.nodes(&e.children, path);
                    self.close(e, path, start);
                }
            }
        }
    }

    /// Updates the current book and chapter from a chapter marker
    #[inline(never)]
    fn chapter_marker(&mut self, e: &Element) {
        let id = MarkerId::of(e);
        if let Some(book) = id.book {
            self.book = Some(book.to_string());
        }
        self.chapter = marker_number(e).or(id.chapter).map(str::to_string);
    }

    /**
    Writes what comes before a kept element's contents; false when the element is done (its
    contents aren't written, or it has none to write)
    - Kept apart from the recursion, like [`Flattener::close`], so each level of nesting takes
      little stack
    */
    #[inline(never)]
    fn open(&mut self, e: &Element, path: &[usize]) -> bool {
        let start = self.text.len();
        match e.kind {
            Kind::WordsOfJesus | Kind::Quote if e.milestone.is_some() => {
                self.words_milestone(e, path);
                false
            }
            Kind::Verse | Kind::Chapter | Kind::Milestone if e.children.is_empty() => {
                if e.kind != Kind::Milestone && e.milestone != Some(Milestone::End) {
                    self.label(e);
                }
                self.push_span(start, e.kind, path);
                false
            }
            Kind::Note if e.children.is_empty() => false,
            Kind::LineBreak => {
                self.line_break();
                false
            }
            Kind::Note => {
                self.space();
                self.open_template(e.kind);
                true
            }
            Kind::Title => {
                self.line_break();
                self.open_template(e.kind);
                true
            }
            Kind::Verse | Kind::Chapter => {
                if e.milestone != Some(Milestone::End) {
                    self.label(e);
                }
                true
            }
            Kind::Paragraph | Kind::LineGroup | Kind::Entry => {
                if !e.children.is_empty() || e.milestone == Some(Milestone::Start) {
                    self.line_break();
                }
                true
            }
            Kind::Line | Kind::Word => true,
            _ => {
                self.open_template(e.kind);
                true
            }
        }
    }

    /// Writes what comes after a kept element's contents, and its span
    #[inline(never)]
    fn close(&mut self, e: &Element, path: &[usize], start: usize) {
        match e.kind {
            Kind::Note => self.close_template(e.kind),
            Kind::Title => {
                self.close_template(e.kind);
                self.line_break();
            }
            Kind::Verse | Kind::Chapter => {}
            Kind::Paragraph | Kind::LineGroup | Kind::Entry => {
                if !e.children.is_empty() {
                    self.line_break();
                }
            }
            Kind::Line => {
                if e.milestone != Some(Milestone::Start) {
                    self.line_break();
                }
            }
            Kind::Word => {
                if self.opts.strongs {
                    let numbers = e.strongs();
                    if !numbers.is_empty() {
                        let list: Vec<String> = numbers.iter().map(ToString::to_string).collect();
                        self.space();
                        self.piece(&format!("[{}]", list.join(" ")), None);
                    }
                }
            }
            Kind::WordsOfJesus if e.unclosed => {
                // Its closing text is written where the words of Jesus end
                self.words = Some(OpenWords {
                    start,
                    entry: self.entry(),
                    path: NodePath(path.to_vec()),
                    id: None,
                });
                return;
            }
            _ => self.close_template(e.kind),
        }
        if e.milestone.is_none() {
            self.push_span(start, e.kind, path);
        }
    }

    fn push_span(&mut self, start: usize, kind: Kind, path: &[usize]) {
        self.spans.push(Span {
            range: start..self.text.len(),
            kind,
            entry: self.entry(),
            path: NodePath(path.to_vec()),
        });
    }

    /// Writes the opening text of a kind's template
    fn open_template(&mut self, kind: Kind) {
        let (before, _) = self.template(kind);
        let before = before.to_string();
        self.write_before(&before);
    }

    /// Writes the closing text of a kind's template
    fn close_template(&mut self, kind: Kind) {
        let (_, after) = self.template(kind);
        let after = after.to_string();
        self.write_after(&after);
    }

    fn words_milestone(&mut self, e: &Element, path: &[usize]) {
        match e.milestone {
            Some(Milestone::Start) if e.kind == Kind::WordsOfJesus && self.words.is_none() => {
                self.words = Some(OpenWords {
                    start: self.text.len(),
                    entry: self.entry(),
                    path: NodePath(path.to_vec()),
                    id: e.attr("sID").map(str::to_string),
                });
                let (before, _) = self.template(Kind::WordsOfJesus);
                let before = before.to_string();
                self.write_before(&before);
            }
            Some(Milestone::End) => {
                let Some(words) = &self.words else { return };
                if !words_match(words.id.as_deref(), e) {
                    return;
                }
                let (_, after) = self.template(Kind::WordsOfJesus);
                let after = after.to_string();
                self.write_after(&after);
                if let Some(words) = self.words.take() {
                    self.spans.push(Span {
                        range: words.start..self.text.len(),
                        kind: Kind::WordsOfJesus,
                        entry: words.entry,
                        path: words.path,
                    });
                }
            }
            _ => {}
        }
    }

    fn label(&mut self, e: &Element) {
        let template = if e.kind == Kind::Chapter {
            self.opts.chapter_label.as_deref()
        } else {
            self.opts.verse_label.as_deref()
        };
        let Some(template) = template else { return };
        let id = MarkerId::of(e);
        let number = marker_number(e);
        let book = id.book.or(self.book.as_deref()).unwrap_or("");
        let label = if e.kind == Kind::Chapter {
            let chapter = number.or(id.chapter).unwrap_or("");
            fill_label(template, book, chapter, "")
        } else {
            let chapter = id.chapter.or(self.chapter.as_deref()).unwrap_or("");
            let verse = number.or(id.verse).unwrap_or("");
            fill_label(template, book, chapter, verse)
        };
        if label.trim().is_empty() {
            return;
        }
        if e.kind == Kind::Chapter {
            self.line_break();
            self.piece(&label, None);
            self.line_break();
        } else {
            self.space();
            self.piece(&label, None);
            self.space();
        }
    }
}

/// Whether an end milestone ends the open words of Jesus
pub(crate) fn words_match(open_id: Option<&str>, end: &Element) -> bool {
    match (open_id, end.attr("eID")) {
        (Some(open), Some(end_id)) => open == end_id,
        (None, None) => true,
        _ => end.kind == Kind::WordsOfJesus,
    }
}

/// A range without the whitespace at its ends; only whitespace gives the empty range at its end
fn trim(text: &str, range: Range<usize>) -> Range<usize> {
    let Some(covered) = text.get(range.clone()) else {
        return range.end.min(text.len())..range.end.min(text.len());
    };
    let leading = covered.len() - covered.trim_start().len();
    let start = range.start + leading;
    if start >= range.end {
        return range.end..range.end;
    }
    let trailing = covered.len() - covered.trim_end().len();
    start..range.end - trailing
}

/// Flattens one entry (see [`Flattener`])
pub fn flatten(nodes: &[Node], opts: &TextOptions) -> Flat {
    let mut flattener = Flattener::new(opts.clone());
    flattener.push(nodes);
    flattener.finish()
}
