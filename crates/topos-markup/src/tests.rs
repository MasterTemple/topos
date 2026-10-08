use super::*;
use crate::Markup;

fn first_el(nodes: &[Node]) -> &Element {
    nodes.iter().find_map(Node::as_element).unwrap()
}

#[test]
fn osis_words_notes_and_milestones() {
    let src = r#"<div sID="gen4" type="x-p"/>And <w lemma="strong:H0430">God</w> <w lemma="strong:H0559" morph="x:1">said</w>, <q who="Jesus" sID="q1"/>Let there be light<note type="crossReference" n="a"><reference osisRef="John.1.5">John 1:5</reference></note>: &amp; <transChange type="added">it was</transChange> <divineName>Lord</divineName>"#;
    let nodes = parse(Markup::Osis, src);
    let words = find_all(&nodes, Kind::Word);
    assert_eq!(words.len(), 2);
    assert_eq!(words[0].strongs()[0].to_string(), "H430");
    assert_eq!(words[1].morph(), vec!["x:1"]);
    let p = first_el(&nodes);
    assert_eq!(
        (p.kind, p.milestone),
        (Kind::Paragraph, Some(Milestone::Start))
    );
    let q = find_all(&nodes, Kind::WordsOfJesus);
    assert_eq!(q[0].milestone, Some(Milestone::Start));
    let note = &find_all(&nodes, Kind::Note)[0];
    assert!(note.is_cross_reference());
    assert_eq!(
        find_all(&nodes, Kind::Reference)[0].reference_target(),
        Some("John.1.5")
    );
    assert_eq!(
        plain_text(&nodes, &TextOptions::default()),
        "And God said, Let there be light: & it was Lord"
    );
    let with_notes = plain_text(
        &nodes,
        &TextOptions {
            filter: Filter::text().keep(Kind::Note),
            ..Default::default()
        },
    );
    assert!(with_notes.contains("light [John 1:5]:"));
}

#[test]
fn osis_unbalanced_fragments() {
    // closes a <q> opened in an earlier verse, opens one continuing into the next
    let nodes = parse(
        Markup::Osis,
        r#"end of quote.</q> New <q who="Jesus">start"#,
    );
    let q = find_all(&nodes, Kind::Quote);
    assert_eq!(q[0].milestone, Some(Milestone::End));
    let woj = find_all(&nodes, Kind::WordsOfJesus);
    assert!(woj[0].unclosed);
    assert_eq!(woj[0].text(), "start");
    let mut r = HtmlRenderer::new();
    let html = r.render(&nodes);
    assert!(html.ends_with("<span class=\"topos-woj\">start</span>"));
    // red letter carries over to the next entry, and the orphan `</q>` ends it
    let html2 = r.render(&parse(Markup::Osis, "more words</q> narration"));
    assert_eq!(
        html2,
        "<span class=\"topos-woj\">more words</span> narration"
    );
}

#[test]
fn thml_sync_tags_become_words() {
    let src = r#"ιουδας<sync type="Strongs" value="G2455" /><sync type="morph" class="Robinson" value="N-NSM" /> δε<sync type="Strongs" value="G1161" /> εγεννησεν<sync type="Strongs" value="G1080" /><sync type="Strongs" value="G5656" /><br>τον"#;
    let nodes = parse(Markup::Thml, src);
    let words = find_all(&nodes, Kind::Word);
    assert_eq!(words.len(), 3);
    assert_eq!(words[0].text(), "ιουδας");
    assert_eq!(words[0].morph(), vec!["robinson:N-NSM"]);
    assert_eq!(
        words[2]
            .strongs()
            .iter()
            .map(|s| s.number)
            .collect::<Vec<_>>(),
        vec![1080, 5656]
    );
    assert_eq!(find_all(&nodes, Kind::LineBreak).len(), 1);
    assert_eq!(
        plain_text(
            &nodes,
            &TextOptions {
                line_breaks: false,
                ..Default::default()
            }
        ),
        "ιουδας δε εγεννησεν τον"
    );
}

