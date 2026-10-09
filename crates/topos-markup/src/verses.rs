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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// A verse or chapter marker found in the tree
struct Marker<'a> {
    path: Vec<usize>,
    element: &'a Element,
}

fn markers<'a>(nodes: &'a [Node], path: &mut Vec<usize>, out: &mut Vec<Marker<'a>>) {
    for (i, node) in nodes.iter().enumerate() {
        let Node::Element(e) = node else { continue };
        path.push(i);
        if e.kind == Kind::Verse || e.kind == Kind::Chapter {
            out.push(Marker {
                path: path.clone(),
                element: e,
            });
        }
        if e.kind != Kind::Verse {
            markers(&e.children, path, out);
        }
        path.pop();
    }
}

/// The value of a number's leading ASCII digits, after trimming whitespace
fn leading_number(s: &str) -> Option<u32> {
    let s = s.trim();
    let digits = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let digits = s.get(..digits)?;
    if digits.is_empty() {
        return None;
    }
    Some(digits.bytes().fold(0u32, |n, d| {
        n.saturating_mul(10).saturating_add(u32::from(d - b'0'))
    }))
}

/// A marker's number as (first, last)
fn number(e: &Element) -> Option<(u8, u8)> {
    let written = match e.attr("number").or_else(|| e.attr("n")) {
        Some(n) => n,
        None => {
            let id = e.attr("osisID").or_else(|| e.attr("sID"))?;
            let word = id.split_whitespace().last().unwrap_or("");
            word.rsplit(['.', ':']).next().unwrap_or("")
        }
    };
    let (first, rest) = match written.find(['-', '–', ',']) {
        Some(i) => {
            let sep = written.get(i..)?.chars().next()?.len_utf8();
            (written.get(..i)?, written.get(i + sep..))
        }
        None => (written, None),
    };
    let first = u8::try_from(leading_number(first)?).ok()?;
    let last = rest
        .and_then(leading_number)
        .and_then(|n| u8::try_from(n).ok())
        .unwrap_or(first);
    Some((first, last.max(first)))
}

/// The place right after a node
fn after(path: &[usize]) -> NodePath {
    let mut place = path.to_vec();
    if let Some(last) = place.last_mut() {
        *last = last.saturating_add(1);
    }
    NodePath(place)
}

/**
Every verse in a book's tree, in order
- Chapters come from chapter markers (`<chapter number>`, OSIS `osisID="John.3"`); verse numbers
  from `number`, `n`, or the last part of `osisID`/`sID` (`John.3.16`, `JHN 3:16`)
- Verses that are elements with their text inside (OSIS without milestones) work too: the
  verse is that element
*/
pub fn verses(nodes: &[Node]) -> Vec<VerseSpan> {
    let mut found = Vec::new();
    markers(nodes, &mut Vec::new(), &mut found);
    let tree_end = NodePath(vec![nodes.len()]);
    let mut out = Vec::new();
    let mut chapter: u8 = 0;
    let mut next_start: Option<NodePath> = None;
    // The chapter marker before, when it's an element holding its verses (OSIS
    // `<chapter osisID>…</chapter>`), whose first verse starts inside it
    let mut holding: Option<Vec<usize>> = None;
    let mut open: Option<VerseSpan> = None;
    for marker in found {
        let e = marker.element;
        let milestone = if e.children.is_empty() {
            Some(e.milestone.unwrap_or(Milestone::Start))
        } else {
            e.milestone
        };
        let here = NodePath(marker.path.clone());
        if e.kind == Kind::Chapter {
            if milestone == Some(Milestone::End) {
                continue;
            }
            if let Some(mut verse) = open.take() {
                verse.end = here.clone();
                out.push(verse);
            }
            chapter = match number(e) {
                Some((n, _)) => n,
                None => chapter.saturating_add(1),
            };
            next_start = Some(after(&marker.path));
            holding = (!e.children.is_empty()).then(|| marker.path.clone());
            continue;
        }
        if milestone == Some(Milestone::End) {
            if let Some(mut verse) = open.take() {
                verse.end = here;
                out.push(verse);
                next_start = Some(after(&marker.path));
            }
            continue;
        }
        if let Some(mut verse) = open.take() {
            verse.end = here.clone();
            out.push(verse);
        }
        let Some((first, last)) = number(e) else {
            continue;
        };
        let start = match (holding.take(), next_start.take()) {
            // The chapter's first verse inside it starts at the chapter's start
            (Some(chapter), _) if marker.path.starts_with(&chapter) => {
                NodePath([chapter.as_slice(), &[0]].concat())
            }
            (_, start) => start.unwrap_or_else(|| here.clone()),
        };
        let verse = VerseSpan {
            chapter,
            verse: first,
            last,
            marker: here.clone(),
            start,
            end: tree_end.clone(),
        };
        if milestone.is_none() {
            out.push(VerseSpan {
                end: after(&marker.path),
                ..verse
            });
            next_start = Some(after(&marker.path));
        } else {
            open = Some(verse);
        }
    }
    if let Some(verse) = open {
        out.push(verse);
    }
    out
}

