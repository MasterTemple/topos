//! Strong's numbers and morphology (docs/spec/strongs.md).

use topos_bible_markup::{Element, Kind, Markup, Strongs, StrongsLang, parse, strongs_numbers};

fn parsed(token: &str) -> Option<(StrongsLang, u32, Option<String>)> {
    Strongs::parse(token).map(|s| (s.lang, s.number, s.suffix))
}

#[test]
fn tokens() {
    use StrongsLang::{Greek, Hebrew};
    assert_eq!(
        parsed("strong:H0430a"),
        Some((Hebrew, 430, Some("a".into())))
    );
    assert_eq!(parsed("strong:H07225"), Some((Hebrew, 7225, None)));
    assert_eq!(parsed("x-Strongs:G25"), Some((Greek, 25, None)));
    assert_eq!(parsed("Strongs:G26"), Some((Greek, 26, None)));
    assert_eq!(parsed("G25"), Some((Greek, 25, None)));
    assert_eq!(parsed("g25"), Some((Greek, 25, None)));
    assert_eq!(parsed("h1"), Some((Hebrew, 1, None)));
    assert_eq!(parsed("G0"), Some((Greek, 0, None)));
    assert_eq!(parsed("strong:G12!"), Some((Greek, 12, Some("!".into()))));
    assert_eq!(parsed("H1a2"), Some((Hebrew, 1, Some("a2".into()))));
    for bad in [
        "lemma.TR:foo",
        "strong:Gx",
        "G-1",
        "Ga1",
        "H",
        "",
        "strong:",
        "G12x-y",
        "strong:strong:G1",
        "strong:G99999999999",
        "STRONG:G1",
        "X25",
        "G1 ",
    ] {
        assert_eq!(Strongs::parse(bad), None, "{bad}");
    }
}

#[test]
fn lists_and_keys() {
    let s = Strongs::parse("strong:H0430a").expect("a number");
    assert_eq!(s.dictionary_key(), "00430");
    assert_eq!(s.dictionary_module(), "StrongsHebrew");
    assert_eq!(s.to_string(), "H430a");
    let g = Strongs::parse("G123456").expect("a number");
    assert_eq!(g.dictionary_key(), "123456");
    assert_eq!(g.dictionary_module(), "StrongsGreek");
    assert_eq!(g.to_string(), "G123456");
    assert_eq!(
        Strongs::parse("G0").map(|s| s.dictionary_key()),
        Some("00000".into())
    );
    assert_eq!(
        Strongs::parse_list("strong:G1 x-Strongs:G2 lemma.TR:x").len(),
        2
    );
    assert_eq!(Strongs::parse_list("  G1\tG1\nH2 ").len(), 3);
    assert!(Strongs::parse_list("").is_empty());
}

#[test]
fn order_is_hebrew_then_greek() {
    let mut list: Vec<Strongs> = ["G1", "H2", "H1a", "H1", "G0"]
        .iter()
        .filter_map(|t| Strongs::parse(t))
        .collect();
    list.sort();
    let shown: Vec<String> = list.iter().map(ToString::to_string).collect();
    assert_eq!(shown, ["H1", "H1a", "H2", "G0", "G1"]);
}

#[test]
fn on_elements() {
    let w = Element {
        kind: Kind::Word,
        name: "w".into(),
        attrs: vec![
            ("lemma".into(), "strong:G1 lemma.TR:x".into()),
            ("strong".into(), "G2,G3 H4,,".into()),
            (
                "morph".into(),
                "robinson:V-AAI-3S  strongsMorph:TH8804".into(),
            ),
        ],
        children: vec![],
        milestone: None,
        unclosed: false,
    };
    let numbers: Vec<String> = w.strongs().iter().map(ToString::to_string).collect();
    assert_eq!(numbers, ["G1", "G2", "G3", "H4"]);
    assert_eq!(w.morph(), ["robinson:V-AAI-3S", "strongsMorph:TH8804"]);
    let mut bare = w.clone();
    bare.attrs.clear();
    assert!(bare.strongs().is_empty() && bare.morph().is_empty());
}

#[test]
fn numbers_in_reading_order() {
    let nodes = parse(
        Markup::Osis,
        r#"<w lemma="strong:H1">a</w> <note><w lemma="strong:H2 strong:H2">b</w></note> <hi lemma="strong:H9">not a word</hi>"#,
    );
    let numbers: Vec<String> = strongs_numbers(&nodes)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(numbers, ["H1", "H2", "H2"]);
    let usx = parse(Markup::Usx, r#"<char style="w" strong="G25">loved</char>"#);
    assert_eq!(strongs_numbers(&usx)[0].to_string(), "G25");
}
