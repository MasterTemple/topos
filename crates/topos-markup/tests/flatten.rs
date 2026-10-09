//! `Flat`, `Flattener`, spans, entries, and sources (docs/spec/flatten.md).

use topos_bible_markup::{Flattener, Kind, Markup, NodePath, TextOptions, flatten::flatten, parse};

fn path(p: &[usize]) -> NodePath {
    NodePath(p.to_vec())
}

#[test]
fn text_maps_back_to_the_tree() {
    let nodes = parse(
        Markup::Osis,
        "See <hi type=\"italic\">John</hi>   3:16\n<note>n</note> now",
    );
    let flat = flatten(&nodes, &TextOptions::default());
    assert_eq!(flat.text, "See John 3:16 now");
    let (start, end) = flat.source_range(4..13).expect("from the tree");
    assert_eq!(
        (start.entry, start.path.clone(), start.offset),
        (0, path(&[1, 0]), 0)
    );
    assert_eq!(
        (end.entry, end.path.clone(), end.offset),
        (0, path(&[2]), 7)
    );
    assert_eq!(
        start.path.resolve(&nodes).and_then(|n| n.as_text()),
        Some("John")
    );
    assert_eq!(flat.source(8), None, "collapsed whitespace has no source");
    let at = flat.source(10).expect("from the tree");
    assert_eq!((at.path, at.offset), (path(&[2]), 4));
    let hi = flat.spans_at(4..8).next().expect("a span");
    assert_eq!(
        (hi.kind, &flat.text[hi.range.clone()]),
        (Kind::Highlight, "John")
    );
    assert_eq!(flat.source_range(3..4), None);
    assert_eq!(flat.source_range(0..0), None);
}

