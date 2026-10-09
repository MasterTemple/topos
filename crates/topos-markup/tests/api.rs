//! Compile-time checks of the interface dependents rely on (docs/spec/api.md).

use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug, Display};
use std::hash::Hash;

use topos_bible_markup::{
    Action, Element, Filter, Flat, Flattener, HtmlOptions, HtmlRenderer, Kind, Markup, Milestone,
    Node, NodePath, Point, Span, Strongs, StrongsLang, TextOptions, refs::TaggedReference,
    verses::VerseSpan,
};

fn send_sync<T: Send + Sync>() {}
fn value<T: Debug + Clone + PartialEq + Eq>() {}
fn key<T: Hash + Eq + Ord + Copy>() {}

#[test]
fn traits() {
    send_sync::<Filter>();
    send_sync::<TextOptions>();
    send_sync::<HtmlOptions>();
    send_sync::<HtmlRenderer>();
    send_sync::<Flattener>();
    send_sync::<Flat>();
    send_sync::<Node>();
    send_sync::<VerseSpan>();
    value::<Node>();
    value::<Element>();
    value::<Flat>();
    value::<Span>();
    value::<Point>();
    value::<NodePath>();
    value::<VerseSpan>();
    value::<TaggedReference>();
    value::<Strongs>();
    key::<Kind>();
    key::<StrongsLang>();
    fn hash<T: Hash + Eq + Copy + Debug>() {}
    hash::<Markup>();
    hash::<Milestone>();
    hash::<Action>();
    fn ordered<T: Ord + Hash + Default>() {}
    ordered::<NodePath>();
    fn point<T: Ord + Hash>() {}
    point::<Point>();
    fn shown<T: Display>() {}
    shown::<Strongs>();
    shown::<NodePath>();
    fn defaults<T: Default + Clone + Debug>() {}
    defaults::<TextOptions>();
    defaults::<HtmlOptions>();
    defaults::<HtmlRenderer>();
    defaults::<Filter>();
    defaults::<Flat>();
    let _: Option<Flattener> = None::<Flattener>.clone();
}

#[test]
fn shapes() {
    // Public fields, by struct literal and update syntax, as dependents write them
    let opts = TextOptions {
        filter: Filter::all(),
        strongs: false,
        line_breaks: true,
        verse_label: None,
        chapter_label: None,
        templates: BTreeMap::new(),
    };
    let _ = TextOptions {
        line_breaks: false,
        ..opts
    };
    let html = HtmlOptions {
        class_prefix: "x-".into(),
        filter: Filter::text(),
    };
    let renderer = HtmlRenderer::with_options(html);
    let _: &HtmlOptions = &renderer.opts;
    let span = VerseSpan {
        chapter: 1,
        verse: 2,
        last: 3,
        marker: NodePath(vec![0]),
        start: NodePath::default(),
        end: NodePath(vec![1]),
    };
    let mut by_marker: HashMap<&NodePath, &VerseSpan> = HashMap::new();
    by_marker.insert(&span.marker, &span);
    let point = Point {
        entry: 0,
        path: NodePath(vec![0]),
        offset: 0,
    };
    let _ = Span {
        range: 0..0,
        kind: Kind::Verse,
        entry: point.entry,
        path: point.path.clone(),
    };
    let s = Strongs {
        lang: StrongsLang::Greek,
        number: 25,
        suffix: None,
    };
    assert_eq!(s.to_string(), "G25");
    let _ = Element {
        kind: Kind::Other,
        name: String::new(),
        attrs: vec![],
        children: vec![],
        milestone: Some(Milestone::End),
        unclosed: false,
    };
}
