//! References the markup tags itself: OSIS `<reference osisRef>`, ThML `<scripRef passage>`,
//! TEI `<ref target>`, and USX `<ref loc>` / `<char style="xt">`.

use topos_bible::{data::BibleData, segments::Passage};

use crate::{Element, Kind, Node, NodePath};

/// A [`Kind::Reference`] element and the passages it points to
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedReference {
    pub path: NodePath,
    /// The target as written (`osisRef`, `passage`, `loc`, ...)
    pub target: Option<String>,
    /// One passage per book; empty when neither the target nor the text is a reference
    pub passages: Vec<Passage>,
}

/**
Every reference element, with the passages it points to

The target is read as OSIS (`John.3.16-John.3.18 Rom.8.28`, with or without a work prefix like
`Bible:`), then as USFM (USX's `JHN 3:16-18`), then as a written reference (`John 3:16`); failing
all three, the element's text is
```
use topos_bible::data::BibleData;
use topos_bible_markup::{Markup, parse, refs::tagged_references};

let data = BibleData::default();
let nodes = parse(Markup::Osis, r#"See <reference osisRef="Bible:John.3.16-John.3.18 Rom.8.28">these</reference>."#);
let refs = tagged_references(&nodes, &data);
let osis: Vec<String> = refs[0].passages.iter().map(|p| p.to_osis(data.books()).unwrap()).collect();
assert_eq!(osis, ["John.3.16-John.3.18", "Rom.8.28"]);
```
*/
pub fn tagged_references(nodes: &[Node], data: &BibleData) -> Vec<TaggedReference> {
    let mut out = Vec::new();
    let mut path = Vec::new();
    collect(nodes, data, &mut path, &mut out);
    out
}

fn collect(
    nodes: &[Node],
    data: &BibleData,
    path: &mut Vec<usize>,
    out: &mut Vec<TaggedReference>,
) {
    for (i, node) in nodes.iter().enumerate() {
        let Node::Element(e) = node else { continue };
        path.push(i);
        if e.kind == Kind::Reference {
            out.push(TaggedReference {
                path: NodePath(path.clone()),
                target: e.reference_target().map(str::to_string),
                passages: passages(e, data),
            });
        }
        collect(&e.children, data, path, out);
        path.pop();
    }
}

/// The passages an element points to
pub fn passages(e: &Element, data: &BibleData) -> Vec<Passage> {
    if let Some(target) = e.reference_target() {
        if let Some(found) = osis(target, data) {
            return found;
        }
        if let Ok(passage) = data.books().parse_usfm(target) {
            return vec![passage];
        }
        if let Some(passage) = data.parse(target) {
            return vec![passage];
        }
    }
    data.parse(&e.text()).into_iter().collect()
}

/// The target as space-separated OSIS references, with or without a work prefix
fn osis(target: &str, data: &BibleData) -> Option<Vec<Passage>> {
    let mut candidates = vec![target];
    if let Some((work, rest)) = target.split_once(':')
        && !work.is_empty()
        && !work.contains(char::is_whitespace)
    {
        candidates.push(rest);
    }
    candidates
        .into_iter()
        .find_map(|candidate| osis_words(candidate, data))
}

fn osis_words(candidate: &str, data: &BibleData) -> Option<Vec<Passage>> {
    let mut out: Vec<Passage> = Vec::new();
    for word in candidate.split_whitespace() {
        let passage = data.books().parse_osis(word).ok()?;
        match out.last_mut() {
            Some(last) if last.book == passage.book => {
                last.segments.0.extend(passage.segments.0);
            }
            _ => out.push(passage),
        }
    }
    (!out.is_empty()).then_some(out)
}
