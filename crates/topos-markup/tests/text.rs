//! `plain_text` and `TextOptions` (docs/spec/plain-text.md).

use std::collections::BTreeMap;

use topos_bible_markup::{Filter, Kind, Markup, TextOptions, parse, plain_text};

fn text(markup: Markup, input: &str, opts: &TextOptions) -> String {
    plain_text(&parse(markup, input), opts)
}

fn default_text(markup: Markup, input: &str) -> String {
    text(markup, input, &TextOptions::default())
}

fn templates(list: &[(Kind, &str)]) -> BTreeMap<Kind, String> {
    list.iter().map(|(k, t)| (*k, t.to_string())).collect()
}

#[test]
fn defaults() {
    let opts = TextOptions::default();
    assert!(!opts.strongs);
    assert!(opts.line_breaks);
    assert!(opts.verse_label.is_none() && opts.chapter_label.is_none());
    assert!(opts.templates.is_empty());
}

#[test]
fn osis_verse_text_and_notes() {
    let input = r#"<div sID="gen4" type="x-p"/>And <w lemma="strong:H0430">God</w> <w lemma="strong:H0559" morph="x:1">said</w>, <q who="Jesus" sID="q1"/>Let there be light<note type="crossReference" n="a"><reference osisRef="John.1.5">John 1:5</reference></note>: &amp; <transChange type="added">it was</transChange> <divineName>Lord</divineName>"#;
    assert_eq!(
        default_text(Markup::Osis, input),
        "And God said, Let there be light: & it was Lord"
    );
    let notes = TextOptions {
        filter: Filter::text().keep(Kind::Note),
        ..TextOptions::default()
    };
    assert!(text(Markup::Osis, input, &notes).contains("light [John 1:5]:"));
}

#[test]
fn readme_example() {
    let nodes = parse(
        Markup::Osis,
        r#"<w lemma="strong:G2316">God</w> so loved the world<note type="explanation">Or, cosmos</note>"#,
    );
    assert_eq!(
        plain_text(&nodes, &TextOptions::default()),
        "God so loved the world"
    );
    let with_notes = TextOptions {
        filter: Filter::text().keep(Kind::Note),
        ..TextOptions::default()
    };
    assert_eq!(
        plain_text(&nodes, &with_notes),
        "God so loved the world [Or, cosmos]"
    );
}

#[test]
fn whitespace_and_breaks() {
    assert_eq!(
        default_text(Markup::Plaintext, "one\r\ntwo  words\n\n\nthree"),
        "one\ntwo words\n\nthree"
    );
    assert_eq!(
        default_text(Markup::Osis, "  lead\t\ttabs   and\u{a0}nbsp  "),
        "lead tabs and nbsp"
    );
    assert_eq!(default_text(Markup::Osis, "a\n\n\nb"), "a b");
    assert_eq!(
        default_text(Markup::Osis, "a<lb/>b<lb/><lb/>c<lb/><lb/><lb/>d"),
        "a\nb\nc\n\nd"
    );
    assert_eq!(
        default_text(Markup::Osis, "<p>one</p><p>two</p>three"),
        "one\ntwo\nthree"
    );
    assert_eq!(default_text(Markup::Osis, "a<hi>b</hi> c"), "ab c");
    assert_eq!(default_text(Markup::Osis, "   "), "");
    assert_eq!(default_text(Markup::Osis, ""), "");
}

#[test]
fn line_breaks_off() {
    let input = r#"ιουδας<sync type="Strongs" value="G2455" /><sync type="morph" class="Robinson" value="N-NSM" /> δε<sync type="Strongs" value="G1161" /> εγεννησεν<sync type="Strongs" value="G1080" /><sync type="Strongs" value="G5656" /><br>τον"#;
    let opts = TextOptions {
        line_breaks: false,
        ..TextOptions::default()
    };
    assert_eq!(text(Markup::Thml, input, &opts), "ιουδας δε εγεννησεν τον");
    assert_eq!(text(Markup::Plaintext, "a\n\n\nb", &opts), "a b");
}

#[test]
fn tei_entry() {
    let input = r#"<entryFree n="00003"> <orth>Ἀβαδδών</orth> <pron rend="italic">ab-ad-dohn'</pron><lb/> <def>of Hebrew origin (<ref target="StrongsHebrew:011">H11</ref>); a destroying angel</def> </entryFree>"#;
    assert_eq!(
        default_text(Markup::Tei, input),
        "Ἀβαδδών ab-ad-dohn'\nof Hebrew origin (H11); a destroying angel"
    );
}

#[test]
fn gbf_text() {
    let input = "In the beginning<WH7225> God<WH430><WTN-M> said,<FR>Let there<FI>be<Fi> light<Fr><RF>a note<Rf><CM><<Geschrieben von Rom.>";
    let out = default_text(Markup::Gbf, input);
    assert!(out.starts_with("In the beginning God said,Let therebe light"));
    assert!(out.ends_with("<<Geschrieben von Rom.>"));
}

#[test]
fn usx_text_skips_metadata_notes_and_titles() {
    let input = r#"<usx version="3.0">
  <book code="JHN" style="id">World English Bible</book>
  <chapter number="3" style="c" sid="JHN 3"/>
  <para style="s1">Jesus and Nicodemus</para>
  <para style="p">
    <verse number="16" style="v" sid="JHN 3:16"/><char style="wj">For God so <char style="w" strong="G25">loved</char> the world<note caller="+" style="x"><char style="xo">3:16 </char><char style="xt">Rom 5:8</char></note>, that he gave his one and only Son</char>.<verse eid="JHN 3:16"/>
  </para>
  <para style="q1">The <char style="nd">Lord</char> is my shepherd</para>
  <chapter eid="JHN 3"/>
</usx>"#;
    assert_eq!(
        default_text(Markup::Usx, input),
        "For God so loved the world, that he gave his one and only Son.\nThe Lord is my shepherd"
    );
}

