/*!
Verses in a whole book's markup (USX, or OSIS with verse milestones), and the tree between two
places in it.

A book is one tree, and a verse is not an element in it: USX marks where each verse starts
(`<verse sid>`) and ends (`<verse eid>`), and a verse can start in one paragraph and end in the
next. [`verses`] finds where each verse is, and [`slice`] copies the tree between two places,
keeping the elements around the cut (a paragraph cut in two is two paragraphs), so a passage
renders with its paragraphs and poetry intact.

```
use topos_bible_markup::{HtmlRenderer, Markup, parse, verses::{slice, verses}};

let usx = r#"<usx><book code="JHN"/><chapter number="3" sid="JHN 3"/>
<para style="s1">Jesus and Nicodemus</para>
<para style="p"><verse number="16" sid="JHN 3:16"/>For God so loved the world.<verse eid="JHN 3:16"/><verse number="17" sid="JHN 3:17"/>For God did not send His Son.<verse eid="JHN 3:17"/></para>
<chapter eid="JHN 3"/></usx>"#;
let nodes = parse(Markup::Usx, usx);
let spans = verses(&nodes);
assert_eq!((spans[1].chapter, spans[1].verse), (3, 17));

// A verse starts where the one before it ends, so the heading before 3:16 belongs to it
let html = HtmlRenderer::new().render(&slice(&nodes, &spans[0].start, Some(&spans[0].end)));
assert!(html.contains("Jesus and Nicodemus") && html.contains("so loved") && !html.contains("did not send"));
```
*/

use std::ops::Range;

use topos_bible::segments::Passage;

use crate::{Element, Kind, Milestone, Node, NodePath};

/// Where one verse is in a book's tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerseSpan {
    pub chapter: u8,
    pub verse: u8,
    /// The last verse, for a bridge like `<verse number="4-5">` (else the same as `verse`)
    pub last: u8,
    /// The verse's marker
    pub marker: NodePath,
    /**
    The first place in the verse: right after the verse before it ends (or after its chapter's
    marker), so a heading between two verses belongs to the second
    - Without end markers, the verse's own marker
    - A place is a path in document order; it can be one past the last child of an element
    */
    pub start: NodePath,
    /// The first place after the verse: its end marker, the next verse's or chapter's marker, or
    /// one past the last top-level node
    pub end: NodePath,
}

impl VerseSpan {
    /// Whether the verse is in the inclusive range of (chapter, verse) pairs
    fn within(&self, start: (u8, u8), end: (u8, u8)) -> bool {
        (self.chapter, self.last) >= start && (self.chapter, self.verse) <= end
    }
}

/// A marker found while walking the tree
struct Marker {
    path: NodePath,
    kind: Kind,
    milestone: Option<Milestone>,
    /// The chapter or verse number (and the last verse of a bridge)
    number: Option<(u8, u8)>,
}

/**
Every verse in a book's tree, in order
- Chapters come from chapter markers (`<chapter number>`, OSIS `osisID="John.3"`); verse numbers
  from `number`, `n`, or the last part of `osisID`/`sID` (`John.3.16`, `JHN 3:16`)
- Verses that are elements with their text inside (OSIS without milestones) work too: the
  verse is that element
*/
pub fn verses(nodes: &[Node]) -> Vec<VerseSpan> {
    let mut markers = Vec::new();
    collect(nodes, &mut NodePath::default(), &mut markers);
    let after_all = NodePath(vec![nodes.len()]);

    let mut out: Vec<VerseSpan> = Vec::new();
    let mut chapter = 0u8;
    // Where the next verse starts: after the last end marker or chapter marker
    let mut next_start: Option<NodePath> = None;
    // The verse waiting for its end
    let mut open: Option<VerseSpan> = None;
    for m in &markers {
        match (m.kind, m.milestone) {
            (Kind::Chapter, Some(Milestone::End)) => {}
            (Kind::Chapter, _) => {
                if let Some(mut v) = open.take() {
                    v.end = m.path.clone();
                    out.push(v);
                }
                chapter = m.number.map_or(chapter.saturating_add(1), |(n, _)| n);
                next_start = Some(after(&m.path));
            }
            (Kind::Verse, Some(Milestone::End)) => {
                if let Some(mut v) = open.take() {
                    v.end = m.path.clone();
                    next_start = Some(after(&m.path));
                    out.push(v);
                }
            }
            (Kind::Verse, _) => {
                if let Some(mut v) = open.take() {
                    v.end = m.path.clone();
                    out.push(v);
                }
                let Some((verse, last)) = m.number else {
                    continue;
                };
                let start = next_start.take().unwrap_or_else(|| m.path.clone());
                let mut span = VerseSpan {
                    chapter,
                    verse,
                    last,
                    marker: m.path.clone(),
                    start,
                    end: after_all.clone(),
                };
                if m.milestone.is_none() {
                    // The verse is the element itself
                    span.end = after(&m.path);
                    next_start = Some(span.end.clone());
                    out.push(span);
                } else {
                    open = Some(span);
                }
            }
            _ => {}
        }
    }
    if let Some(v) = open {
        out.push(v);
    }
    out
}

