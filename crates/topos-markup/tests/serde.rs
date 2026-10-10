//! With the `serde` feature, the tree, paths, and verse spans serialize (docs/spec/api.md).
//!
//! `cargo test --features serde --test serde`

#![cfg(feature = "serde")]

use serde::{Serialize, de::DeserializeOwned};
use topos_bible_markup::{Element, Kind, Milestone, Node, NodePath, verses::VerseSpan};

fn both<T: Serialize + DeserializeOwned>() {}

#[test]
fn derives() {
    both::<Node>();
    both::<Element>();
    both::<Kind>();
    both::<Milestone>();
    both::<NodePath>();
    both::<VerseSpan>();
    both::<Vec<Node>>();
}

#[test]
fn json_shape() {
    let nodes = topos_bible_markup::parse(
        topos_bible_markup::Markup::Osis,
        r#"<q who="Jesus" sID="q1"/>Hi <w lemma="strong:G1">x</w><verse osisID="John.1.1">y</verse>"#,
    );
    assert_eq!(
        serde_json::to_string(&nodes).expect("serializes"),
        r#"[{"Element":{"kind":"WordsOfJesus","name":"q","attrs":[["who","Jesus"],["sID","q1"]],"children":[],"milestone":"Start","unclosed":false}},{"Text":"Hi "},{"Element":{"kind":"Word","name":"w","attrs":[["lemma","strong:G1"]],"children":[{"Text":"x"}],"milestone":null,"unclosed":false}},{"Element":{"kind":"Verse","name":"verse","attrs":[["osisID","John.1.1"]],"children":[{"Text":"y"}],"milestone":null,"unclosed":false}}]"#
    );
    let spans = topos_bible_markup::verses::verses(&nodes);
    let json = serde_json::to_string(&spans).expect("serializes");
    assert_eq!(
        json,
        r#"[{"chapter":0,"verse":1,"last":1,"last_chapter":0,"part":0,"marker":[3],"start":[3],"end":[4]}]"#
    );
    let back: Vec<VerseSpan> = serde_json::from_str(&json).expect("deserializes");
    assert_eq!(back, spans);
    let tree: Vec<Node> = serde_json::from_str(&serde_json::to_string(&nodes).expect("serializes"))
        .expect("deserializes");
    assert_eq!(tree, nodes);
}
