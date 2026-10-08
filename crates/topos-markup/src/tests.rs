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

#[test]
fn marks_across_entries_and_elements() {
    let a = parse(Markup::Osis, r#"Read <w lemma="strong:G1">John</w>"#);
    let b = parse(Markup::Osis, r#"3:16 and Rom 8:28 &amp; Rom 8:28."#);
    let mut flat = Flattener::new(TextOptions::default());
    flat.push(&a).push(&b);
    let flat = flat.finish();
    assert_eq!(flat.text, "Read John 3:16 and Rom 8:28 & Rom 8:28.");
    let mut html = HtmlRenderer::new();
    // `John 3:16` runs from the first entry into the second; the second `Rom 8:28` overlaps a
    // wider mark, and overlapping marks are merged
    for range in [5..14, 19..27, 30..38, 28..38] {
        let (start, end) = flat.source_range(range).unwrap();
        html.mark(start, end);
    }
    let out = format!("{}|{}", html.render(&a), html.render(&b));
    assert_eq!(
        out,
        r#"Read <span class="topos-w" data-strong="G1" data-lemma="strong:G1"><mark class="topos-mark" data-mark="0">John</mark></span>|<mark class="topos-mark" data-mark="0">3:16</mark> and <mark class="topos-mark" data-mark="1">Rom 8:28</mark> <mark class="topos-mark" data-mark="3">&amp; Rom 8:28</mark>."#
    );
}

// Expected passage parts are lists of index ranges, often just one
#[allow(clippy::single_range_in_vec_init)]
mod verses {
    use super::*;
    use crate::verses::{passage_ranges, slice, verses};
    use topos_bible::data::BibleData;

    const USX: &str = r#"<usx version="3.0"><book code="PSA" style="id"/>
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

    fn text(nodes: &[Node]) -> String {
        plain_text(
            nodes,
            &TextOptions {
                filter: Filter::all(),
                ..TextOptions::default()
            },
        )
    }

    #[test]
    fn usx_verses_cross_paragraphs_and_keep_headings() {
        let nodes = parse(Markup::Usx, USX);
        let spans = verses(&nodes);
        let numbers: Vec<_> = spans.iter().map(|s| (s.chapter, s.verse, s.last)).collect();
        assert_eq!(numbers, [(1, 1, 1), (1, 2, 3), (1, 4, 4), (2, 1, 1)]);
        // Each chapter's verse count, a bridge counting to its last verse
        assert_eq!(crate::verses::chapter_verses(&spans), [4, 1]);
        assert_eq!(crate::verses::usx_chapter_verses(USX), [4, 1]);
        let verse = |i: usize| slice(&nodes, &spans[i].start, Some(&spans[i].end));
        // The heading after the chapter marker goes with verse 1, the verse keeps both lines
        let first = verse(0);
        assert_eq!(
            text(&first),
            "A heading\nBlessed is the man\nwho does not walk"
        );
        assert_eq!(find_all(&first, Kind::Line).len(), 2);
        // A heading between verses belongs to the verse after it; the book title to none
        assert_eq!(text(&verse(1)), "The way\nBut his delight");
        assert_eq!(text(&verse(2)), "Not so");
        assert_eq!(text(&verse(3)), "Why do the nations rage");
        assert!(!(0..4).any(|i| text(&verse(i)).contains("Psalms")));
    }

    #[test]
    fn passages_pick_verses() {
        let nodes = parse(Markup::Usx, USX);
        let spans = verses(&nodes);
        let data = BibleData::default();
        let ranges = |r: &str| passage_ranges(&spans, &data.parse(r).unwrap());
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
            text(&chapter),
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
        assert_eq!(
            spans
                .iter()
                .map(|s| (s.chapter, s.verse))
                .collect::<Vec<_>>(),
            [(3, 16), (3, 17)]
        );
        assert_eq!(
            text(&slice(&elements, &spans[1].start, Some(&spans[1].end))),
            "For God sent"
        );

        let milestones = parse(
            Markup::Osis,
            r#"<chapter sID="John.3" osisID="John.3"/><verse sID="John.3.16" osisID="John.3.16"/>For God<verse eID="John.3.16"/><verse sID="John.3.17" osisID="John.3.17"/>sent<verse eID="John.3.17"/><chapter eID="John.3"/>"#,
        );
        let spans = verses(&milestones);
        assert_eq!(
            spans
                .iter()
                .map(|s| (s.chapter, s.verse))
                .collect::<Vec<_>>(),
            [(3, 16), (3, 17)]
        );
        assert_eq!(
            text(&slice(&milestones, &spans[0].start, Some(&spans[0].end))),
            "For God"
        );
        // No end: to the end of the tree
        assert_eq!(text(&slice(&milestones, &spans[1].start, None)), "sent");
    }
}

/// Verse and chapter labels are written into the text, and their spans cover them
#[test]
fn verse_and_chapter_labels() {
    let usx = r#"<usx><book code="JHN" style="id">x</book><chapter number="3" style="c" sid="JHN 3"/>
<para style="p"><verse number="16" style="v" sid="JHN 3:16"/>For God so <char style="wj">loved</char> the world.<verse eid="JHN 3:16"/><verse number="17-18" style="v" sid="JHN 3:17-18"/>For God<verse eid="JHN 3:17-18"/></para><chapter eid="JHN 3"/></usx>"#;
    let nodes = parse(Markup::Usx, usx);
    let opts = TextOptions {
        verse_label: Some(String::from("[{verse}]")),
        chapter_label: Some(String::from("{book} {chapter}")),
        ..TextOptions::default()
    };
    let flat = flatten::flatten(&nodes, &opts);
    assert_eq!(
        flat.text,
        "JHN 3\n[16] For God so loved the world. [17-18] For God"
    );
    let labels: Vec<(Kind, &str)> = flat
        .spans
        .iter()
        .filter(|s| matches!(s.kind, Kind::Verse | Kind::Chapter) && !s.range.is_empty())
        .map(|s| (s.kind, &flat.text[s.range.clone()]))
        .collect();
    assert_eq!(
        labels,
        [
            (Kind::Chapter, "JHN 3"),
            (Kind::Verse, "[16]"),
            (Kind::Verse, "[17-18]")
        ]
    );
    // Labels aren't from the tree
    let at = flat.text.find("[16]").unwrap();
    assert_eq!(flat.source(at), None);
    assert!(flat.source(at + 5).is_some());

    // A passage cut from the middle of a chapter still knows its chapter (from the verse's id)
    let osis = parse(
        Markup::Osis,
        r#"<verse osisID="John.3.16">For God</verse> <verse osisID="John.3.17">sent</verse>"#,
    );
    let opts = TextOptions {
        verse_label: Some(String::from("{book} {chapter}:{verse}")),
        ..TextOptions::default()
    };
    assert_eq!(plain_text(&osis, &opts), "John 3:16 For God John 3:17 sent");
    // Without labels, verse markers stay empty
    let flat = flatten::flatten(&nodes, &TextOptions::default());
    assert!(flat.text.starts_with("For God"));
}

/// Each pushed entry's range in the text
#[test]
fn entry_ranges() {
    let mut flat = Flattener::new(TextOptions::default());
    flat.push(&parse(Markup::Osis, "In the beginning "))
        .push(&parse(Markup::Osis, "<note>dropped</note>"))
        .push(&parse(Markup::Osis, " was <q who=\"Jesus\">the Word"))
        .push(&parse(Markup::Osis, "</q>"));
    let flat = flat.finish();
    let entries: Vec<&str> = flat.entries.iter().map(|r| &flat.text[r.clone()]).collect();
    assert_eq!(entries, ["In the beginning", "", "was the Word", ""]);
    assert_eq!(flat.entries[1], 17..17);
}

/// Templates around elements: notes, headings, and words of Jesus (also across entries)
#[test]
fn element_templates() {
    let usx = r#"<usx><para style="s1">The Word</para><para style="p">Jesus said, <char style="wj"> I am the light</char>, and<note style="f"><char style="ft">Or bread</char></note> went.</para></usx>"#;
    let nodes = parse(Markup::Usx, usx);
    let opts = TextOptions {
        filter: Filter::all().drop(Kind::Metadata),
        templates: [
            (Kind::Title, "## {text}"),
            (Kind::WordsOfJesus, "“{text}”"),
            (Kind::Note, "^({text})"),
        ]
        .into_iter()
        .map(|(kind, template)| (kind, template.to_string()))
        .collect(),
        ..TextOptions::default()
    };
    let flat = flatten::flatten(&nodes, &opts);
    assert_eq!(
        flat.text,
        "## The Word\nJesus said, “I am the light”, and ^(Or bread) went."
    );
    // Spans cover their template's text
    let red = flat
        .spans
        .iter()
        .find(|s| s.kind == Kind::WordsOfJesus)
        .unwrap();
    assert_eq!(&flat.text[red.range.clone()], "“I am the light”");
    // Notes keep their brackets without a template
    let plain = TextOptions {
        filter: Filter::all().drop(Kind::Metadata),
        ..TextOptions::default()
    };
    assert!(plain_text(&nodes, &plain).contains("and [Or bread] went"));

    // Words of Jesus between milestones, across entries: one opening and one closing quote
    let mut flat = Flattener::new(opts.clone());
    flat.push(&parse(
        Markup::Osis,
        r#"He said, <q who="Jesus" sID="q1"/>Follow"#,
    ))
    .push(&parse(Markup::Osis, r#"me.<q eID="q1"/> And they did."#));
    assert_eq!(flat.finish().text, "He said, “Follow me.” And they did.");
}
