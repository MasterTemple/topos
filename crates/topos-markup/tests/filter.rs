//! `Filter` and `Action` (docs/spec/filters.md).

use topos_bible_markup::{
    Action, Element, Filter, HtmlOptions, HtmlRenderer, Kind, Markup, TextOptions, parse,
    plain_text,
};

fn element(kind: Kind, attrs: &[(&str, &str)]) -> Element {
    Element {
        kind,
        name: String::from("x"),
        attrs: attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        children: vec![],
        milestone: None,
        unclosed: false,
    }
}

#[test]
fn defaults() {
    let all = Filter::all();
    let text = Filter::text();
    for kind in [
        Kind::Word,
        Kind::Note,
        Kind::Title,
        Kind::Metadata,
        Kind::Other,
    ] {
        assert_eq!(all.action(&element(kind, &[])), Action::Keep);
    }
    assert_eq!(text.action(&element(Kind::Note, &[])), Action::Drop);
    assert_eq!(text.action(&element(Kind::Title, &[])), Action::Drop);
    assert_eq!(text.action(&element(Kind::Metadata, &[])), Action::Drop);
    assert_eq!(text.action(&element(Kind::Word, &[])), Action::Keep);
    assert_eq!(
        Filter::default().action(&element(Kind::Note, &[])),
        Action::Keep
    );
}

#[test]
fn precedence() {
    let study = element(Kind::Note, &[("type", "Study")]);
    let untyped = element(Kind::Note, &[]);
    let filter = Filter::text().note_type("study", Action::Unwrap);
    assert_eq!(filter.action(&study), Action::Unwrap);
    assert_eq!(filter.action(&untyped), Action::Drop);
    // Later note-type settings win
    let filter = filter.note_type("STUDY", Action::Keep);
    assert_eq!(filter.action(&study), Action::Keep);
    // Note types only for notes
    let not_a_note = element(Kind::Title, &[("type", "study")]);
    assert_eq!(filter.action(&not_a_note), Action::Drop);
    // Rules come first, latest first, and `None` passes
    let filter = filter
        .rule(|e| (e.attr("type") == Some("Study")).then_some(Action::Drop))
        .rule(|_| None);
    assert_eq!(filter.action(&study), Action::Drop);
    let filter = filter.rule(|e| (e.kind == Kind::Note).then_some(Action::Keep));
    assert_eq!(filter.action(&study), Action::Keep);
    // Setting a kind again replaces it
    let filter = Filter::all()
        .drop(Kind::Word)
        .keep(Kind::Word)
        .unwrap(Kind::Word);
    assert_eq!(filter.action(&element(Kind::Word, &[])), Action::Unwrap);
    let filter = Filter::all().kind(Kind::Word, Action::Drop);
    assert_eq!(filter.action(&element(Kind::Word, &[])), Action::Drop);
}

#[test]
fn notes_and_cross_references() {
    let nodes = parse(
        Markup::Osis,
        r#"<title>The Word</title>In the beginning<note type="crossReference">Gen 1:1</note><note type="explanation">Or, origin</note>"#,
    );
    assert_eq!(
        plain_text(&nodes, &TextOptions::default()),
        "In the beginning"
    );
    let footnotes = TextOptions {
        filter: Filter::text()
            .keep(Kind::Note)
            .note_type("crossReference", Action::Drop),
        ..TextOptions::default()
    };
    assert_eq!(
        plain_text(&nodes, &footnotes),
        "In the beginning [Or, origin]"
    );
    let everything = TextOptions {
        filter: Filter::all(),
        ..TextOptions::default()
    };
    assert_eq!(
        plain_text(&nodes, &everything),
        "The Word\nIn the beginning [Gen 1:1] [Or, origin]"
    );
}

#[test]
fn unwrap_keeps_the_contents_only() {
    let nodes = parse(
        Markup::Osis,
        r#"<q who="Jesus">I am <w lemma="strong:G1">the</w> way</q>"#,
    );
    let html = HtmlRenderer::with_options(HtmlOptions {
        filter: Filter::all().unwrap(Kind::WordsOfJesus).drop(Kind::Word),
        ..HtmlOptions::default()
    })
    .render(&nodes);
    assert_eq!(html, "I am  way");
}

#[test]
fn clone_and_debug() {
    let filter = Filter::text().rule(|_| None);
    let copy = filter.clone();
    assert_eq!(copy.action(&element(Kind::Note, &[])), Action::Drop);
    let shown = format!("{filter:?}");
    assert!(shown.contains("Filter"));
}
