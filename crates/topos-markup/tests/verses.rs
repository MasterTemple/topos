//! Verses in a book's tree (docs/spec/verses.md).

// Expected passage parts are lists of index ranges, often just one
#![allow(clippy::single_range_in_vec_init)]

use topos_bible::data::BibleData;
use topos_bible_markup::{
    Filter, HtmlRenderer, Kind, Markup, Node, NodePath, TextOptions, find_all, parse, plain_text,
    verses::{VerseSpan, chapter_verses, passage_ranges, slice, usx_chapter_verses, verses},
};

const PSALMS: &str = r#"<usx version="3.0"><book code="PSA" style="id"/>
<para style="mt1">Psalms</para>
<chapter number="1" style="c" sid="PSA 1"/>
<para style="d">A heading</para>
<para style="q1"><verse number="1" style="v" sid="PSA 1:1"/>Blessed is the man</para>
<para style="q2">who does not walk<verse eid="PSA 1:1"/></para>
<para style="s1">The way</para>
<para style="q1"><verse number="2-3" style="v" sid="PSA 1:2-3"/>But his delight<verse eid="PSA 1:2-3"/><verse number="4" style="v" sid="PSA 1:4"/>Not so<verse eid="PSA 1:4"/></para>
<chapter eid="PSA 1"/>
<chapter number="2" style="c" sid="PSA 2"/>
<para style="q1"><verse number="1" style="v" sid="PSA 2:1"/>Why do the nations rage<verse eid="PSA 2:1"/></para>
<chapter eid="PSA 2"/></usx>"#;

fn all_text(nodes: &[Node]) -> String {
    plain_text(
        nodes,
        &TextOptions {
            filter: Filter::all(),
            ..TextOptions::default()
        },
    )
}

fn numbers(spans: &[VerseSpan]) -> Vec<(u8, u8, u8)> {
    spans.iter().map(|s| (s.chapter, s.verse, s.last)).collect()
}

#[test]
fn usx_verses_cross_paragraphs_and_keep_headings() {
    let nodes = parse(Markup::Usx, PSALMS);
    let spans = verses(&nodes);
    assert_eq!(
        numbers(&spans),
        [(1, 1, 1), (1, 2, 3), (1, 4, 4), (2, 1, 1)]
    );
    assert_eq!(chapter_verses(&spans), [4, 1]);
    assert_eq!(usx_chapter_verses(PSALMS), [4, 1]);
    let verse = |i: usize| slice(&nodes, &spans[i].start, Some(&spans[i].end));
    // The heading after the chapter marker goes with verse 1, which keeps both its lines
    let first = verse(0);
    assert_eq!(
        all_text(&first),
        "A heading\nBlessed is the man\nwho does not walk"
    );
    assert_eq!(find_all(&first, Kind::Line).len(), 2);
    // A heading between verses belongs to the verse after it; the book title to none
    assert_eq!(all_text(&verse(1)), "The way\nBut his delight");
    assert_eq!(all_text(&verse(2)), "Not so");
    assert_eq!(all_text(&verse(3)), "Why do the nations rage");
    assert!(!(0..4).any(|i| all_text(&verse(i)).contains("Psalms")));
    // Markers and places
    let marker = spans[0]
        .marker
        .resolve(&nodes)
        .and_then(Node::as_element)
        .expect("a marker");
    assert_eq!(marker.attr("sid"), Some("PSA 1:1"));
    let end = spans[0]
        .end
        .resolve(&nodes)
        .and_then(Node::as_element)
        .expect("an end marker");
    assert_eq!(end.attr("eid"), Some("PSA 1:1"));
}

#[test]
fn passages_pick_verses() {
    let nodes = parse(Markup::Usx, PSALMS);
    let spans = verses(&nodes);
    let data = BibleData::default();
    let ranges = |r: &str| passage_ranges(&spans, &data.parse(r).expect("a reference"));
    // A bridged verse is in the passage when any of its verses is
    assert_eq!(ranges("Psalm 1:3"), [1..2]);
    assert_eq!(ranges("Psalm 1:1-3"), [0..2]);
    assert_eq!(ranges("Psalm 1"), [0..3]);
    assert_eq!(ranges("Psalm 1:4; 2"), [2..3, 3..4]);
    assert!(ranges("Psalm 3").is_empty());
    // A whole chapter slices to all its verses, with the cut paragraph left whole
    let r = &ranges("Psalm 1")[0];
    let chapter = slice(&nodes, &spans[r.start].start, Some(&spans[r.end - 1].end));
    assert_eq!(
        all_text(&chapter),
        "A heading\nBlessed is the man\nwho does not walk\nThe way\nBut his delightNot so"
    );
}

#[test]
fn osis_verse_elements_and_milestones() {
    let elements = parse(
        Markup::Osis,
        r#"<div type="book" osisID="John"><chapter osisID="John.3"><verse osisID="John.3.16">For God</verse> <verse osisID="John.3.17">For God sent</verse></chapter></div>"#,
    );
    let spans = verses(&elements);
    assert_eq!(numbers(&spans), [(3, 16, 16), (3, 17, 17)]);
    assert_eq!(
        all_text(&slice(&elements, &spans[1].start, Some(&spans[1].end))),
        "For God sent"
    );

    let milestones = parse(
        Markup::Osis,
        r#"<chapter sID="John.3" osisID="John.3"/><verse sID="John.3.16" osisID="John.3.16"/>For God<verse eID="John.3.16"/><verse sID="John.3.17" osisID="John.3.17"/>sent<verse eID="John.3.17"/><chapter eID="John.3"/>"#,
    );
    let spans = verses(&milestones);
    assert_eq!(numbers(&spans), [(3, 16, 16), (3, 17, 17)]);
    assert_eq!(
        all_text(&slice(&milestones, &spans[0].start, Some(&spans[0].end))),
        "For God"
    );
    // No end: to the end of the tree
    assert_eq!(all_text(&slice(&milestones, &spans[1].start, None)), "sent");
}

