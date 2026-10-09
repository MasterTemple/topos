//! `decode_entities` (docs/spec/entities.md).

use topos_bible_markup::decode_entities as decode;

#[test]
fn known_numeric_unknown_and_bare() {
    assert_eq!(
        decode("a &amp; b &lt;c&gt; &#8212; &#x41; &eacute;&Uuml; &bogus; R&D"),
        "a & b <c> \u{2014} A éÜ &bogus; R&D"
    );
}

#[test]
fn every_named_entity() {
    let table = [
        ("amp", "&"),
        ("lt", "<"),
        ("gt", ">"),
        ("quot", "\""),
        ("apos", "'"),
        ("nbsp", "\u{a0}"),
        ("mdash", "\u{2014}"),
        ("ndash", "\u{2013}"),
        ("hellip", "\u{2026}"),
        ("lsquo", "\u{2018}"),
        ("rsquo", "\u{2019}"),
        ("ldquo", "\u{201c}"),
        ("rdquo", "\u{201d}"),
        ("laquo", "\u{ab}"),
        ("raquo", "\u{bb}"),
        ("para", "\u{b6}"),
        ("sect", "\u{a7}"),
        ("copy", "\u{a9}"),
        ("reg", "\u{ae}"),
        ("deg", "\u{b0}"),
        ("middot", "\u{b7}"),
        ("dagger", "\u{2020}"),
        ("Dagger", "\u{2021}"),
        ("bull", "\u{2022}"),
        ("times", "\u{d7}"),
        ("divide", "\u{f7}"),
        ("frac12", "\u{bd}"),
        ("frac14", "\u{bc}"),
        ("frac34", "\u{be}"),
        ("shy", "\u{ad}"),
        ("iexcl", "\u{a1}"),
        ("iquest", "\u{bf}"),
        ("szlig", "\u{df}"),
        ("aelig", "\u{e6}"),
        ("AElig", "\u{c6}"),
        ("oelig", "\u{153}"),
        ("OElig", "\u{152}"),
        ("thinsp", "\u{2009}"),
        ("ensp", "\u{2002}"),
        ("emsp", "\u{2003}"),
        ("zwj", "\u{200d}"),
        ("zwnj", "\u{200c}"),
        ("lrm", "\u{200e}"),
        ("rlm", "\u{200f}"),
    ];
    for (name, value) in table {
        assert_eq!(decode(&format!("&{name};")), value, "{name}");
    }
    for name in [
        "AMP", "Amp", "hearts", "euro", "ymacr", "Ae", "EACUTE", "eAcute", "ss",
    ] {
        let reference = format!("&{name};");
        assert_eq!(decode(&reference), reference, "{name}");
    }
}

#[test]
fn accented_letters() {
    let letters = [
        ("grave", "aeiou", "àèìòù"),
        ("acute", "aeiouy", "áéíóúý"),
        ("circ", "aeiou", "âêîôû"),
        ("tilde", "aon", "ãõñ"),
        ("uml", "aeiouy", "äëïöüÿ"),
        ("ring", "a", "å"),
        ("slash", "o", "ø"),
        ("cedil", "c", "ç"),
    ];
    for (accent, bases, results) in letters {
        for (base, result) in bases.chars().zip(results.chars()) {
            assert_eq!(decode(&format!("&{base}{accent};")), result.to_string());
            let upper = base.to_ascii_uppercase();
            let expected: String = result.to_uppercase().collect();
            assert_eq!(decode(&format!("&{upper}{accent};")), expected);
        }
    }
    assert_eq!(decode("&Yuml;"), "\u{178}");
    assert_eq!(decode("&bcirc;"), "&bcirc;");
}

#[test]
fn numeric_references() {
    assert_eq!(decode("&#65;&#x42;&#X43;&#0067;"), "ABCC");
    assert_eq!(decode("&#9;&#xa;"), "\t\n");
    assert_eq!(decode("&#x1F600;"), "😀");
    assert_eq!(decode("&#0;"), "\u{0}");
    assert_eq!(decode("&#+65;&#x+41;"), "AA");
    for kept in [
        "&#;",
        "&#x;",
        "&#-65;",
        "&#x110000;",
        "&#xD800;",
        "&#xZZ;",
        "&#4294967296;",
        "&#65",
    ] {
        assert_eq!(decode(kept), kept, "{kept}");
    }
}

#[test]
fn body_limits() {
    // At most 31 bytes between `&` and `;`
    let ok = format!("&#{}65;", "0".repeat(28));
    assert_eq!(decode(&ok), "A");
    let long = format!("&#{}65;", "0".repeat(30));
    assert_eq!(decode(&long), long);
}

#[test]
fn what_stops_a_reference() {
    assert_eq!(decode("&amp"), "&amp");
    assert_eq!(decode("&a&amp;"), "&a&");
    assert_eq!(decode("&&amp;"), "&&");
    assert_eq!(decode("&a b;"), "&a b;");
    assert_eq!(decode("&a<b;"), "&a<b;");
    assert_eq!(decode("a&"), "a&");
    assert_eq!(decode("&amp;lt;"), "&lt;");
    assert_eq!(decode("&nbsp;x"), "\u{a0}x");
    assert_eq!(decode("no references"), "no references");
    assert_eq!(decode(""), "");
}
