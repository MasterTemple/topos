//! `HtmlRenderer` and `HtmlOptions` (docs/spec/html.md).

use topos_bible_markup::{
    Action, Filter, Flattener, HtmlOptions, HtmlRenderer, Kind, Markup, TextOptions,
    flatten::flatten, parse,
};

fn render(markup: Markup, input: &str) -> String {
    HtmlRenderer::new().render(&parse(markup, input))
}

#[test]
fn classes_and_data_attributes() {
    let nodes = parse(
        Markup::Osis,
        r#"<title type="psalm">A Psalm</title><w lemma="strong:H3068" morph="x">LORD</w><note type="crossReference" n="a">Ps 1:1</note><note type="explanation">Or, Jehovah</note> <hi type="small-caps">x</hi><transChange type="added">is</transChange>"#,
    );
    assert_eq!(
        HtmlRenderer::new().render(&nodes),
        r#"<div class="topos-title topos-psalm" data-title-type="psalm">A Psalm</div><span class="topos-w" data-strong="H3068" data-lemma="strong:H3068" data-morph="x">LORD</span><span class="topos-note topos-crossref" data-note-type="crossReference" data-n="a">Ps 1:1</span><span class="topos-note topos-explanation" data-note-type="explanation">Or, Jehovah</span> <span class="topos-hi topos-small-caps">x</span><span class="topos-added">is</span>"#
    );
    // No prefix, only footnotes, words unwrapped
    let mut bare = HtmlRenderer::with_options(HtmlOptions {
        class_prefix: "".into(),
        filter: Filter::all()
            .drop(Kind::Title)
            .unwrap(Kind::Word)
            .note_type("crossReference", Action::Drop),
    });
    let html = bare.render(&nodes);
    assert!(
        html.starts_with(r#"LORD<span class="note explanation" data-note-type="explanation">"#),
        "{html}"
    );
}

#[test]
fn wrap() {
    let html = HtmlRenderer::new();
    assert_eq!(html.wrap("x"), r#"<div class="topos">x</div>"#);
    let bare = HtmlRenderer::with_options(HtmlOptions {
        class_prefix: "".into(),
        ..HtmlOptions::default()
    });
    assert_eq!(bare.wrap("x"), "<div>x</div>");
    let custom = HtmlRenderer::with_options(HtmlOptions {
        class_prefix: "bible_".into(),
        ..HtmlOptions::default()
    });
    assert_eq!(
        custom.wrap("<b>x</b>"),
        r#"<div class="bible"><b>x</b></div>"#
    );
    let odd = HtmlRenderer::with_options(HtmlOptions {
        class_prefix: "a\"b-_-".into(),
        ..HtmlOptions::default()
    });
    assert_eq!(odd.wrap(""), r#"<div class="a&quot;b"></div>"#);
}

#[test]
fn doc_example() {
    let mut html = HtmlRenderer::new();
    let verse = html.render(&parse(
        Markup::Osis,
        r#"<w lemma="strong:G2424">Jesus</w> said, <q who="Jesus">Follow me</q>"#,
    ));
    assert_eq!(
        verse,
        r#"<span class="topos-w" data-strong="G2424" data-lemma="strong:G2424">Jesus</span> said, <span class="topos-woj">Follow me</span>"#
    );
    assert_eq!(
        html.wrap(&verse),
        format!(r#"<div class="topos">{verse}</div>"#)
    );
}

#[test]
fn tags_by_kind() {
    let cases = [
        (
            r#"<hi type="italic">x</hi>"#,
            r#"<i class="topos-hi topos-italic">x</i>"#,
        ),
        (
            r#"<hi type="bold">x</hi>"#,
            r#"<b class="topos-hi topos-bold">x</b>"#,
        ),
        (
            r#"<hi type="super">x</hi>"#,
            r#"<sup class="topos-hi topos-super">x</sup>"#,
        ),
        (
            r#"<hi type="sub">x</hi>"#,
            r#"<sub class="topos-hi topos-sub">x</sub>"#,
        ),
        (
            r#"<hi type="x-Odd Style!">x</hi>"#,
            r#"<span class="topos-hi topos-x-odd-style-">x</span>"#,
        ),
        (
            r#"<hi rend="Italic">x</hi>"#,
            r#"<i class="topos-hi topos-italic">x</i>"#,
        ),
        (r#"<hi>x</hi>"#, r#"<span class="topos-hi">x</span>"#),
        (r#"<p>x</p>"#, r#"<p class="topos-p">x</p>"#),
        (r#"<p/>"#, r#"<br class="topos-p">"#),
        (r#"<p sID="a"/>"#, r#"<br class="topos-p">"#),
        (r#"<p eID="a"/>"#, ""),
        (
            r#"<lg><l>x</l></lg>"#,
            r#"<div class="topos-lg"><span class="topos-l">x</span><br class="topos-l"></div>"#,
        ),
        (r#"<l sID="a"/>x<l eID="a"/>"#, r#"x<br class="topos-l">"#),
        (r#"<lb/>"#, r#"<br class="topos-lb">"#),
        (
            r#"<reference osisRef="Gen.1.1">x</reference>"#,
            r#"<a class="topos-ref" data-osis-ref="Gen.1.1">x</a>"#,
        ),
        (r#"<q>x</q>"#, r#"<span class="topos-quote">x</span>"#),
        (
            r#"<foreign>x</foreign>"#,
            r#"<span class="topos-foreign">x</span>"#,
        ),
        (
            r#"<divineName>x</divineName>"#,
            r#"<span class="topos-divine-name">x</span>"#,
        ),
        (
            r#"<verse osisID="Gen.1.1" n="1"/>"#,
            r#"<span class="topos-verse" data-osis-id="Gen.1.1" data-n="1"></span>"#,
        ),
        (r#"<verse eID="v"/>"#, ""),
        (
            r#"<chapter sID="Gen.1"/>"#,
            r#"<span class="topos-chapter" data-osis-id="Gen.1"></span>"#,
        ),
        (r#"<verse osisID="Gen.1.1">text</verse>"#, "text"),
        (
            r#"<div type="section">x</div><figure>y</figure><seg>z</seg><milestone/>"#,
            "xyz",
        ),
        (r#"<note type="study"/>x"#, "x"),
        (r#"<title/>x"#, "x"),
    ];
    for (input, expected) in cases {
        assert_eq!(render(Markup::Osis, input), expected, "{input}");
    }
    assert_eq!(
        render(Markup::Tei, "<entryFree>x</entryFree>"),
        r#"<div class="topos-entry">x</div>"#
    );
    assert_eq!(
        render(Markup::Usx, r#"<note caller="+" style="f">x</note>"#),
        r#"<span class="topos-note topos-footnote" data-note-type="footnote" data-n="+">x</span>"#
    );
    assert_eq!(
        render(Markup::Usx, r#"<ref loc="GEN 1:1">x</ref>"#),
        r#"<a class="topos-ref" data-osis-ref="GEN 1:1">x</a>"#
    );
}

#[test]
fn escaping() {
    assert_eq!(
        render(
            Markup::Osis,
            r#"<w lemma="a&quot;b" morph="&lt;&amp;">x &lt; y &gt; &quot;z&quot; 's</w>"#
        ),
        r#"<span class="topos-w" data-lemma="a&quot;b" data-morph="&lt;&amp;">x &lt; y &gt; &quot;z&quot; 's</span>"#
    );
}

#[test]
fn metadata_is_hidden_by_default() {
    let usx = r#"<usx><book code="JHN" style="id">World English Bible</book><para style="p"><verse number="16" style="v" sid="JHN 3:16"/>For God<verse eid="JHN 3:16"/></para></usx>"#;
    let html = render(Markup::Usx, usx);
    assert!(!html.contains("World English Bible"));
    assert!(
        html.contains(r#"<span class="topos-verse" data-osis-id="JHN 3:16" data-n="16"></span>"#)
    );
    let all = HtmlRenderer::with_options(HtmlOptions {
        filter: Filter::all(),
        ..HtmlOptions::default()
    })
    .render(&parse(Markup::Usx, usx));
    assert!(all.starts_with("World English Bible"));
}

#[test]
fn words_of_jesus_carry_across_entries() {
    let mut html = HtmlRenderer::new();
    let first = html.render(&parse(
        Markup::Osis,
        r#"end of quote.</q> New <q who="Jesus">start"#,
    ));
    assert!(
        first.ends_with(r#"<span class="topos-woj">start</span>"#),
        "{first}"
    );
    let second = html.render(&parse(Markup::Osis, "more words</q> narration"));
    assert_eq!(
        second,
        r#"<span class="topos-woj">more words</span> narration"#
    );
}

#[test]
fn words_of_jesus_milestones_match_by_id() {
    let a = parse(
        Markup::Osis,
        r#"He said, <q who="Jesus" sID="q1"/>I am <q sID="q2"/>the way<q eID="q2"/>"#,
    );
    let b = parse(Markup::Osis, r#"and the life.<q eID="q1"/> Then"#);
    let mut html = HtmlRenderer::new();
    let out = format!("{}|{}", html.render(&a), html.render(&b));
    assert_eq!(
        out,
        r#"He said, <span class="topos-woj">I am </span><span class="topos-woj">the way</span>|<span class="topos-woj">and the life.</span> Then"#
    );
}

#[test]
fn marks_in_one_entry() {
    let nodes = parse(
        Markup::Osis,
        "See <hi type=\"italic\">John</hi> 3:16 today.",
    );
    let flat = flatten(&nodes, &TextOptions::default());
    let (start, end) = flat.source_range(4..13).expect("from the tree");
    let mut html = HtmlRenderer::new();
    assert_eq!(html.mark(start, end), 0);
    assert_eq!(
        html.render(&nodes),
        r#"See <i class="topos-hi topos-italic"><mark class="topos-mark" data-mark="0">John</mark></i><mark class="topos-mark" data-mark="0"> 3:16</mark> today."#
    );
}

#[test]
fn marks_across_entries_and_elements() {
    let a = parse(Markup::Osis, r#"Read <w lemma="strong:G1">John</w>"#);
    let b = parse(Markup::Osis, r#"3:16 and Rom 8:28 &amp; Rom 8:28."#);
    let mut flat = Flattener::new(TextOptions::default());
    flat.push(&a).push(&b);
    let flat = flat.finish();
    assert_eq!(flat.text, "Read John 3:16 and Rom 8:28 & Rom 8:28.");
    let mut html = HtmlRenderer::new();
    for (i, range) in [5..14, 19..27, 30..38, 28..38].into_iter().enumerate() {
        let (start, end) = flat.source_range(range).expect("from the tree");
        assert_eq!(html.mark(start, end), i);
    }
    let out = format!("{}|{}", html.render(&a), html.render(&b));
    assert_eq!(
        out,
        r#"Read <span class="topos-w" data-strong="G1" data-lemma="strong:G1"><mark class="topos-mark" data-mark="0">John</mark></span>|<mark class="topos-mark" data-mark="0">3:16</mark> and <mark class="topos-mark" data-mark="1">Rom 8:28</mark> <mark class="topos-mark" data-mark="3">&amp; Rom 8:28</mark>."#
    );
}

#[test]
fn clearing_marks() {
    let nodes = parse(Markup::Osis, "abc");
    let flat = flatten(&nodes, &TextOptions::default());
    let (start, end) = flat.source_range(0..3).expect("from the tree");
    let mut html = HtmlRenderer::new();
    html.mark(start.clone(), end.clone());
    html.clear_marks();
    assert_eq!(html.render(&nodes), "abc");
    // The entry count is not reset: these points are in entry 0, and this is entry 1
    assert_eq!(html.mark(start, end), 0);
    assert_eq!(html.render(&nodes), "abc");
}
