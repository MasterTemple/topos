//! Plain text with a map back to the tree: what plain-text rendering, search, and editor
//! highlighting are built on.

use std::ops::Range;

use crate::{Element, Kind, Milestone, Node, filter::Action, path::NodePath, text::TextOptions};

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

/// A run of [`Flat::text`] copied from one text node
#[derive(Debug, Clone, PartialEq, Eq)]
struct Piece {
    range: Range<usize>,
    entry: usize,
    path: NodePath,
    node_start: usize,
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
    pieces: Vec<Piece>,
}

impl Flat {
    /// Where a byte of [`Flat::text`] came from, or [`None`] for added text and whitespace
    /// between runs
    pub fn source(&self, byte: usize) -> Option<Point> {
        let i = self.pieces.partition_point(|p| p.range.end <= byte);
        let piece = self.pieces.get(i).filter(|p| p.range.start <= byte)?;
        Some(piece.point(byte))
    }

    /**
    Where a range of [`Flat::text`] came from: the start of its first source text and the end of
    its last, or [`None`] if none of it is from the tree
    - The end is exclusive, like the range
    */
    pub fn source_range(&self, range: Range<usize>) -> Option<(Point, Point)> {
        let first = self.pieces.partition_point(|p| p.range.end <= range.start);
        let last = self.pieces.partition_point(|p| p.range.start < range.end);
        if first >= last {
            return None;
        }
        let (a, b) = (&self.pieces[first], &self.pieces[last - 1]);
        Some((
            a.point(range.start.max(a.range.start)),
            b.point(range.end.min(b.range.end)),
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
        self.spans
            .iter()
            .filter(|s| s.kind == kind && s.range.start <= byte)
            .max_by_key(|s| s.range.start)
    }
}

impl Piece {
    fn point(&self, byte: usize) -> Point {
        Point {
            entry: self.entry,
            path: self.path.clone(),
            offset: self.node_start + (byte - self.range.start),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pending {
    None,
    Space,
    Breaks(u8),
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
    flat: Flat,
    pending: Pending,
    entries: usize,
    /// Words of Jesus that are still open
    woj: Option<OpenWoj>,
    /// The book and chapter of the last chapter marker, for labels
    book: Option<String>,
    chapter: Option<String>,
}

/// Words of Jesus that started and haven't ended
#[derive(Debug, Clone)]
struct OpenWoj {
    start: usize,
    entry: usize,
    path: NodePath,
    /// The `sID` of a start milestone, or [`None`] for an unclosed `<q>`
    id: Option<String>,
}

/**
Whether an end milestone or orphan end tag (a [`Kind::WordsOfJesus`] or [`Kind::Quote`]) ends words
of Jesus that started with the `sID` `id` ([`None`]: an unclosed `<q>`)
- OSIS end milestones (`<q eID="q1"/>`) don't repeat `who`, so they match by id
*/
pub(crate) fn ends_woj(id: Option<&str>, end: &Element) -> bool {
    match (id, end.attr("eID")) {
        (Some(id), Some(eid)) => id == eid,
        // An orphan `</q>` ends an unclosed `<q who="Jesus">` from an earlier entry
        (None, None) => true,
        _ => end.kind == Kind::WordsOfJesus,
    }
}

impl Flattener {
    pub fn new(opts: TextOptions) -> Self {
        Self {
            opts,
            flat: Flat::default(),
            pending: Pending::None,
            entries: 0,
            woj: None,
            book: None,
            chapter: None,
        }
    }

    /// Adds an entry (a verse, say), separated from the previous one by a space
    pub fn push(&mut self, nodes: &[Node]) -> &mut Self {
        let entry = self.entries;
        self.entries += 1;
        if entry > 0 {
            self.space();
            if let Some(open) = self.woj.take() {
                // Words of Jesus continue from the previous entry: close that part there
                let id = open.id.clone();
                self.close_woj(open);
                self.woj = Some(OpenWoj {
                    start: self.flat.text.len(),
                    entry,
                    path: NodePath::default(),
                    id,
                });
            }
        }
        self.nodes(nodes, entry, &NodePath::default());
        self
    }

    pub fn finish(mut self) -> Flat {
        if let Some(open) = self.woj.take() {
            self.close_woj(open);
        }
        self.entries_up_to(self.flat.text.len());
        let text = &self.flat.text;
        for span in &mut self.flat.spans {
            let slice = &text[span.range.clone()];
            let start = span.range.start + (slice.len() - slice.trim_start().len());
            let end = (span.range.end - (slice.len() - slice.trim_end().len())).max(start);
            span.range = start..end;
        }
        self.flat
    }

    fn close_woj(&mut self, open: OpenWoj) {
        self.flat.spans.push(Span {
            range: open.start..self.flat.text.len(),
            kind: Kind::WordsOfJesus,
            entry: open.entry,
            path: open.path,
        });
    }

    fn space(&mut self) {
        if self.pending == Pending::None {
            self.pending = Pending::Space;
        }
    }

    fn brk(&mut self) {
        if !self.opts.line_breaks {
            return self.space();
        }
        self.pending = match self.pending {
            Pending::Breaks(n) => Pending::Breaks(n.saturating_add(1)),
            _ => Pending::Breaks(1),
        };
    }

    fn visible(&mut self, s: &str, source: Option<(usize, &NodePath, usize)>) {
        if !self.flat.text.is_empty() {
            match self.pending {
                Pending::None => {}
                Pending::Space => self.flat.text.push(' '),
                Pending::Breaks(n) if n <= 2 => self.flat.text.push('\n'),
                Pending::Breaks(_) => self.flat.text.push_str("\n\n"),
            }
        }
        self.pending = Pending::None;
        let start = self.flat.text.len();
        self.entries_up_to(start);
        self.flat.text.push_str(s);
        if let Some(entry) = self.flat.entries.last_mut() {
            entry.end = self.flat.text.len();
        }
        if let Some((entry, path, node_start)) = source {
            self.flat.pieces.push(Piece {
                range: start..self.flat.text.len(),
                entry,
                path: path.clone(),
                node_start,
            });
        }
    }

    /// Starts the ranges of the entries pushed since the last text, at `at`
    fn entries_up_to(&mut self, at: usize) {
        while self.flat.entries.len() < self.entries {
            self.flat.entries.push(at..at);
        }
    }

    /// A verse's or chapter's label from its template, or [`None`] without one
    fn label(&self, e: &Element) -> Option<String> {
        let template = match e.kind {
            Kind::Verse => self.opts.verse_label.as_deref()?,
            Kind::Chapter => self.opts.chapter_label.as_deref()?,
            _ => return None,
        };
        if e.milestone == Some(Milestone::End) {
            return None;
        }
        let (book, chapter, verse) = id_parts(e);
        let number = e.attr("number").or_else(|| e.attr("n"));
        let (chapter, verse) = match e.kind {
            Kind::Chapter => (number.or(chapter), None),
            _ => (chapter.or(self.chapter.as_deref()), number.or(verse)),
        };
        let label = template
            .replace("{book}", book.or(self.book.as_deref()).unwrap_or_default())
            .replace("{chapter}", chapter.unwrap_or_default())
            .replace("{verse}", verse.unwrap_or_default());
        (!label.trim().is_empty()).then_some(label)
    }

    /// Writes a verse's label (as its own word) or a chapter's (on its own line)
    fn write_label(&mut self, e: &Element) {
        let Some(label) = self.label(e) else { return };
        if e.kind == Kind::Chapter {
            self.brk();
            self.visible(&label, None);
            self.brk();
        } else {
            self.space();
            self.visible(&label, None);
            self.space();
        }
    }

    fn text(&mut self, text: &str, entry: usize, path: &NodePath) {
        let mut run_start = None;
        for (i, c) in text.char_indices().chain([(text.len(), ' ')]) {
            match (c.is_whitespace(), run_start) {
                (true, Some(start)) => {
                    self.visible(&text[start..i], Some((entry, path, start)));
                    run_start = None;
                    if i < text.len() {
                        self.space();
                    }
                }
                (true, None) => {
                    if i < text.len() {
                        self.space();
                    }
                }
                (false, None) => run_start = Some(i),
                (false, Some(_)) => {}
            }
        }
    }

    fn nodes(&mut self, nodes: &[Node], entry: usize, path: &NodePath) {
        for (i, node) in nodes.iter().enumerate() {
            let path = path.child(i);
            match node {
                Node::Text(t) => self.text(t, entry, &path),
                Node::Element(e) => self.element(e, entry, &path),
            }
        }
    }

    fn element(&mut self, e: &Element, entry: usize, path: &NodePath) {
        if e.kind == Kind::Chapter && e.milestone != Some(Milestone::End) {
            let (book, chapter, _) = id_parts(e);
            if let Some(book) = book {
                self.book = Some(book.to_string());
            }
            let number = e.attr("number").or_else(|| e.attr("n")).or(chapter);
            self.chapter = number.map(str::to_string);
        }
        match self.opts.filter.action(e) {
            Action::Drop => return,
            Action::Unwrap => return self.nodes(&e.children, entry, path),
            Action::Keep => {}
        }
        let start = self.flat.text.len();
        match e.kind {
            Kind::WordsOfJesus | Kind::Quote if e.milestone.is_some() => {
                match e.milestone {
                    Some(Milestone::Start) if e.kind == Kind::WordsOfJesus => {
                        self.woj.get_or_insert(OpenWoj {
                            start,
                            entry,
                            path: path.clone(),
                            id: e.attr("sID").map(str::to_string),
                        });
                    }
                    Some(Milestone::End) => match self.woj.take() {
                        Some(open) if ends_woj(open.id.as_deref(), e) => self.close_woj(open),
                        open => self.woj = open,
                    },
                    _ => {}
                }
                return;
            }
            Kind::Verse | Kind::Chapter | Kind::Milestone if e.children.is_empty() => {
                self.write_label(e);
                self.flat.spans.push(Span {
                    range: start..self.flat.text.len(),
                    kind: e.kind,
                    entry,
                    path: path.clone(),
                });
                return;
            }
            Kind::Note => {
                if e.children.is_empty() {
                    return;
                }
                self.space();
                self.visible("[", None);
                self.nodes(&e.children, entry, path);
                self.visible("]", None);
            }
            Kind::Title => {
                self.brk();
                self.nodes(&e.children, entry, path);
                self.brk();
            }
            Kind::LineBreak => self.brk(),
            // Verses and chapters with their text inside
            Kind::Verse | Kind::Chapter => {
                self.write_label(e);
                self.nodes(&e.children, entry, path);
            }
            Kind::Paragraph | Kind::LineGroup | Kind::Entry => {
                if e.milestone == Some(Milestone::Start) || !e.children.is_empty() {
                    self.brk();
                }
                self.nodes(&e.children, entry, path);
                if !e.children.is_empty() {
                    self.brk();
                }
            }
            Kind::Line => {
                self.nodes(&e.children, entry, path);
                if e.milestone != Some(Milestone::Start) {
                    self.brk();
                }
            }
            Kind::Word => {
                self.nodes(&e.children, entry, path);
                let strongs = e.strongs();
                if self.opts.strongs && !strongs.is_empty() {
                    let list: Vec<String> = strongs.iter().map(ToString::to_string).collect();
                    self.space();
                    self.visible(&format!("[{}]", list.join(" ")), None);
                }
            }
            _ => self.nodes(&e.children, entry, path),
        }
        if e.kind == Kind::WordsOfJesus && e.unclosed {
            // Continues into the next entry
            self.woj = Some(OpenWoj {
                start,
                entry,
                path: path.clone(),
                id: None,
            });
            return;
        }
        if e.milestone.is_none() && e.kind != Kind::LineBreak {
            self.flat.spans.push(Span {
                range: start..self.flat.text.len(),
                kind: e.kind,
                entry,
                path: path.clone(),
            });
        }
    }
}

/// The book, chapter, and verse in a marker's id: `JHN 3:16` (USX), `John.3.16` (OSIS), `JHN 3`
fn id_parts(e: &Element) -> (Option<&str>, Option<&str>, Option<&str>) {
    let Some(id) = ["sID", "osisID"].into_iter().find_map(|name| e.attr(name)) else {
        return (None, None, None);
    };
    let (book, rest) = match id.split_once(' ') {
        // USX: `JHN 3:16`
        Some((book, rest)) if !rest.contains('.') => (book, rest.split(':').collect::<Vec<_>>()),
        // OSIS: `John.3.16` (the first, if several)
        _ => {
            let first = id.split_whitespace().next().unwrap_or_default();
            let mut parts = first.split('.');
            (parts.next().unwrap_or_default(), parts.collect())
        }
    };
    let part = |i: usize| rest.get(i).copied().filter(|p| !p.is_empty());
    (Some(book).filter(|b| !b.is_empty()), part(0), part(1))
}

/// Flattens one entry (see [`Flattener`])
pub fn flatten(nodes: &[Node], opts: &TextOptions) -> Flat {
    let mut flattener = Flattener::new(opts.clone());
    flattener.push(nodes);
    flattener.finish()
}