#[test]
fn gbf_tokens() {
    let src = "In the beginning<WH7225> God<WH430><WTN-M> said,<FR>Let there<FI>be<Fi> light<Fr><RF>a note<Rf><CM><<Geschrieben von Rom.>";
    let nodes = parse(Markup::Gbf, src);
    let words = find_all(&nodes, Kind::Word);
    assert_eq!(words.len(), 2);
    assert_eq!(words[0].text(), "beginning");
    assert_eq!(words[1].morph(), vec!["N-M"]);
    let woj = &find_all(&nodes, Kind::WordsOfJesus)[0];
    assert_eq!(woj.text(), "Let therebe light");
    assert_eq!(find_all(&nodes, Kind::Highlight)[0].style(), Some("italic"));
    assert_eq!(find_all(&nodes, Kind::Note)[0].text(), "a note");
    assert_eq!(find_all(&nodes, Kind::Paragraph).len(), 1);
    assert!(plain_text(&nodes, &TextOptions::default()).ends_with("<<Geschrieben von Rom.>"));
}

#[test]
fn tei_dictionary_entry() {
    let src = r#"<entryFree n="00003"> <orth>Ἀβαδδών</orth> <pron rend="italic">ab-ad-dohn'</pron><lb/> <def>of Hebrew origin (<ref target="StrongsHebrew:011">H11</ref>); a destroying angel</def> </entryFree>"#;
    let nodes = parse(Markup::Tei, src);
    assert_eq!(first_el(&nodes).kind, Kind::Entry);
    assert_eq!(
        find_all(&nodes, Kind::Reference)[0].reference_target(),
        Some("StrongsHebrew:011")
    );
    assert_eq!(
        plain_text(&nodes, &TextOptions::default()),
        "Ἀβαδδών ab-ad-dohn'\nof Hebrew origin (H11); a destroying angel"
    );
}

#[test]
fn strongs_parsing() {
    let s = Strongs::parse("strong:H0430a").unwrap();
    assert_eq!(
        (s.lang, s.number, s.suffix.as_deref()),
        (StrongsLang::Hebrew, 430, Some("a"))
    );
    assert_eq!(s.dictionary_key(), "00430");
    assert!(Strongs::parse("lemma.TR:foo").is_none());
    assert_eq!(
        Strongs::parse_list("strong:G1 x-Strongs:G2 lemma.TR:x").len(),
        2
    );
}

#[test]
fn woj_milestones_match_by_id() {
    // OSIS end milestones don't repeat `who`
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
}