#[test]
fn module_doc_example() {
    let usx = r#"<usx><book code="JHN"/><chapter number="3" sid="JHN 3"/>
<para style="s1">Jesus and Nicodemus</para>
<para style="p"><verse number="16" sid="JHN 3:16"/>For God so loved the world.<verse eid="JHN 3:16"/><verse number="17" sid="JHN 3:17"/>For God did not send His Son.<verse eid="JHN 3:17"/></para>
<chapter eid="JHN 3"/></usx>"#;
    let nodes = parse(Markup::Usx, usx);
    let spans = verses(&nodes);
    assert_eq!((spans[1].chapter, spans[1].verse), (3, 17));
    let html = HtmlRenderer::new().render(&slice(&nodes, &spans[0].start, Some(&spans[0].end)));
    assert!(
        html.contains("Jesus and Nicodemus")
            && html.contains("so loved")
            && !html.contains("did not send")
    );
}

#[test]
fn numbers_from_attributes_and_ids() {
    let nodes = parse(
        Markup::Osis,
        r#"<chapter n="5"/><verse n="4-5"/>a<verse n="6a"/>b<verse n="7,8"/>c<verse n="9–10"/>d<verse osisID="Gen.5.11 Gen.5.12"/>e<verse sID="JHN 5:13"/>f<verse n="x"/>g<verse n="256"/>h<verse n="14-3"/>i"#,
    );
    assert_eq!(
        numbers(&verses(&nodes)),
        [
            (5, 4, 5),
            (5, 6, 6),
            (5, 7, 8),
            (5, 9, 10),
            (5, 12, 12),
            (5, 13, 13),
            (5, 14, 14)
        ]
    );
}

#[test]
fn chapters_without_numbers_count_up() {
    let nodes = parse(
        Markup::Osis,
        r#"<chapter/>a<verse n="1"/>b<chapter/>c<verse n="1"/>d"#,
    );
    assert_eq!(numbers(&verses(&nodes)), [(1, 1, 1), (2, 1, 1)]);
}

#[test]
fn starts_without_end_markers() {
    let nodes = parse(
        Markup::Usx,
        r#"<usx><chapter number="1"/><para style="s1">Greeting</para><para style="p"><verse number="1"/>Jude <verse number="2"/>Mercy</para></usx>"#,
    );
    let spans = verses(&nodes);
    // The first verse starts after the chapter marker (so it has the heading); the second at its marker
    assert_eq!(
        all_text(&slice(&nodes, &spans[0].start, Some(&spans[0].end))),
        "Greeting\nJude"
    );
    assert_eq!(spans[1].start, spans[1].marker);
    assert_eq!(spans[1].end, NodePath(vec![1]));
}

#[test]
fn slices() {
    let nodes = parse(Markup::Osis, "<p>a<hi>b</hi>c</p><p>d</p>e");
    let p = |v: &[usize]| NodePath(v.to_vec());
    assert_eq!(
        all_text(&slice(&nodes, &p(&[0, 1, 0]), Some(&p(&[1, 0])))),
        "bc"
    );
    assert_eq!(all_text(&slice(&nodes, &p(&[0, 2]), None)), "c\nd\ne");
    assert_eq!(all_text(&slice(&nodes, &p(&[0]), Some(&p(&[0, 1])))), "a");
    assert!(slice(&nodes, &p(&[2]), Some(&p(&[1]))).is_empty());
    assert!(
        slice(&nodes, &p(&[0, 3]), Some(&p(&[1]))).is_empty(),
        "an element cut to nothing is left out"
    );
    // Cut elements keep what they were
    let cut = slice(&nodes, &p(&[0, 1]), Some(&p(&[0, 2])));
    let para = cut[0].as_element().expect("the paragraph");
    assert_eq!((para.kind, para.children.len()), (Kind::Paragraph, 1));
}

#[test]
fn counts() {
    let spans = verses(&parse(
        Markup::Usx,
        r#"<chapter number="2"/><verse number="3"/><verse number="5-7"/><chapter number="4"/><verse number="1"/>"#,
    ));
    assert_eq!(chapter_verses(&spans), [0, 7, 0, 1]);
    assert!(chapter_verses(&[]).is_empty());
    assert_eq!(
        usx_chapter_verses(
            r#"<chapter number="2"/><verse number="3"/><verse number='5-7'/><chapter number="4" /><verse altnumber="9" number="1"/>"#
        ),
        [0, 7, 0, 1]
    );
    // Verses before any chapter or in chapter 0 are not counted; bridges count to their end
    assert_eq!(
        usx_chapter_verses(
            r#"<verse number="9"/><chapter number="0"/><verse number="9"/><chapter number="1"/><verse number="4a"/><verse number="2–6"/>"#
        ),
        [6]
    );
    assert!(usx_chapter_verses("").is_empty());
    assert!(usx_chapter_verses("<chapter number=1><verse number=2>").is_empty());
}
