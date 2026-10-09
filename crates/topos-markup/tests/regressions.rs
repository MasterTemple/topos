//! Inputs on which version 0.5.0 panicked; the required behaviour is in docs/spec/entities.md
//! and docs/spec/gaps.md. These are not in the golden files.

use topos_bible_markup::{Markup, Node, TextOptions, decode_entities, parse, plain_text};

#[test]
fn empty_reference_name_is_kept() {
    assert_eq!(decode_entities("&;"), "&;");
    assert_eq!(decode_entities("a &; b &amp;"), "a &; b &");
}

#[test]
fn non_ascii_reference_name_is_kept() {
    assert_eq!(decode_entities("&é;"), "&é;");
    assert_eq!(decode_entities("&éa; &שׁ; &😀;"), "&éa; &שׁ; &😀;");
}

#[test]
fn parsing_such_references_keeps_them() {
    for markup in [
        Markup::Osis,
        Markup::Thml,
        Markup::Tei,
        Markup::Usx,
        Markup::Gbf,
    ] {
        assert_eq!(
            parse(markup, "a &; b &é; c"),
            vec![Node::Text("a &; b &é; c".into())],
            "{markup:?}"
        );
        let nodes = parse(markup, r#"<x a="&é;">&;</x>"#);
        assert!(
            plain_text(&nodes, &TextOptions::default()).contains("&;"),
            "{markup:?}"
        );
    }
    let nodes = parse(Markup::Osis, r#"<w lemma="&;">&é;</w>"#);
    let w = nodes[0].as_element().expect("a word");
    assert_eq!(w.attr("lemma"), Some("&;"));
    assert_eq!(w.text(), "&é;");
}