/// The verse and chapter markers in document order (not looking inside verse elements)
fn collect(nodes: &[Node], path: &mut NodePath, out: &mut Vec<Marker>) {
    for (i, node) in nodes.iter().enumerate() {
        let Node::Element(e) = node else { continue };
        path.0.push(i);
        if matches!(e.kind, Kind::Verse | Kind::Chapter) {
            out.push(Marker {
                path: path.clone(),
                kind: e.kind,
                milestone: if e.children.is_empty() {
                    // `<verse number="1"/>` without sid is a start too
                    Some(e.milestone.unwrap_or(Milestone::Start))
                } else {
                    e.milestone
                },
                number: number(e),
            });
            if e.children.is_empty() || e.kind == Kind::Verse {
                path.0.pop();
                continue;
            }
        }
        collect(&e.children, path, out);
        path.0.pop();
    }
}

/// The place right after a node (its next sibling's place)
fn after(path: &NodePath) -> NodePath {
    let mut next = path.clone();
    if let Some(last) = next.0.last_mut() {
        *last += 1;
    }
    next
}

/// A chapter or verse number, with the last verse of a bridge (`4-5`)
fn number(e: &Element) -> Option<(u8, u8)> {
    let text = e
        .attr("number")
        .or_else(|| e.attr("n"))
        .map(str::to_string)
        .or_else(|| {
            // `John.3.16`, `JHN 3:16`, `Gen.1`
            let id = e.attr("osisID").or_else(|| e.attr("sID"))?;
            let id = id.split_whitespace().next_back()?;
            Some(id.rsplit(['.', ':']).next()?.to_string())
        })?;
    let mut parts = text.splitn(2, ['-', '–', ',']);
    let first = leading_number(parts.next()?)?;
    let last = parts.next().and_then(leading_number).unwrap_or(first);
    Some((first, last.max(first)))
}

fn leading_number(s: &str) -> Option<u8> {
    let digits: String = s.trim().chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/**
A copy of the tree from `start` up to (not including) `end`; `None` is the end of the tree
- An element the cut goes through is copied with only its children inside the range, and
  dropped if none are left
- Places compare in document order, so `start` and `end` can be anywhere (see [`VerseSpan`])
*/
pub fn slice(nodes: &[Node], start: &NodePath, end: Option<&NodePath>) -> Vec<Node> {
    slice_in(nodes, &mut NodePath::default(), start, end)
}

fn slice_in(
    nodes: &[Node],
    prefix: &mut NodePath,
    start: &NodePath,
    end: Option<&NodePath>,
) -> Vec<Node> {
    let mut out = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        prefix.0.push(i);
        let here = &*prefix;
        // At or after the end (an ancestor of the end is before it, and is cut instead)
        if end.is_some_and(|end| here >= end) {
            prefix.0.pop();
            break;
        }
        let cuts_start = is_strict_prefix(here, start);
        let cuts_end = end.is_some_and(|end| is_strict_prefix(here, end));
        if here < start && !cuts_start {
            prefix.0.pop();
            continue;
        }
        match node {
            Node::Element(e) if cuts_start || cuts_end => {
                let children = slice_in(&e.children, prefix, start, end);
                if !children.is_empty() {
                    let mut copy = Element {
                        children,
                        ..Element::new(e.name.clone(), e.attrs.clone())
                    };
                    copy.kind = e.kind;
                    copy.milestone = e.milestone;
                    copy.unclosed = e.unclosed;
                    out.push(Node::Element(copy));
                }
            }
            _ => out.push(node.clone()),
        }
        prefix.0.pop();
    }
    out
}

fn is_strict_prefix(a: &NodePath, b: &NodePath) -> bool {
    a.0.len() < b.0.len() && b.0.starts_with(&a.0)
}

/**
The places each part of a passage covers, as `start..end` for [`slice`], in the passage's order
- Verses the book doesn't have are left out; a part with none of its verses is left out
- A whole chapter is every verse the book has in it, whatever the versification says
*/
pub fn passage_ranges(spans: &[VerseSpan], passage: &Passage) -> Vec<Range<usize>> {
    passage
        .ranges(None)
        .into_iter()
        .filter_map(|r| {
            let (start, end) = (
                (r.start.chapter, r.start.verse),
                (r.end.chapter, r.end.verse),
            );
            let first = spans.iter().position(|s| s.within(start, end))?;
            let last = spans.iter().rposition(|s| s.within(start, end))?;
            Some(first..last + 1)
        })
        .collect()
}

/**
The number of verses in each chapter, from chapter 1 to the last chapter with verses, as the
book's own markers number them (a bridge like `4-5` counts to 5; a chapter without verses is 0)
- This is the book's versification, for checking references against this translation, like
  `Translations::set_chapter_verses` in topos-bible
*/
pub fn chapter_verses(spans: &[VerseSpan]) -> Vec<u8> {
    let mut counts: Vec<u8> = vec![];
    for span in spans.iter().filter(|s| s.chapter > 0) {
        let idx = span.chapter as usize - 1;
        if counts.len() <= idx {
            counts.resize(idx + 1, 0);
        }
        counts[idx] = counts[idx].max(span.last.max(span.verse));
    }
    counts
}
