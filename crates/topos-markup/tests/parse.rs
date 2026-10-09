//! Parsing each markup into the tree, classification, and the element accessors
//! (docs/spec/parsing.md, docs/spec/tree.md).

mod common;

use common::{elements, first_element, tree};
use topos_bible_markup::{
    Element, Kind, Markup, Milestone, Node, find_all, parse, strongs_numbers, walk,
};

#[test]
fn markup_names() {
    for (name, markup) in [
        ("osis", Markup::Osis),
        ("OSIS", Markup::Osis),
        ("ThML", Markup::Thml),
        ("gbf", Markup::Gbf),
        ("Tei", Markup::Tei),
        ("usx", Markup::Usx),
        ("plaintext", Markup::Plaintext),
        ("Plain", Markup::Plaintext),
    ] {
        assert_eq!(Markup::from_name(name), Some(markup), "{name}");
    }
    for name in ["", "usfm", " osis", "text", "html"] {
        assert_eq!(Markup::from_name(name), None, "{name}");
    }
}

#[test]
fn osis_words_notes_and_milestones() {
    let input = r#"<div sID="gen4" type="x-p"/>And <w lemma="strong:H0430">God</w> <w lemma="strong:H0559" morph="x:1">said</w>, <q who="Jesus" sID="q1"/>Let there be light<note type="crossReference" n="a"><reference osisRef="John.1.5">John 1:5</reference></note>: &amp; <transChange type="added">it was</transChange> <divineName>Lord</divineName>"#;
    let nodes = parse(Markup::Osis, input);

    let first = first_element(&nodes);
    assert_eq!(first.kind, Kind::Paragraph);
    assert_eq!(first.milestone, Some(Milestone::Start));

    let words = find_all(&nodes, Kind::Word);
    assert_eq!(words.len(), 2);
    assert_eq!(words[0].strongs()[0].to_string(), "H430");
    assert_eq!(words[1].morph(), ["x:1"]);

    let red = find_all(&nodes, Kind::WordsOfJesus);
    assert_eq!(red.len(), 1);
    assert_eq!(red[0].milestone, Some(Milestone::Start));

    let notes = find_all(&nodes, Kind::Note);
    assert!(notes[0].is_cross_reference());
    assert_eq!(notes[0].attr("n"), Some("a"));
    let refs = find_all(&nodes, Kind::Reference);
    assert_eq!(refs[0].reference_target(), Some("John.1.5"));

    assert_eq!(find_all(&nodes, Kind::Added)[0].text(), "it was");
    assert_eq!(find_all(&nodes, Kind::DivineName)[0].text(), "Lord");
    // `&amp;` in text is decoded
    assert!(nodes.iter().any(|n| n.as_text() == Some(": & ")));
}

#[test]
fn fragments_make_orphan_ends_and_unclosed_elements() {
    let nodes = parse(
        Markup::Osis,
        r#"end of quote.</q> New <q who="Jesus">start"#,
    );
    // The `</q>` has no start here: an end milestone, and (no `who`) a plain quote
    let quotes = find_all(&nodes, Kind::Quote);
    assert_eq!(quotes.len(), 1);
    assert_eq!(quotes[0].milestone, Some(Milestone::End));
    assert!(quotes[0].children.is_empty() && quotes[0].attrs.is_empty());
    // The `<q who="Jesus">` is never closed
    let red = find_all(&nodes, Kind::WordsOfJesus);
    assert!(red[0].unclosed);
    assert_eq!(red[0].text(), "start");
}

