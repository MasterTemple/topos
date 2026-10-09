//! References the markup tags (docs/spec/references.md).

use topos_bible::data::BibleData;
use topos_bible_markup::{
    Kind, Markup, find_all, parse,
    refs::{passages, tagged_references},
};

fn osis_list(markup: Markup, input: &str) -> Vec<Vec<String>> {
    let data = BibleData::default();
    tagged_references(&parse(markup, input), &data)
        .iter()
        .map(|r| {
            r.passages
                .iter()
                .map(|p| p.to_osis(data.books()).unwrap_or_default())
                .collect()
        })
        .collect()
}

#[test]
fn osis_targets_with_works_and_several_books() {
    let data = BibleData::default();
    let nodes = parse(
        Markup::Osis,
        r#"See <reference osisRef="Bible:John.3.16-John.3.18 Rom.8.28">these</reference>."#,
    );
    let refs = tagged_references(&nodes, &data);
    assert_eq!(refs.len(), 1);
    assert_eq!(
        refs[0].target.as_deref(),
        Some("Bible:John.3.16-John.3.18 Rom.8.28")
    );
    assert_eq!(refs[0].path.to_string(), "/1");
    let osis: Vec<String> = refs[0]
        .passages
        .iter()
        .filter_map(|p| p.to_osis(data.books()))
        .collect();
    assert_eq!(osis, ["John.3.16-John.3.18", "Rom.8.28"]);
}

#[test]
fn fallbacks() {
    assert_eq!(
        osis_list(
            Markup::Osis,
            r#"<reference osisRef="Gen.1.1 Gen.1.3 Exod.2.1">a</reference><reference>Romans 8:28</reference><reference osisRef="not a ref">John 1:1</reference><reference osisRef="JHN 3:16-18">b</reference><reference osisRef="Jn 3:16">c</reference><reference osisRef="">d</reference><reference>nothing</reference>"#
        ),
        [
            vec!["Gen.1.1 Gen.1.3".to_string(), "Exod.2.1".to_string()],
            vec!["Rom.8.28".to_string()],
            vec!["John.1.1".to_string()],
            vec!["John.3.16-John.3.18".to_string()],
            vec!["John.3.16".to_string()],
            vec![],
            vec![],
        ]
    );
}

#[test]
fn usx_references_and_notes() {
    let usx = r#"<usx><para style="p">world<note caller="+" style="x"><char style="xo">3:16 </char><char style="xt">Rom 5:8</char></note> <ref loc="GEN 1:1">Gen 1:1</ref></para></usx>"#;
    assert_eq!(
        osis_list(Markup::Usx, usx),
        [vec!["Rom.5.8".to_string()], vec!["Gen.1.1".to_string()]]
    );
    // `passages` on its own
    let data = BibleData::default();
    let nodes = parse(Markup::Usx, usx);
    let xt = find_all(&nodes, Kind::Reference)[0];
    assert_eq!(passages(xt, &data).len(), 1);
}

#[test]
fn nested_and_untargeted() {
    let data = BibleData::default();
    let nodes = parse(
        Markup::Tei,
        r#"<ref target="StrongsHebrew:011">H11</ref><note><ref target="Gen.1.1"><ref>John 3:16</ref></ref></note>"#,
    );
    let refs = tagged_references(&nodes, &data);
    let paths: Vec<String> = refs.iter().map(|r| r.path.to_string()).collect();
    assert_eq!(paths, ["/0", "/1/0", "/1/0/0"]);
    assert!(refs[0].passages.is_empty());
    assert_eq!(refs[2].target, None);
}