#[test]
fn strongs_after_words() {
    let opts = TextOptions {
        strongs: true,
        ..TextOptions::default()
    };
    assert_eq!(
        text(
            Markup::Osis,
            r#"In the <w lemma="strong:H7225 strong:H853">beginning</w> <w lemma="lemma.TR:x">God</w>"#,
            &opts
        ),
        "In the beginning [H7225 H853] God"
    );
}

#[test]
fn templates_glue_to_their_element() {
    let input = r#"<usx><para style="s1">The Word</para><para style="p">Jesus said, <char style="wj"> I am the light</char>, and<note style="f"><char style="ft">Or bread</char></note> went.</para></usx>"#;
    let opts = TextOptions {
        filter: Filter::all().drop(Kind::Metadata),
        templates: templates(&[
            (Kind::Title, "## {text}"),
            (Kind::WordsOfJesus, "“{text}”"),
            (Kind::Note, "^({text})"),
        ]),
        ..TextOptions::default()
    };
    assert_eq!(
        text(Markup::Usx, input, &opts),
        "## The Word\nJesus said, “I am the light”, and ^(Or bread) went."
    );
    let plain = TextOptions {
        filter: Filter::all().drop(Kind::Metadata),
        ..TextOptions::default()
    };
    assert!(text(Markup::Usx, input, &plain).contains("and [Or bread] went"));
}

#[test]
fn template_shapes() {
    let opts = |list: &[(Kind, &str)]| TextOptions {
        templates: templates(list),
        ..TextOptions::default()
    };
    // Without `{text}` the template goes before; only the first `{text}` splits
    assert_eq!(
        text(
            Markup::Osis,
            "a <divineName>Lord</divineName> b",
            &opts(&[(Kind::DivineName, "^")])
        ),
        "a ^Lord b"
    );
    assert_eq!(
        text(
            Markup::Osis,
            "<transChange>x</transChange>",
            &opts(&[(Kind::Added, "<{text}|{text}>")])
        ),
        "<x|{text}>"
    );
    // An empty element still gets both parts; words ignore templates
    assert_eq!(
        text(
            Markup::Osis,
            r#"a<q who="Jesus"></q>b"#,
            &opts(&[(Kind::WordsOfJesus, "“{text}”")])
        ),
        "a“”b"
    );
    assert_eq!(
        text(
            Markup::Osis,
            "<w>word</w>",
            &opts(&[(Kind::Word, "*{text}*")])
        ),
        "word"
    );
}

#[test]
fn verse_and_chapter_labels() {
    let usx = r#"<usx><book code="JHN" style="id">x</book><chapter number="3" style="c" sid="JHN 3"/>
<para style="p"><verse number="16" style="v" sid="JHN 3:16"/>For God so <char style="wj">loved</char> the world.<verse eid="JHN 3:16"/><verse number="17-18" style="v" sid="JHN 3:17-18"/>For God<verse eid="JHN 3:17-18"/></para><chapter eid="JHN 3"/></usx>"#;
    let opts = TextOptions {
        verse_label: Some("[{verse}]".into()),
        chapter_label: Some("{book} {chapter}".into()),
        ..TextOptions::default()
    };
    assert_eq!(
        text(Markup::Usx, usx, &opts),
        "JHN 3\n[16] For God so loved the world. [17-18] For God"
    );
    // Without labels, verse markers write nothing
    assert!(default_text(Markup::Usx, usx).starts_with("For God"));

    let osis =
        r#"<verse osisID="John.3.16">For God</verse> <verse osisID="John.3.17">sent</verse>"#;
    let opts = TextOptions {
        verse_label: Some("{book} {chapter}:{verse}".into()),
        ..TextOptions::default()
    };
    assert_eq!(
        text(Markup::Osis, osis, &opts),
        "John 3:16 For God John 3:17 sent"
    );
}

#[test]
fn label_values() {
    let opts = TextOptions {
        verse_label: Some("[{book}|{chapter}|{verse}]".into()),
        chapter_label: Some("<{book}|{chapter}|{verse}>".into()),
        ..TextOptions::default()
    };
    // osis2mod's ids: the sID is the id
    assert_eq!(
        text(
            Markup::Osis,
            r#"<chapter osisID="Gen.1" sID="gen2"/><verse osisID="Gen.1.1" sID="gen3"/>x"#,
            &opts
        ),
        "<gen2||>\n[gen3||] x"
    );
    // The current book and chapter carry to verses without them, and an empty label is not written
    assert_eq!(
        text(
            Markup::Osis,
            r#"<chapter osisID="Exod.4"/><verse n="2"/>x"#,
            &opts
        ),
        "<Exod|4|>\n[Exod|4|2] x"
    );
    let blank = TextOptions {
        verse_label: Some("{verse}".into()),
        ..TextOptions::default()
    };
    assert_eq!(text(Markup::Osis, "a<verse/>b", &blank), "ab");
    // End markers have no label; a dropped chapter still sets the chapter
    let dropped = TextOptions {
        filter: Filter::text().drop(Kind::Chapter),
        verse_label: Some("{chapter}:{verse}".into()),
        ..TextOptions::default()
    };
    assert_eq!(
        text(
            Markup::Usx,
            r#"<chapter number="7"/><verse number="1"/>a<verse eid="X 7:1"/>"#,
            &dropped
        ),
        "7:1 a"
    );
}