#[test]
fn end_tags_close_the_nearest_match_ignoring_case() {
    assert_eq!(
        tree(
            Markup::Osis,
            r#"<hi type="bold"><w lemma="strong:G1">x</hi></w>y"#
        ),
        "Highlight hi type=\"bold\"\n  Word w @unclosed lemma=\"strong:G1\"\n    text \"x\"\nWord w @end\ntext \"y\"\n"
    );
    assert_eq!(
        tree(Markup::Osis, r#"<hi type="bold">x</HI>y"#),
        "Highlight hi type=\"bold\"\n  text \"x\"\ntext \"y\"\n"
    );
    // Everything open at the end is closed and marked
    let nodes = parse(Markup::Osis, "<p>a<note>b<hi>c");
    let mut unclosed = Vec::new();
    walk(&nodes, &mut |e: &Element| {
        unclosed.push((e.name.clone(), e.unclosed))
    });
    assert_eq!(
        unclosed,
        [
            ("p".into(), true),
            ("note".into(), true),
            ("hi".into(), true)
        ]
    );
}

#[test]
fn adjacent_text_is_one_node() {
    let nodes = parse(
        Markup::Osis,
        "text<!-- a comment -->, more &amp; <![CDATA[<raw>]]> end",
    );
    assert_eq!(nodes, vec![Node::Text("text, more & <raw> end".into())]);
}

#[test]
fn milestone_attributes_in_any_case() {
    for markup in [Markup::Osis, Markup::Thml, Markup::Tei, Markup::Usx] {
        let nodes = parse(
            markup,
            r#"<x sid="a"/><x EID="b"/><x sID="c" eID="c"/><x/>"#,
        );
        let marks: Vec<Option<Milestone>> = nodes
            .iter()
            .filter_map(Node::as_element)
            .map(|e| e.milestone)
            .collect();
        assert_eq!(
            marks,
            [
                Some(Milestone::Start),
                Some(Milestone::End),
                Some(Milestone::Start),
                None
            ],
            "{markup:?}"
        );
    }
    // GBF has no such rule
    let nodes = parse(Markup::Gbf, "<CM>");
    assert_eq!(first_element(&nodes).milestone, None);
}

#[test]
fn osis_kinds() {
    let cases = [
        (r#"<w>x</w>"#, Kind::Word),
        (r#"<note>x</note>"#, Kind::Note),
        (r#"<reference>x</reference>"#, Kind::Reference),
        (r#"<title>x</title>"#, Kind::Title),
        (r#"<q who="JESUS">x</q>"#, Kind::WordsOfJesus),
        (r#"<q who="Paul">x</q>"#, Kind::Quote),
        (r#"<transChange>x</transChange>"#, Kind::Added),
        (r#"<divineName>x</divineName>"#, Kind::DivineName),
        (r#"<hi>x</hi>"#, Kind::Highlight),
        (r#"<foreign>x</foreign>"#, Kind::Foreign),
        (r#"<p>x</p>"#, Kind::Paragraph),
        (r#"<div type="paragraph">x</div>"#, Kind::Paragraph),
        (r#"<div type="x-p">x</div>"#, Kind::Paragraph),
        (r#"<div type="Paragraph">x</div>"#, Kind::Division),
        (r#"<div>x</div>"#, Kind::Division),
        (r#"<lg>x</lg>"#, Kind::LineGroup),
        (r#"<l>x</l>"#, Kind::Line),
        (r#"<lb/>"#, Kind::LineBreak),
        (r#"<chapter/>"#, Kind::Chapter),
        (r#"<verse/>"#, Kind::Verse),
        (r#"<milestone/>"#, Kind::Milestone),
        (r#"<figure/>"#, Kind::Image),
        (r#"<seg>x</seg>"#, Kind::Other),
        (r#"<W>x</W>"#, Kind::Other),
        (r#"<osis:w>x</osis:w>"#, Kind::Other),
    ];
    for (input, kind) in cases {
        assert_eq!(
            first_element(&parse(Markup::Osis, input)).kind,
            kind,
            "{input}"
        );
    }
    // OSIS adds no default style
    assert_eq!(
        first_element(&parse(Markup::Osis, "<hi>x</hi>")).style(),
        None
    );
}

#[test]
fn thml_kinds_and_styles() {
    let cases = [
        ("<scripRef>x</scripRef>", Kind::Reference, None),
        ("<SCRIPREF>x</SCRIPREF>", Kind::Reference, None),
        ("<a>x</a>", Kind::Reference, None),
        ("<note>x</note>", Kind::Note, None),
        ("<i>x</i>", Kind::Highlight, Some("italic")),
        ("<EM>x</EM>", Kind::Highlight, Some("italic")),
        ("<cite>x</cite>", Kind::Highlight, Some("italic")),
        ("<b>x</b>", Kind::Highlight, Some("bold")),
        ("<strong>x</strong>", Kind::Highlight, Some("bold")),
        ("<u>x</u>", Kind::Highlight, Some("underline")),
        ("<sup>x</sup>", Kind::Highlight, Some("super")),
        ("<sub>x</sub>", Kind::Highlight, Some("sub")),
        ("<font>x</font>", Kind::Highlight, None),
        ("<span>x</span>", Kind::Highlight, None),
        ("<br>", Kind::LineBreak, None),
        ("<p>x</p>", Kind::Paragraph, None),
        ("<h3>x</h3>", Kind::Title, None),
        (r#"<div class="a sechead">x</div>"#, Kind::Title, None),
        (r#"<div class="SECHEAD">x</div>"#, Kind::Division, None),
        ("<added>x</added>", Kind::Added, None),
        ("<foreign>x</foreign>", Kind::Foreign, None),
        ("<q>x</q>", Kind::Quote, None),
        ("<blockquote>x</blockquote>", Kind::Quote, None),
        ("<img>", Kind::Image, None),
        ("<w>x</w>", Kind::Word, None),
        ("<hr>", Kind::Other, None),
    ];
    for (input, kind, style) in cases {
        let nodes = parse(Markup::Thml, input);
        let e = first_element(&nodes);
        assert_eq!((e.kind, e.style()), (kind, style), "{input}");
    }
    // An existing `type` is kept
    let nodes = parse(Markup::Thml, r#"<i type="bold">x</i>"#);
    assert_eq!(first_element(&nodes).style(), Some("bold"));
    assert_eq!(first_element(&nodes).attrs.len(), 1);
}

#[test]
fn thml_void_elements() {
    let nodes = parse(Markup::Thml, "a<br>b<BR>c<br></br>d<img src=\"x\">e</img>f");
    assert_eq!(find_all(&nodes, Kind::LineBreak).len(), 3);
    assert_eq!(find_all(&nodes, Kind::Image).len(), 1);
    assert!(
        nodes
            .iter()
            .all(|n| n.as_element().is_none_or(|e| e.children.is_empty()))
    );
    let texts: Vec<&str> = nodes.iter().filter_map(Node::as_text).collect();
    assert_eq!(texts, ["a", "b", "c", "d", "ef"]);
}

#[test]
fn thml_sync_tags_become_words() {
    let input = r#"ιουδας<sync type="Strongs" value="G2455" /><sync type="morph" class="Robinson" value="N-NSM" /> δε<sync type="Strongs" value="G1161" /> εγεννησεν<sync type="Strongs" value="G1080" /><sync type="Strongs" value="G5656" /><br>τον"#;
    let nodes = parse(Markup::Thml, input);
    let words = find_all(&nodes, Kind::Word);
    assert_eq!(words.len(), 3);
    assert_eq!(words[0].text(), "ιουδας");
    assert_eq!(words[0].attr("lemma"), Some("strong:G2455"));
    assert_eq!(words[0].morph(), ["robinson:N-NSM"]);
    assert_eq!(words[1].text(), "δε");
    let numbers: Vec<u32> = words[2].strongs().iter().map(|s| s.number).collect();
    assert_eq!(numbers, [1080, 5656]);
    assert_eq!(words[2].attr("lemma"), Some("strong:G1080 strong:G5656"));
    assert_eq!(find_all(&nodes, Kind::LineBreak).len(), 1);
}

#[test]
fn thml_sync_edge_cases() {
    // After whitespace: an empty word
    assert_eq!(
        tree(
            Markup::Thml,
            r#"word <sync type="Strongs" value="G1"/>next"#
        ),
        "text \"word \"\nWord w lemma=\"strong:G1\"\ntext \"next\"\n"
    );
    // After an explicit word: that word
    let nodes = parse(
        Markup::Thml,
        r#"<w>explicit</w><sync type="strongs" value="G2"/>"#,
    );
    assert_eq!(nodes.len(), 1);
    assert_eq!(first_element(&nodes).attr("lemma"), Some("strong:G2"));
    // After another element: a new empty word
    let nodes = parse(
        Markup::Thml,
        r#"<b>bold</b><sync type="Strongs" value="G1"/>"#,
    );
    assert_eq!(find_all(&nodes, Kind::Word)[0].children, vec![]);
    // Morph without a class; other types and missing values are plain elements
    let nodes = parse(Markup::Thml, r#"λογος<sync type="morph" value="N-NSM"/>"#);
    assert_eq!(first_element(&nodes).morph(), ["N-NSM"]);
    let nodes = parse(
        Markup::Thml,
        r#"a<sync type="lemma" value="x"/> b<sync type="Strongs"/>"#,
    );
    assert!(find_all(&nodes, Kind::Word).is_empty());
    assert_eq!(find_all(&nodes, Kind::Other).len(), 2);
}

#[test]
fn gbf_tokens() {
    let input = "In the beginning<WH7225> God<WH430><WTN-M> said,<FR>Let there<FI>be<Fi> light<Fr><RF>a note<Rf><CM><<Geschrieben von Rom.>";
    let nodes = parse(Markup::Gbf, input);
    let words = find_all(&nodes, Kind::Word);
    assert_eq!(words.len(), 2);
    assert_eq!(words[0].text(), "beginning");
    assert_eq!(words[0].attr("lemma"), Some("strong:H7225"));
    assert_eq!(words[1].text(), "God");
    assert_eq!(words[1].morph(), ["N-M"]);
    let red = find_all(&nodes, Kind::WordsOfJesus);
    assert_eq!(red[0].name, "FR");
    assert_eq!(red[0].text(), "Let therebe light");
    let italic = find_all(&nodes, Kind::Highlight);
    assert_eq!(italic[0].style(), Some("italic"));
    assert_eq!(find_all(&nodes, Kind::Note)[0].text(), "a note");
    assert_eq!(find_all(&nodes, Kind::Paragraph).len(), 1);
    // Not a token: kept as text
    assert_eq!(
        nodes.last().and_then(Node::as_text),
        Some("<<Geschrieben von Rom.>")
    );
}

#[test]
fn gbf_codes() {
    let kinds = [
        ("FI", Kind::Highlight, Some("italic")),
        ("FB", Kind::Highlight, Some("bold")),
        ("FU", Kind::Highlight, Some("underline")),
        ("FS", Kind::Highlight, Some("super")),
        ("FV", Kind::Highlight, Some("sub")),
        ("FC", Kind::Highlight, Some("small-caps")),
        ("FN", Kind::Highlight, None),
        ("FR", Kind::WordsOfJesus, None),
        ("FO", Kind::Quote, None),
        ("RF", Kind::Note, None),
        ("RX", Kind::Reference, None),
        ("TS", Kind::Title, None),
        ("TT", Kind::Title, None),
        ("TH", Kind::Title, None),
        ("PP", Kind::LineGroup, None),
        ("RB", Kind::Other, None),
    ];
    for (code, kind, style) in kinds {
        let close = format!("{}{}", &code[..1], code[1..].to_ascii_lowercase());
        let nodes = parse(Markup::Gbf, &format!("<{code}>x<{close}>"));
        assert_eq!(nodes.len(), 1, "{code}");
        let e = first_element(&nodes);
        assert_eq!((e.name.as_str(), e.kind, e.style()), (code, kind, style));
        assert_eq!(e.text(), "x");
    }
    let nodes = parse(Markup::Gbf, "<CM><CL><PI2><RXGen.1.1>see<Rx><WGabc>");
    let shape: Vec<(String, Kind, Option<&str>)> = nodes
        .iter()
        .filter_map(Node::as_element)
        .map(|e| (e.name.clone(), e.kind, e.attr("value")))
        .collect();
    assert_eq!(
        shape,
        [
            ("CM".into(), Kind::Paragraph, None),
            ("CL".into(), Kind::LineBreak, None),
            ("PI".into(), Kind::Other, Some("2")),
            ("RX".into(), Kind::Reference, Some("Gen.1.1")),
            ("WG".into(), Kind::Other, Some("abc")),
        ]
    );
}

#[test]
fn gbf_text_and_non_tokens() {
    assert_eq!(
        tree(Markup::Gbf, "<ab> <A1> <A> <a> <AB c> x"),
        "text \"<ab> <A1> <A> <a> <AB c> x\"\n"
    );
    // A `<` that does not start a token is text, and the scan resumes after it
    assert_eq!(tree(Markup::Gbf, "<<Ge>"), "text \"<\"\nOther Ge\n");
    assert_eq!(
        tree(Markup::Gbf, "x <FI and more"),
        "text \"x <FI and more\"\n"
    );
    assert_eq!(
        tree(Markup::Gbf, "&amp;&eacute; &bogus;"),
        "text \"&é &bogus;\"\n"
    );
    // Strong's after whitespace: an empty word; consecutive numbers share a word
    assert_eq!(
        tree(Markup::Gbf, "two words<WH1><WH2a> more"),
        "text \"two \"\nWord w lemma=\"strong:H1 strong:H2a\"\n  text \"words\"\ntext \" more\"\n"
    );
    assert_eq!(
        tree(Markup::Gbf, "<WG25>start"),
        "Word w lemma=\"strong:G25\"\ntext \"start\"\n"
    );
    // `<WT>` alone is not morphology
    assert_eq!(tree(Markup::Gbf, "a<WT>"), "text \"a\"\nOther WT\n");
}

#[test]
fn tei_dictionary_entry() {
    let input = r#"<entryFree n="00003"> <orth>Ἀβαδδών</orth> <pron rend="italic">ab-ad-dohn'</pron><lb/> <def>of Hebrew origin (<ref target="StrongsHebrew:011">H11</ref>); a destroying angel</def> </entryFree>"#;
    let nodes = parse(Markup::Tei, input);
    assert_eq!(first_element(&nodes).kind, Kind::Entry);
    assert_eq!(
        find_all(&nodes, Kind::Reference)[0].reference_target(),
        Some("StrongsHebrew:011")
    );
    assert_eq!(find_all(&nodes, Kind::LineBreak).len(), 1);
    // `pron` is not a highlight; `orth` and `def` are other
    assert!(find_all(&nodes, Kind::Highlight).is_empty());
}

#[test]
fn tei_kinds_and_rend() {
    let cases = [
        ("<entryFree>x</entryFree>", Kind::Entry, None),
        ("<entry>x</entry>", Kind::Entry, None),
        ("<superentry>x</superentry>", Kind::Entry, None),
        ("<superEntry>x</superEntry>", Kind::Other, None),
        ("<ref>x</ref>", Kind::Reference, None),
        ("<xr>x</xr>", Kind::Reference, None),
        (r#"<hi rend="bold">x</hi>"#, Kind::Highlight, Some("bold")),
        (
            r#"<hi rend="bold" type="x">x</hi>"#,
            Kind::Highlight,
            Some("x"),
        ),
        ("<emph>x</emph>", Kind::Highlight, Some("italic")),
        (
            r#"<emph rend="bold">x</emph>"#,
            Kind::Highlight,
            Some("italic"),
        ),
        ("<title>x</title>", Kind::Highlight, Some("italic")),
        ("<lb/>", Kind::LineBreak, None),
        ("<p>x</p>", Kind::Paragraph, None),
        ("<note>x</note>", Kind::Note, None),
        ("<foreign>x</foreign>", Kind::Foreign, None),
        ("<q>x</q>", Kind::Quote, None),
        ("<quote>x</quote>", Kind::Quote, None),
        ("<w>x</w>", Kind::Word, None),
        ("<figure/>", Kind::Image, None),
        ("<graphic/>", Kind::Image, None),
        ("<orth>x</orth>", Kind::Other, None),
        ("<Ref>x</Ref>", Kind::Other, None),
    ];
    for (input, kind, style) in cases {
        let nodes = parse(Markup::Tei, input);
        let e = first_element(&nodes);
        assert_eq!((e.kind, e.attr("type")), (kind, style), "{input}");
    }
}

#[test]
fn usx_book() {
    let input = r#"<?xml version="1.0" encoding="utf-8"?>
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
    let nodes = parse(Markup::Usx, input);
    assert_eq!(find_all(&nodes, Kind::Metadata).len(), 1);
    assert_eq!(find_all(&nodes, Kind::Title)[0].title_type(), Some("s1"));
    assert_eq!(
        find_all(&nodes, Kind::Word)[0].strongs()[0].to_string(),
        "G25"
    );
    assert!(find_all(&nodes, Kind::Note)[0].is_cross_reference());
    assert_eq!(find_all(&nodes, Kind::Line).len(), 1);
    assert_eq!(find_all(&nodes, Kind::DivineName)[0].text(), "Lord");
    let verses = find_all(&nodes, Kind::Verse);
    assert_eq!(
        (verses[0].milestone, verses[1].milestone),
        (Some(Milestone::Start), Some(Milestone::End))
    );
    let chapters = find_all(&nodes, Kind::Chapter);
    assert_eq!(
        (chapters[0].milestone, chapters[1].milestone),
        (Some(Milestone::Start), Some(Milestone::End))
    );
}

#[test]
fn usx_styles() {
    let paras = [
        ("p", Kind::Paragraph, None),
        ("q2", Kind::Line, None),
        ("qr", Kind::Line, None),
        ("qm1", Kind::Line, None),
        ("b", Kind::LineBreak, None),
        ("s2", Kind::Title, Some("s2")),
        ("ms1", Kind::Title, Some("ms1")),
        ("mt", Kind::Title, Some("mt")),
        ("d", Kind::Title, Some("d")),
        ("r", Kind::Title, Some("r")),
        ("cl", Kind::Title, Some("cl")),
        ("toc1", Kind::Metadata, None),
        ("h", Kind::Metadata, None),
        ("usfm", Kind::Metadata, None),
        ("li1", Kind::Paragraph, None),
        ("", Kind::Paragraph, None),
    ];
    for (style, kind, ty) in paras {
        let nodes = parse(Markup::Usx, &format!(r#"<para style="{style}">x</para>"#));
        let e = first_element(&nodes);
        assert_eq!((e.kind, e.attr("type")), (kind, ty), "para {style}");
    }
    let chars = [
        ("wj", Kind::WordsOfJesus, None),
        ("add", Kind::Added, None),
        ("nd", Kind::DivineName, None),
        ("w", Kind::Word, None),
        ("xt", Kind::Reference, None),
        ("qt", Kind::Quote, None),
        ("tl", Kind::Foreign, None),
        ("it", Kind::Highlight, Some("italic")),
        ("em", Kind::Highlight, Some("italic")),
        ("bk", Kind::Highlight, Some("italic")),
        ("bd", Kind::Highlight, Some("bold")),
        ("bdit", Kind::Highlight, Some("bold-italic")),
        ("sc", Kind::Highlight, Some("small-caps")),
        ("sup", Kind::Highlight, Some("super")),
        ("no", Kind::Highlight, Some("normal")),
        ("ft", Kind::Other, None),
        ("wj1", Kind::WordsOfJesus, None),
        ("WJ", Kind::Other, None),
    ];
    for (style, kind, ty) in chars {
        let nodes = parse(Markup::Usx, &format!(r#"<char style="{style}">x</char>"#));
        let e = first_element(&nodes);
        assert_eq!((e.kind, e.attr("type")), (kind, ty), "char {style}");
    }
    let notes = [
        ("x", "crossReference"),
        ("ex", "crossReference"),
        ("ef", "study"),
        ("f", "footnote"),
        ("fe", "footnote"),
        ("zz", "footnote"),
    ];
    for (style, ty) in notes {
        let nodes = parse(Markup::Usx, &format!(r#"<note style="{style}">x</note>"#));
        assert_eq!(first_element(&nodes).note_type(), Some(ty), "note {style}");
    }
    let nodes = parse(Markup::Usx, r#"<note style="x" type="mine">x</note>"#);
    assert_eq!(first_element(&nodes).note_type(), Some("mine"));
    for (name, kind) in [
        ("book", Kind::Metadata),
        ("ref", Kind::Reference),
        ("figure", Kind::Image),
        ("optbreak", Kind::LineBreak),
        ("table", Kind::Division),
        ("sidebar", Kind::Division),
        ("periph", Kind::Division),
        ("row", Kind::Paragraph),
        ("cell", Kind::Other),
        ("usx", Kind::Other),
    ] {
        let nodes = parse(Markup::Usx, &format!("<{name}>x</{name}>"));
        assert_eq!(first_element(&nodes).kind, kind, "{name}");
    }
}

#[test]
fn plain_text_lines() {
    let nodes = parse(Markup::Plaintext, "one\r\ntwo  words\n\n\nthree");
    let shape: Vec<String> = nodes
        .iter()
        .map(|n| match n {
            Node::Text(t) => t.clone(),
            Node::Element(e) => format!("<{}:{:?}>", e.name, e.kind),
        })
        .collect();
    assert_eq!(
        shape,
        [
            "one",
            "<br:LineBreak>",
            "two  words",
            "<br:LineBreak>",
            "<br:LineBreak>",
            "<br:LineBreak>",
            "three"
        ]
    );
    assert!(parse(Markup::Plaintext, "").is_empty());
    assert_eq!(
        parse(Markup::Plaintext, "<b>&amp;</b>"),
        vec![Node::Text("<b>&amp;</b>".into())]
    );
}

#[test]
fn unreadable_xml_is_kept_as_text() {
    assert_eq!(
        tree(Markup::Osis, "x<!-- unclosed <hi>y</hi>"),
        "text \"x<!-- unclosed <hi>y</hi>\"\n"
    );
    assert_eq!(
        tree(Markup::Osis, "<p>q</p>R&D <hi>x</hi>"),
        "Paragraph p\n  text \"q\"\ntext \"R&D <hi>x</hi>\"\n"
    );
    assert_eq!(tree(Markup::Osis, "text <hi"), "text \"text <hi\"\n");
    assert_eq!(
        tree(Markup::Osis, "a &bogus; <hi/>"),
        "text \"a &bogus; \"\nHighlight hi\n"
    );
    // Entities in what is kept are still decoded
    assert_eq!(
        tree(Markup::Osis, "<p/>a &amp b &lt; c"),
        "Paragraph p\ntext \"a &amp b < c\"\n"
    );
}

#[test]
fn xml_lexical_corners() {
    assert_eq!(
        tree(Markup::Osis, "\u{feff}<w>x</w>"),
        "Word w\n  text \"x\"\n"
    );
    assert_eq!(
        tree(Markup::Osis, "<?xml version=\"1.0\"?><!DOCTYPE osis>z"),
        "text \"z\"\n"
    );
    assert_eq!(
        tree(Markup::Osis, "a<![CDATA[&amp;]]>"),
        "text \"a&amp;\"\n"
    );
    assert_eq!(
        tree(
            Markup::Osis,
            r#"<w lemma=strong:G1 morph="ok" x b="1"c="2" d='q"'>x</w>"#
        ),
        "Word w morph=\"ok\" b=\"1\" c=\"2\" d=\"q\\\"\"\n  text \"x\"\n"
    );
    assert_eq!(
        tree(Markup::Osis, r#"<a b="x>y" b="z&amp;">t</a>"#),
        "Other a b=\"x>y\" b=\"z&\"\n  text \"t\"\n"
    );
    assert_eq!(
        tree(Markup::Osis, "x</>y"),
        "text \"x\"\nOther  @end\ntext \"y\"\n"
    );
    assert_eq!(
        tree(Markup::Osis, "<lb/ >x"),
        "Other lb/ @unclosed\n  text \"x\"\n"
    );
}

#[test]
fn element_accessors() {
    let e = Element {
        kind: Kind::Note,
        name: "note".into(),
        attrs: vec![
            ("TYPE".into(), "CrossReference".into()),
            ("type".into(), "crossReference".into()),
            ("rend".into(), "bold".into()),
            ("Href".into(), "h".into()),
            ("loc".into(), "l".into()),
        ],
        children: vec![
            Node::Text("a ".into()),
            Node::Element(Element {
                kind: Kind::Word,
                name: "w".into(),
                attrs: vec![("lemma".into(), "strong:G1".into())],
                children: vec![Node::Text("b".into())],
                milestone: None,
                unclosed: false,
            }),
            Node::Text(" c".into()),
        ],
        milestone: None,
        unclosed: false,
    };
    // Exact match first, then any case
    assert_eq!(e.attr("type"), Some("crossReference"));
    assert_eq!(e.attr("Type"), Some("CrossReference"));
    assert_eq!(e.attr("href"), Some("h"));
    assert_eq!(e.attr("missing"), None);
    assert!(e.is_cross_reference());
    assert_eq!(e.note_type(), Some("crossReference"));
    assert_eq!(e.title_type(), Some("crossReference"));
    assert_eq!(e.style(), Some("crossReference"));
    // `loc` comes before `href`
    assert_eq!(e.reference_target(), Some("l"));
    assert_eq!(e.text(), "a b c");
    let names: Vec<&str> = e.descendants().iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["note", "w"]);
    assert_eq!(strongs_numbers(&e.children).len(), 1);
    // Not a note: never a cross reference
    let mut other = e.clone();
    other.kind = Kind::Other;
    assert!(!other.is_cross_reference());
    // `style` falls back to `rend`
    let hi = Element {
        kind: Kind::Highlight,
        name: "hi".into(),
        attrs: vec![("rend".into(), "bold".into())],
        children: vec![],
        milestone: None,
        unclosed: false,
    };
    assert_eq!(hi.style(), Some("bold"));
}

#[test]
fn walk_and_find_are_preorder() {
    let nodes = parse(
        Markup::Osis,
        r#"<p><w lemma="strong:G1">a</w><note><w lemma="strong:G2">b</w></note></p><w lemma="strong:G3"><w lemma="strong:G4">c</w></w>"#,
    );
    let mut names = Vec::new();
    walk(&nodes, &mut |e: &Element| names.push(e.name.clone()));
    assert_eq!(names, ["p", "w", "note", "w", "w", "w"]);
    let lemmas: Vec<&str> = find_all(&nodes, Kind::Word)
        .iter()
        .filter_map(|w| w.attr("lemma"))
        .collect();
    assert_eq!(lemmas, ["strong:G1", "strong:G2", "strong:G3", "strong:G4"]);
    let numbers: Vec<String> = strongs_numbers(&nodes)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(numbers, ["G1", "G2", "G3", "G4"]);
}

#[test]
fn class_names() {
    let all = [
        (Kind::Word, "w"),
        (Kind::Note, "note"),
        (Kind::Reference, "ref"),
        (Kind::Title, "title"),
        (Kind::WordsOfJesus, "woj"),
        (Kind::Quote, "quote"),
        (Kind::Added, "added"),
        (Kind::DivineName, "divine-name"),
        (Kind::Highlight, "hi"),
        (Kind::Foreign, "foreign"),
        (Kind::Paragraph, "p"),
        (Kind::LineGroup, "lg"),
        (Kind::Line, "l"),
        (Kind::LineBreak, "lb"),
        (Kind::Division, "div"),
        (Kind::Chapter, "chapter"),
        (Kind::Verse, "verse"),
        (Kind::Milestone, "milestone"),
        (Kind::Image, "image"),
        (Kind::Entry, "entry"),
        (Kind::Metadata, "metadata"),
        (Kind::Other, "other"),
    ];
    for (kind, name) in all {
        assert_eq!(kind.class_name(), name);
    }
    // Declaration order
    let mut sorted: Vec<Kind> = all.iter().map(|(k, _)| *k).rev().collect();
    sorted.sort();
    assert_eq!(sorted, all.iter().map(|(k, _)| *k).collect::<Vec<_>>());
}

#[test]
fn node_helpers() {
    let text = Node::Text("x".into());
    assert_eq!(text.as_text(), Some("x"));
    assert!(text.as_element().is_none());
    let nodes = parse(Markup::Osis, "<w>x</w>");
    assert_eq!(nodes[0].as_element().map(|e| e.kind), Some(Kind::Word));
    assert_eq!(nodes[0].as_text(), None);
    let _ = elements(Markup::Osis, "<w>x</w>", Kind::Word);
}
