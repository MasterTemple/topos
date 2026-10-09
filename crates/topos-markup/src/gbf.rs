//! GBF: text with `<XY…>` tokens.

use crate::tree::Builder;
use crate::{Markup, Node, decode_entities};

/// Codes that open with two capitals and close with the second letter in lowercase
const PAIRED: &[&str] = &[
    "FI", "FB", "FU", "FS", "FV", "FC", "FN", "FR", "FO", "RF", "RX", "TS", "TT", "TH", "PP", "RB",
];

pub(crate) fn parse(text: &str) -> Vec<Node> {
    let mut builder = Builder::new(Markup::Gbf);
    let mut rest = text;
    while let Some(lt) = rest.find('<') {
        let (before, from_lt) = rest.split_at(lt);
        builder.text(&decode_entities(before));
        let after = &from_lt[1..];
        let Some(gt) = after.find('>') else {
            builder.text(&decode_entities(from_lt));
            return builder.finish();
        };
        let body = &after[..gt];
        if is_token(body) {
            token(&mut builder, body);
            rest = &after[gt + 1..];
        } else {
            builder.text("<");
            rest = after;
        }
    }
    builder.text(&decode_entities(rest));
    builder.finish()
}

fn is_token(body: &str) -> bool {
    let mut chars = body.chars();
    let (Some(first), Some(second)) = (chars.next(), chars.next()) else {
        return false;
    };
    first.is_ascii_uppercase()
        && second.is_ascii_alphabetic()
        && !body.contains(|c: char| c.is_whitespace() || c == '<')
}

fn token(builder: &mut Builder, body: &str) {
    // A token starts with two ASCII letters, so these splits are on character boundaries
    let (code, value) = body.split_at(2.min(body.len()));
    let third_is_digit = value.starts_with(|c: char| c.is_ascii_digit());
    if (code == "WG" || code == "WH") && third_is_digit {
        builder.tag_previous_word("lemma", &format!("strong:{}", &body[1..]));
        return;
    }
    if code == "WT" && !value.is_empty() {
        builder.tag_previous_word("morph", value);
        return;
    }
    let upper = code.to_ascii_uppercase();
    let attrs = if value.is_empty() {
        Vec::new()
    } else {
        vec![(String::from("value"), value.to_string())]
    };
    if PAIRED.contains(&upper.as_str()) {
        if code[1..].starts_with(|c: char| c.is_ascii_lowercase()) {
            builder.end(&upper);
        } else {
            builder.start(upper, attrs);
        }
    } else {
        builder.empty(code.to_string(), attrs);
    }
}
