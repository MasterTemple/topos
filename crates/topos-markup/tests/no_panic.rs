//! Random tag soup through every parser and renderer: nothing may panic, and what the crate
//! hands out (ranges, points, places) must be usable.
//!
//! `NO_PANIC_ROUNDS=100000 cargo test --release --test no_panic` for a longer run.

use topos_bible::data::BibleData;
use topos_bible_markup::{
    Filter, Flattener, HtmlOptions, HtmlRenderer, Kind, Markup, Strongs, TextOptions,
    decode_entities, parse, plain_text,
    refs::tagged_references,
    verses::{chapter_verses, slice, usx_chapter_verses, verses},
};

const PIECES: &[&str] = &[
    "<",
    ">",
    "/",
    "=",
    "\"",
    "'",
    "&",
    ";",
    "#",
    "x",
    " ",
    "\n",
    "\r",
    "\t",
    "é",
    "ש",
    "😀",
    "\u{feff}",
    "&amp;",
    "&#x110000;",
    "&#;",
    "&;",
    "&é;",
    "&#65;",
    "&eacute;",
    "<<",
    "</",
    "/>",
    "<>",
    "<!--",
    "-->",
    "<![CDATA[",
    "]]>",
    "<?pi?>",
    "<!DOCTYPE x>",
    "<!x>",
    "<q who=\"Jesus\">",
    "</q>",
    "<q sID=\"1\"/>",
    "<q who=\"Jesus\" sID=\"1\"/>",
    "<q eID=\"1\"/>",
    "<w lemma=\"strong:G1\">",
    "<w lemma=\"strong:H0430a x\" morph=\"m\">",
    "</w>",
    "<w/>",
    "<note type=\"crossReference\">",
    "<note>",
    "</note>",
    "<title>",
    "</title>",
    "<lb/>",
    "<p>",
    "</p>",
    "<p sID=\"p\"/>",
    "<l eID=\"l\"/>",
    "<lg>",
    "</lg>",
    "<div type=\"x-p\">",
    "<verse osisID=\"John.1.4\">",
    "</verse>",
    "<verse sid=\"JHN 1:1\"/>",
    "<verse eid=\"JHN 1:1\"/>",
    "<verse number=\"2-3\" sid=\"JHN 1:2\"/>",
    "<verse number=\"x\"/>",
    "<verse n=\"300\"/>",
    "<chapter number=\"2\" sid=\"JHN 2\"/>",
    "<chapter eID=\"John.2\"/>",
    "<chapter/>",
    "<char style=\"wj\">",
    "<char style=\"w\" strong=\"G1,H2\">",
    "</char>",
    "<para style=\"q1\">",
    "<para style=\"s1\">",
    "</para>",
    "<book code=\"JHN\">",
    "</book>",
    "<note style=\"x\">",
    "<reference osisRef=\"John.3.16\">",
    "</reference>",
    "<ref loc=\"GEN 1:1\">",
    "</ref>",
    "<scripRef passage=\"Rom 8:28\">",
    "<sync type=\"Strongs\" value=\"G5\"/>",
    "<sync type=\"morph\" class=\"R\" value=\"N\"/>",
    "<br>",
    "<BR>",
    "<img>",
    "</br>",
    "<WG12>",
    "<WH1a>",
    "<WT",
    "<WTV-PAI>",
    "<FR>",
    "<Fr>",
    "<FI>",
    "<Fi>",
    "<RF>",
    "<Rf>",
    "<CM>",
    "<a b=c>",
    "<a b>",
    "< />",
    "<lb/ >",
    "John 3:16",
    "Rom 8:28",
    "In the beginning",
];

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn rich_options() -> TextOptions {
    TextOptions {
        filter: Filter::all(),
        strongs: true,
        line_breaks: true,
        verse_label: Some("[{book} {chapter}:{verse}]".into()),
        chapter_label: Some("{book} {chapter}".into()),
        templates: [
            (Kind::WordsOfJesus, "“{text}”"),
            (Kind::Title, "## {text}"),
            (Kind::Note, "({text})"),
            (Kind::Added, "{text}*"),
            (Kind::Quote, "«"),
        ]
        .into_iter()
        .map(|(k, t)| (k, t.to_string()))
        .collect(),
    }
}

fn exercise(input: &str, data: &BibleData) {
    let _ = decode_entities(input);
    let _ = Strongs::parse(input);
    let _ = Strongs::parse_list(input);
    let _ = usx_chapter_verses(input);
    for markup in [
        Markup::Osis,
        Markup::Thml,
        Markup::Gbf,
        Markup::Tei,
        Markup::Usx,
        Markup::Plaintext,
    ] {
        let nodes = parse(markup, input);
        for opts in [TextOptions::default(), rich_options()] {
            let _ = plain_text(&nodes, &opts);
            let mut flattener = Flattener::new(opts);
            flattener.push(&nodes).push(&nodes).push(&[]);
            let flat = flattener.finish();
            assert_eq!(flat.entries.len(), 3);
            for span in &flat.spans {
                assert!(span.range.start <= span.range.end && span.range.end <= flat.text.len());
                assert!(flat.text.is_char_boundary(span.range.start), "{input:?}");
                assert!(flat.text.is_char_boundary(span.range.end), "{input:?}");
            }
            for range in &flat.entries {
                assert!(flat.text.get(range.clone()).is_some(), "{input:?}");
            }
            for byte in 0..=flat.text.len() {
                if let Some(point) = flat.source(byte) {
                    let node = point.path.resolve(&nodes).and_then(|n| n.as_text());
                    let text = node.expect("a point is in a text node");
                    assert!(point.offset <= text.len());
                }
            }
            let mut html = HtmlRenderer::new();
            let len = flat.text.len();
            for (a, b) in [(0, len), (len / 3, len / 2), (len / 2, len), (len, 0)] {
                if let Some((start, end)) = flat.source_range(a..b) {
                    html.mark(start, end);
                }
            }
            let _ = html.render(&nodes);
            let _ = html.render(&nodes);
            let mut bare = HtmlRenderer::with_options(HtmlOptions {
                class_prefix: "".into(),
                filter: Filter::all(),
            });
            let _ = bare.render(&nodes);
            let _ = bare.wrap(input);
        }
        let _ = tagged_references(&nodes, data);
        let spans = verses(&nodes);
        let _ = chapter_verses(&spans);
        for verse in &spans {
            assert!(verse.start <= verse.end, "{verse:?} in {input:?}");
            let part = slice(&nodes, &verse.start, Some(&verse.end));
            let _ = HtmlRenderer::new().render(&part);
            let _ = slice(&nodes, &verse.end, Some(&verse.start));
            let _ = slice(&nodes, &verse.marker, None);
        }
    }
}

#[test]
fn random_markup_never_panics() {
    let rounds: usize = std::env::var("NO_PANIC_ROUNDS")
        .ok()
        .and_then(|r| r.parse().ok())
        .unwrap_or(3_000);
    let data = BibleData::default();
    let mut rng = Rng(0x2545_F491_4F6C_DD1D);
    for _ in 0..rounds {
        let n = rng.below(40);
        let input: String = (0..n).map(|_| PIECES[rng.below(PIECES.len())]).collect();
        exercise(&input, &data);
    }
}

#[test]
fn every_piece_alone_and_doubled() {
    let data = BibleData::default();
    for piece in PIECES {
        exercise(piece, &data);
        exercise(&piece.repeat(2), &data);
    }
}