#[test]
fn added_text_has_no_source() {
    let opts = TextOptions {
        strongs: true,
        verse_label: Some("[{verse}]".into()),
        ..TextOptions::default()
    };
    let nodes = parse(Markup::Osis, r#"<verse n="1"/><w lemma="strong:G1">x</w>"#);
    let flat = flatten(&nodes, &opts);
    assert_eq!(flat.text, "[1] x [G1]");
    assert_eq!(flat.source(0), None);
    assert!(flat.source(4).is_some());
    assert_eq!(flat.source(7), None);
    // The word's span covers its Strong's list
    let word = flat
        .spans
        .iter()
        .find(|s| s.kind == Kind::Word)
        .expect("a word span");
    assert_eq!(&flat.text[word.range.clone()], "x [G1]");
    // The verse's span covers its label
    let verse = flat
        .spans
        .iter()
        .find(|s| s.kind == Kind::Verse)
        .expect("a verse span");
    assert_eq!(&flat.text[verse.range.clone()], "[1]");
}

#[test]
fn spans_order_and_trimming() {
    let nodes = parse(
        Markup::Osis,
        "<lg><l> first </l><l>second</l></lg><milestone/>",
    );
    let flat = flatten(&nodes, &TextOptions::default());
    assert_eq!(flat.text, "first\nsecond");
    let spans: Vec<(Kind, &str, String)> = flat
        .spans
        .iter()
        .map(|s| (s.kind, &flat.text[s.range.clone()], s.path.to_string()))
        .collect();
    assert_eq!(
        spans,
        [
            (Kind::Line, "first", "/0/0".into()),
            (Kind::Line, "second", "/0/1".into()),
            (Kind::LineGroup, "first\nsecond", "/0".into()),
            (Kind::Milestone, "", "/1".into()),
        ]
    );
    // `spans_at` keeps that order
    let at: Vec<Kind> = flat.spans_at(0..2).map(|s| s.kind).collect();
    assert_eq!(at, [Kind::Line, Kind::LineGroup]);
}

#[test]
fn last_before_finds_the_verse() {
    let usx = r#"<usx><chapter number="3" style="c" sid="JHN 3"/><para style="p"><verse number="16" style="v" sid="JHN 3:16"/>For God so loved the world.<verse eid="JHN 3:16"/><verse number="17" style="v" sid="JHN 3:17"/> For God sent</para></usx>"#;
    let nodes = parse(Markup::Usx, usx);
    let flat = flatten(&nodes, &TextOptions::default());
    let at = flat.text.find("world").expect("in the text");
    let verse = flat.last_before(Kind::Verse, at).expect("a verse");
    let marker = verse
        .path
        .resolve(&nodes)
        .and_then(|n| n.as_element())
        .expect("an element");
    assert_eq!(marker.attr("sid"), Some("JHN 3:16"));
    let at = flat.text.find("sent").expect("in the text");
    let verse = flat.last_before(Kind::Verse, at).expect("a verse");
    let marker = verse
        .path
        .resolve(&nodes)
        .and_then(|n| n.as_element())
        .expect("an element");
    assert_eq!(marker.attr("sid"), Some("JHN 3:17"));
    assert!(flat.last_before(Kind::Note, at).is_none());
}

#[test]
fn entries_and_their_ranges() {
    let mut flat = Flattener::new(TextOptions::default());
    flat.push(&parse(Markup::Osis, "In the beginning "))
        .push(&parse(Markup::Osis, "<note>dropped</note>"))
        .push(&parse(Markup::Osis, " was <q who=\"Jesus\">the Word"))
        .push(&parse(Markup::Osis, "</q>"));
    let flat = flat.finish();
    assert_eq!(flat.text, "In the beginning was the Word");
    let entries: Vec<&str> = flat.entries.iter().map(|r| &flat.text[r.clone()]).collect();
    assert_eq!(entries, ["In the beginning", "", "was the Word", ""]);
    assert_eq!(flat.entries[1], 17..17);
    assert_eq!(flat.entries[3], 29..29);
}

#[test]
fn entries_are_separated_by_a_space() {
    let mut flat = Flattener::new(TextOptions::default());
    flat.push(&parse(Markup::Osis, "one"))
        .push(&parse(Markup::Osis, "two"));
    assert_eq!(flat.finish().text, "one two");
    let empty = Flattener::new(TextOptions::default()).finish();
    assert_eq!(empty.text, "");
    assert!(empty.entries.is_empty() && empty.spans.is_empty());
}

#[test]
fn words_of_jesus_across_entries() {
    // Unclosed in one entry, closed by an orphan end in the next
    let mut flat = Flattener::new(TextOptions::default());
    flat.push(&parse(Markup::Osis, r#"He said, <q who="Jesus">Follow"#));
    flat.push(&parse(Markup::Osis, "me.</q> And they did."));
    let flat = flat.finish();
    assert_eq!(flat.text, "He said, Follow me. And they did.");
    let red: Vec<(&str, usize, String)> = flat
        .spans
        .iter()
        .filter(|s| s.kind == Kind::WordsOfJesus)
        .map(|s| (&flat.text[s.range.clone()], s.entry, s.path.to_string()))
        .collect();
    assert_eq!(red, [("Follow", 0, "/1".into()), ("me.", 1, String::new())]);
}

#[test]
fn words_of_jesus_milestones_match_by_id() {
    let a = parse(
        Markup::Osis,
        r#"He said, <q who="Jesus" sID="q1"/>I am <q sID="q2"/>the way<q eID="q2"/>"#,
    );
    let b = parse(Markup::Osis, r#"and the life.<q eID="q1"/> Then"#);
    let mut flat = Flattener::new(TextOptions::default());
    flat.push(&a).push(&b);
    let flat = flat.finish();
    let red: Vec<&str> = flat
        .spans
        .iter()
        .filter(|s| s.kind == Kind::WordsOfJesus)
        .map(|s| &flat.text[s.range.clone()])
        .collect();
    assert_eq!(red, ["I am the way", "and the life."]);
    // Unmatched ids leave them open
    let nodes = parse(
        Markup::Osis,
        r#"a<q who="Jesus" sID="q1"/>b<q eID="q9"/>c<q eID="q1"/>d"#,
    );
    let flat = flatten(&nodes, &TextOptions::default());
    let red: Vec<&str> = flat
        .spans
        .iter()
        .filter(|s| s.kind == Kind::WordsOfJesus)
        .map(|s| &flat.text[s.range.clone()])
        .collect();
    assert_eq!(red, ["bc"]);
}

#[test]
fn templates_across_entries() {
    let opts = TextOptions {
        templates: [(Kind::WordsOfJesus, "“{text}”".to_string())]
            .into_iter()
            .collect(),
        ..TextOptions::default()
    };
    let mut flat = Flattener::new(opts.clone());
    flat.push(&parse(
        Markup::Osis,
        r#"He said, <q who="Jesus" sID="q1"/>Follow"#,
    ))
    .push(&parse(Markup::Osis, r#"me.<q eID="q1"/> And they did."#));
    assert_eq!(flat.finish().text, "He said, “Follow me.” And they did.");
    let mut flat = Flattener::new(opts);
    flat.push(&parse(Markup::Osis, r#"<q who="Jesus">never closed"#));
    assert_eq!(flat.finish().text, "“never closed”");
}