/// Whether `a` is a proper prefix of `b` (an ancestor of the place `b`)
fn is_ancestor(a: &[usize], b: &[usize]) -> bool {
    a.len() < b.len() && b.starts_with(a)
}

fn slice_nodes(
    nodes: &[Node],
    path: &mut Vec<usize>,
    start: &[usize],
    end: Option<&[usize]>,
) -> Vec<Node> {
    let mut out = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        path.push(i);
        let p = path.as_slice();
        if end.is_some_and(|end| p >= end) {
            path.pop();
            break;
        }
        let holds_start = is_ancestor(p, start);
        if p < start && !holds_start {
            path.pop();
            continue;
        }
        match node {
            Node::Element(e) if holds_start || end.is_some_and(|end| is_ancestor(p, end)) => {
                let children = slice_nodes(&e.children, path, start, end);
                if !children.is_empty() {
                    out.push(Node::Element(Element {
                        kind: e.kind,
                        name: e.name.clone(),
                        attrs: e.attrs.clone(),
                        children,
                        milestone: e.milestone,
                        unclosed: e.unclosed,
                    }));
                }
            }
            _ => out.push(node.clone()),
        }
        path.pop();
    }
    out
}

/**
A copy of the tree from `start` up to (not including) `end`; `None` is the end of the tree
- An element the cut goes through is copied with only its children inside the range, and
  dropped if none are left
- Places compare in document order, so `start` and `end` can be anywhere (see [`VerseSpan`])
*/
pub fn slice(nodes: &[Node], start: &NodePath, end: Option<&NodePath>) -> Vec<Node> {
    slice_nodes(
        nodes,
        &mut Vec::new(),
        &start.0,
        end.map(|e| e.0.as_slice()),
    )
}

/**
The places each part of a passage covers, as `start..end` for [`slice`], in the passage's order
- Verses the book doesn't have are left out; a part with none of its verses is left out
- A whole chapter is every verse the book has in it, whatever the versification says
*/
pub fn passage_ranges(spans: &[VerseSpan], passage: &Passage) -> Vec<Range<usize>> {
    passage
        .ranges(None)
        .iter()
        .filter_map(|range| {
            let from = (range.start.chapter, range.start.verse);
            let to = (range.end.chapter, range.end.verse);
            let mut inside = spans
                .iter()
                .enumerate()
                .filter(|(_, s)| (s.chapter, s.last) >= from && (s.chapter, s.verse) <= to)
                .map(|(i, _)| i);
            let first = inside.next()?;
            let last = inside.next_back().unwrap_or(first);
            Some(first..last + 1)
        })
        .collect()
}

/// Raises a chapter's count, growing the list as needed
fn raise(counts: &mut Vec<u8>, chapter: u8, verse: u8) {
    let index = usize::from(chapter).saturating_sub(1);
    if counts.len() <= index {
        counts.resize(index + 1, 0);
    }
    if let Some(count) = counts.get_mut(index) {
        *count = (*count).max(verse);
    }
}

/**
The number of verses in each chapter, from chapter 1 to the last chapter with verses, as the
book's own markers number them (a bridge like `4-5` counts to 5; a chapter without verses is 0)
- This is the book's versification, for checking references against this translation, like
  `Translations::set_chapter_verses` in topos-bible
*/
pub fn chapter_verses(spans: &[VerseSpan]) -> Vec<u8> {
    let mut counts = Vec::new();
    for span in spans {
        if span.chapter >= 1 {
            raise(&mut counts, span.chapter, span.last.max(span.verse));
        }
    }
    counts
}

/// The `number` attribute in a tag's text, read without parsing the tag
fn number_attribute(tag: &str) -> Option<&str> {
    let mut from = 0;
    while let Some(found) = tag.get(from..)?.find("number") {
        let at = from + found;
        from = at + "number".len();
        let preceded = tag
            .get(..at)
            .and_then(|b| b.chars().next_back())
            .is_some_and(char::is_whitespace);
        if !preceded {
            continue;
        }
        let rest = tag.get(from..)?.trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let value = rest.get(1..)?;
        return Some(value.split(quote).next().unwrap_or(value));
    }
    None
}

/**
A USX book's verse counts, like [`chapter_verses`], from its `<chapter number>` and
`<verse number>` markers alone, without parsing the rest (fast enough to count a whole Bible)
*/
pub fn usx_chapter_verses(usx: &str) -> Vec<u8> {
    let mut counts = Vec::new();
    let mut chapter: u8 = 0;
    for piece in usx.split('<').skip(1) {
        let tag = piece.split('>').next().unwrap_or(piece);
        let name = tag
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or("");
        if name != "chapter" && name != "verse" {
            continue;
        }
        let Some(value) = number_attribute(tag) else {
            continue;
        };
        let last = value.rsplit(['-', '–']).next().unwrap_or(value);
        let Some(n) = leading_number(last) else {
            continue;
        };
        let n = u8::try_from(n).unwrap_or(u8::MAX);
        if name == "chapter" {
            chapter = n;
        } else if chapter >= 1 {
            raise(&mut counts, chapter, n);
        }
    }
    counts
}
