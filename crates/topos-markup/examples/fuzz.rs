//! Throws random tag soup at every parser and renderer; any panic is a bug.
//!
//!     cargo run --release -p topos-bible-markup --example fuzz
use topos_bible::data::BibleData;
use topos_bible_markup::{
    Filter, Flattener, HtmlRenderer, Markup, TextOptions, parse, refs::tagged_references,
};

fn main() {
    let atoms = [
        "<",
        ">",
        "/",
        "</q>",
        "<q who=\"Jesus\">",
        "<q sID=\"1\"/>",
        "<q eID=\"1\"/>",
        "<w lemma=\"strong:G1\">",
        "</w>",
        "<WG12>",
        "<WT",
        "<FR>",
        "<Fr>",
        "<sync type=\"Strongs\" value=\"G5\"/>",
        "<br>",
        "&",
        "&amp;",
        "&#x110000;",
        "&#;",
        "é",
        "ש",
        " ",
        "\n",
        "\r",
        "x",
        "<note>",
        "</note>",
        "<!--",
        "-->",
        "<![CDATA[",
        "]]>",
        "=\"",
        "'",
        "<<",
        "<a b=c>",
        "<?pi?>",
        "<lb/>",
        "<div eID=\"x\"/>",
        "<verse sid=\"JHN 1:1\"/>",
        "<verse eid=\"JHN 1:1\"/>",
        "<char style=\"wj\">",
        "</char>",
        "<para style=\"q1\">",
        "<reference osisRef=\"John.3.16\">",
        "</reference>",
        "John 3:16",
        "<title>",
    ];
    let markups = [
        Markup::Osis,
        Markup::Thml,
        Markup::Gbf,
        Markup::Tei,
        Markup::Usx,
        Markup::Plaintext,
    ];
    let data = BibleData::default();
    let all = TextOptions {
        filter: Filter::all(),
        strongs: true,
        line_breaks: true,
    };
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let rounds = 100_000;
    for _ in 0..rounds {
        let n = (rnd() % 30) as usize;
        let s: String = (0..n)
            .map(|_| atoms[(rnd() % atoms.len() as u64) as usize])
            .collect();
        for markup in markups {
            let nodes = parse(markup, &s);
            let mut flat = Flattener::new(all.clone());
            flat.push(&nodes).push(&nodes);
            let flat = flat.finish();
            for span in &flat.spans {
                assert!(flat.text.is_char_boundary(span.range.start));
                assert!(flat.text.is_char_boundary(span.range.end));
            }
            for i in 0..flat.text.len() {
                if let Some(point) = flat.source(i) {
                    assert!(point.path.resolve(&nodes).is_some());
                }
            }
            let _ = HtmlRenderer::new().render(&nodes);
            let _ = tagged_references(&nodes, &data);
        }
    }
    println!("ok: {} parses without panic", rounds * markups.len());
}
