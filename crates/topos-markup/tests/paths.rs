//! `NodePath` (docs/spec/node-paths.md).

use topos_bible_markup::{Markup, NodePath, parse};

#[test]
fn display_resolve_ancestors() {
    let nodes = parse(Markup::Osis, r#"a<hi>b<w>c</w></hi>"#);
    assert_eq!(NodePath(vec![1, 1, 0]).to_string(), "/1/1/0");
    assert_eq!(NodePath::default().to_string(), "");
    assert_eq!(
        NodePath(vec![1, 1, 0])
            .resolve(&nodes)
            .and_then(|n| n.as_text()),
        Some("c")
    );
    assert_eq!(
        NodePath(vec![0]).resolve(&nodes).and_then(|n| n.as_text()),
        Some("a")
    );
    assert!(
        NodePath(vec![0, 0]).resolve(&nodes).is_none(),
        "into a text node"
    );
    assert!(NodePath(vec![2]).resolve(&nodes).is_none());
    assert!(NodePath(vec![1, 5]).resolve(&nodes).is_none());
    assert!(NodePath::default().resolve(&nodes).is_none());
    let ancestors: Vec<String> = NodePath(vec![2, 0, 1])
        .ancestors()
        .map(|p| p.to_string())
        .collect();
    assert_eq!(ancestors, ["/2", "/2/0"]);
    assert_eq!(NodePath(vec![3]).ancestors().count(), 0);
    assert_eq!(NodePath::default().ancestors().count(), 0);
}

#[test]
fn document_order() {
    let mut paths = [
        NodePath(vec![2]),
        NodePath(vec![1, 2]),
        NodePath(vec![1]),
        NodePath(vec![1, 0]),
        NodePath(vec![0, 9, 9]),
    ];
    paths.sort();
    let shown: Vec<String> = paths.iter().map(ToString::to_string).collect();
    assert_eq!(shown, ["/0/9/9", "/1", "/1/0", "/1/2", "/2"]);
}