#[test]
fn html_classes_prefix_and_filter() {
    let nodes = parse(
        Markup::Osis,
        r#"<title type="psalm">A Psalm</title><w lemma="strong:H3068" morph="x">LORD</w><note type="crossReference" n="a">Ps 1:1</note><note type="explanation">Or, Jehovah</note> <hi type="small-caps">x</hi><transChange type="added">is</transChange>"#,
    );
    let html = HtmlRenderer::new().render(&nodes);
    assert_eq!(
        html,
        r#"<div class="topos-title topos-psalm" data-title-type="psalm">A Psalm</div><span class="topos-w" data-strong="H3068" data-lemma="strong:H3068" data-morph="x">LORD</span><span class="topos-note topos-crossref" data-note-type="crossReference" data-n="a">Ps 1:1</span><span class="topos-note topos-explanation" data-note-type="explanation">Or, Jehovah</span> <span class="topos-hi topos-small-caps">x</span><span class="topos-added">is</span>"#
    );
    // No prefix, and only footnotes; words unwrapped
    let opts = HtmlOptions {
        class_prefix: "".into(),
        filter: Filter::all()
            .drop(Kind::Title)
            .unwrap(Kind::Word)
            .note_type("crossReference", Action::Drop),
    };
    let mut r = HtmlRenderer::with_options(opts);
    let html = r.render(&nodes);
    assert!(
        html.starts_with(r#"LORD<span class="note explanation" data-note-type="explanation">"#),
        "{html}"
    );
    assert_eq!(r.wrap("x"), "<div>x</div>");
    let custom = HtmlRenderer::with_options(HtmlOptions {
        class_prefix: "bible_".into(),
        ..Default::default()
    });
    assert_eq!(custom.wrap("x"), r#"<div class="bible">x</div>"#);
}

#[test]
fn flat_text_maps_back_to_the_tree() {
    let nodes = parse(
        Markup::Osis,
        "See <hi type=\"italic\">John</hi>   3:16\n<note>n</note> now",
    );
    let flat = flatten::flatten(&nodes, &TextOptions::default());
    assert_eq!(flat.text, "See John 3:16 now");
    let (start, end) = flat.source_range(4..13).unwrap();
    // `John` is the text node in <hi>, `3:16` is in the top-level text after it
    assert_eq!(
        (start.path.to_string(), start.offset),
        ("/1/0".to_string(), 0)
    );
    assert_eq!((end.path.to_string(), end.offset), ("/2".to_string(), 7));
    let node = start.path.resolve(&nodes).unwrap();
    assert_eq!(node.as_text(), Some("John"));
    assert_eq!(flat.source(8), None, "collapsed whitespace has no source");
    let hi = flat.spans_at(4..8).next().unwrap();
    assert_eq!(
        (hi.kind, &flat.text[hi.range.clone()]),
        (Kind::Highlight, "John")
    );
}

#[test]
fn usx() {
    let src = r#"<?xml version="1.0" encoding="utf-8"?>
<usx version="3.0">
  <book code="JHN" style="id">World English Bible</book>
  <chapter number="3" style="c" sid="JHN 3"/>
  <para style="s1">Jesus and Nicodemus</para>
  <para style="p">
    <verse number="16" style="v" sid="JHN 3:16"/><char style="wj">For God so <char style="w" strong="G25">loved</char> the world<note caller="+" style="x"><char style="xo">3:16 </char><char style="xt">Rom 5:8</char></note>, that he gave his one and only Son</char>.<verse eid="JHN 3:16"/>
  </para>
  <para style="q1">The <char style="nd">Lord</char> is my shepherd</para>
  <chapter eid="JHN 3"/>
</usx>"#;
    let nodes = parse(Markup::Usx, src);
    assert_eq!(find_all(&nodes, Kind::Metadata).len(), 1);
    assert_eq!(find_all(&nodes, Kind::Title)[0].title_type(), Some("s1"));
    assert_eq!(
        find_all(&nodes, Kind::Word)[0].strongs()[0].to_string(),
        "G25"
    );
    assert!(find_all(&nodes, Kind::Note)[0].is_cross_reference());
    let verses = find_all(&nodes, Kind::Verse);
    assert_eq!(
        (verses[0].milestone, verses[1].milestone),
        (Some(Milestone::Start), Some(Milestone::End))
    );
    assert_eq!(
        plain_text(&nodes, &TextOptions::default()),
        "For God so loved the world, that he gave his one and only Son.\nThe Lord is my shepherd"
    );
    // The verse a match is in
    let flat = flatten::flatten(&nodes, &TextOptions::default());
    let at = flat.text.find("world").unwrap();
    let verse = flat.last_before(Kind::Verse, at).unwrap();
    let verse = verse
        .path
        .resolve(&nodes)
        .and_then(Node::as_element)
        .unwrap();
    assert_eq!(verse.attr("sid"), Some("JHN 3:16"));
    let html = HtmlRenderer::new().render(&nodes);
    assert!(
        html.contains(r#"<span class="topos-verse" data-osis-id="JHN 3:16" data-n="16"></span>"#),
        "{html}"
    );
    assert!(!html.contains("World English Bible"));
    // `xt` is a reference that topos can read
    let data = topos_bible::data::BibleData::default();
    let refs = refs::tagged_references(&nodes, &data);
    assert_eq!(
        refs[0].passages[0].to_osis(data.books()).as_deref(),
        Some("Rom.5.8")
    );
}

#[test]
fn plaintext_lines() {
    let nodes = parse(Markup::Plaintext, "one\r\ntwo  words\n\n\nthree");
    assert_eq!(
        plain_text(&nodes, &TextOptions::default()),
        "one\ntwo words\n\nthree"
    );
}
