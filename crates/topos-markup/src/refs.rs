//! References the markup tags itself: OSIS `<reference osisRef>`, ThML `<scripRef passage>`,
//! TEI `<ref target>`, and USX `<ref loc>` / `<char style="xt">`.

use topos_bible::{data::BibleData, segments::Passage};

use crate::{Element, Kind, Node, path::NodePath};

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
`Bible:`), then as a written reference (`John 3:16`, USX `JHN 3:16`); failing both, the element's
text is
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
    collect(nodes, &NodePath::default(), data, &mut out);
    out
}

fn collect(nodes: &[Node], path: &NodePath, data: &BibleData, out: &mut Vec<TaggedReference>) {
    for (i, node) in nodes.iter().enumerate() {
        if let Node::Element(e) = node {
            let path = path.child(i);
            if e.kind == Kind::Reference {
                out.push(TaggedReference {
                    path: path.clone(),
                    target: e.reference_target().map(str::to_string),
                    passages: passages(e, data),
                });
            }
            collect(&e.children, &path, data, out);
        }
    }
}

/// The passages an element points to
pub fn passages(e: &Element, data: &BibleData) -> Vec<Passage> {
    if let Some(target) = e.reference_target() {
        let without_work = target
            .split_once(':')
            .filter(|(work, _)| !work.is_empty() && !work.contains(char::is_whitespace))
            .map(|(_, rest)| rest);
        for candidate in [Some(target), without_work].into_iter().flatten() {
            if let Some(passages) = osis(candidate, data) {
                return passages;
            }
        }
        if let Some(passage) = data.parse(target) {
            return vec![passage];
        }
    }
    data.parse(&e.text()).into_iter().collect()
}

/// Space-separated OSIS references, merged into one passage per run of the same book
fn osis(target: &str, data: &BibleData) -> Option<Vec<Passage>> {
    let mut out: Vec<Passage> = Vec::new();
    for reference in target.split_whitespace() {
        let passage = data.books().parse_osis(reference).ok()?;
        match out.last_mut() {
            Some(last) if last.book == passage.book => last.segments.extend(passage.segments),
            _ => out.push(passage),
        }
    }
    (!out.is_empty()).then_some(out)
}
