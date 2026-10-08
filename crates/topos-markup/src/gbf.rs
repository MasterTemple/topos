//! GBF (General Bible Format). Not XML: tokens are `<CODE>` where paired codes
//! open with two capitals (`<FR>`) and close with the second letter lowercased
//! (`<Fr>`). Strong's (`<WG25>`, `<WH430>`) and morphology (`<WTV-PAI-3S>`)
//! tokens follow the word they tag.
//!
//! Anything in angle brackets that does not look like a GBF token (e.g. the
//! literal `<<Geschrieben von Rom…>` in GerLut1545) is kept as text.

use super::classify;
use super::entities::decode_entities;
use super::tree::TreeBuilder;
use super::{Element, Node};

/// Codes that come in start/end pairs.
const PAIRED: &[&str] = &[
    "FI", "FB", "FU", "FS", "FV", "FC", "FN", "FR", "FO", "RF", "RX", "TS", "TT", "TH", "PP", "RB",
];

fn is_token(body: &str) -> bool {
    let b = body.as_bytes();
    b.len() >= 2
        && b[0].is_ascii_uppercase()
        && b[1].is_ascii_alphabetic()
        && !body.contains(|c: char| c.is_whitespace() || c == '<')
}

pub(crate) fn parse(text: &str) -> Vec<Node> {
    let mut b = TreeBuilder::new(classify::gbf);
    let mut rest = text;
    while let Some(lt) = rest.find('<') {
        let (before, after) = rest.split_at(lt);
        b.text(&decode_entities(before));
        let Some(gt) = after.find('>') else {
            b.text(&decode_entities(after));
            rest = "";
            break;
        };
        let body = &after[1..gt];
        if is_token(body) {
            token(&mut b, body);
            rest = &after[gt + 1..];
        } else {
            b.text("<");
            rest = &after[1..];
        }
    }
    b.text(&decode_entities(rest));
    b.finish()
}

fn token(b: &mut TreeBuilder, body: &str) {
    let bytes = body.as_bytes();
    // Strong's: WG1234 / WH1234 (optionally with a letter suffix)
    if bytes[0] == b'W'
        && (bytes[1] == b'G' || bytes[1] == b'H')
        && bytes.get(2).is_some_and(u8::is_ascii_digit)
    {
        b.tag_previous_word("lemma", format!("strong:{}", &body[1..]));
        return;
    }
    // Morphology: WT<code>
    if body.starts_with("WT") && body.len() > 2 {
        b.tag_previous_word("morph", body[2..].to_string());
        return;
    }
    let code = &body[..2];
    let upper = code.to_ascii_uppercase();
    if PAIRED.contains(&upper.as_str()) {
        let attrs = if body.len() > 2 {
            vec![("value".to_string(), body[2..].to_string())]
        } else {
            Vec::new()
        };
        if bytes[1].is_ascii_lowercase() {
            b.end(&upper);
        } else {
            b.start(Element::new(upper, attrs));
        }
        return;
    }
    let attrs = if body.len() > 2 {
        vec![("value".to_string(), body[2..].to_string())]
    } else {
        Vec::new()
    };
    b.empty(Element::new(code.to_string(), attrs));
}
