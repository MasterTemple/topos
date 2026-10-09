//! Character and entity references.

/// The longest reference body (the text between `&` and `;`) that is looked up
const MAX_BODY: usize = 31;

/// Named references other than the accented Latin-1 letters
const NAMED: &[(&str, char)] = &[
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("nbsp", '\u{a0}'),
    ("mdash", '\u{2014}'),
    ("ndash", '\u{2013}'),
    ("hellip", '\u{2026}'),
    ("lsquo", '\u{2018}'),
    ("rsquo", '\u{2019}'),
    ("ldquo", '\u{201c}'),
    ("rdquo", '\u{201d}'),
    ("laquo", '\u{ab}'),
    ("raquo", '\u{bb}'),
    ("para", '\u{b6}'),
    ("sect", '\u{a7}'),
    ("copy", '\u{a9}'),
    ("reg", '\u{ae}'),
    ("deg", '\u{b0}'),
    ("middot", '\u{b7}'),
    ("dagger", '\u{2020}'),
    ("Dagger", '\u{2021}'),
    ("bull", '\u{2022}'),
    ("times", '\u{d7}'),
    ("divide", '\u{f7}'),
    ("frac12", '\u{bd}'),
    ("frac14", '\u{bc}'),
    ("frac34", '\u{be}'),
    ("shy", '\u{ad}'),
    ("iexcl", '\u{a1}'),
    ("iquest", '\u{bf}'),
    ("szlig", '\u{df}'),
    ("aelig", '\u{e6}'),
    ("AElig", '\u{c6}'),
    ("oelig", '\u{153}'),
    ("OElig", '\u{152}'),
    ("thinsp", '\u{2009}'),
    ("ensp", '\u{2002}'),
    ("emsp", '\u{2003}'),
    ("zwj", '\u{200d}'),
    ("zwnj", '\u{200c}'),
    ("lrm", '\u{200e}'),
    ("rlm", '\u{200f}'),
];

/// Accent words, with the lowercase letters each applies to and the letters they make
const ACCENTS: &[(&str, &str, &str)] = &[
    ("grave", "aeiou", "àèìòù"),
    ("acute", "aeiouy", "áéíóúý"),
    ("circ", "aeiou", "âêîôû"),
    ("tilde", "aon", "ãõñ"),
    ("uml", "aeiouy", "äëïöüÿ"),
    ("ring", "a", "å"),
    ("slash", "o", "ø"),
    ("cedil", "c", "ç"),
];

/// Decode entities in `s`.
pub fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        let (before, from_amp) = rest.split_at(amp);
        out.push_str(before);
        let after = &from_amp[1..];
        let stop = after
            .find(|c: char| c == ';' || c == '&' || c == '<' || c.is_whitespace())
            .unwrap_or(after.len());
        let body = &after[..stop];
        let terminated = after[stop..].starts_with(';');
        if terminated
            && body.len() <= MAX_BODY
            && let Some(c) = reference(body)
        {
            out.push(c);
            rest = &after[stop + 1..];
        } else {
            out.push('&');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

/// The character a reference body stands for
fn reference(body: &str) -> Option<char> {
    match body.strip_prefix('#') {
        Some(number) => numeric(number),
        None => named(body),
    }
}

fn numeric(number: &str) -> Option<char> {
    let value = match number.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => number.parse::<u32>().ok()?,
    };
    char::from_u32(value)
}

fn named(name: &str) -> Option<char> {
    if let Some((_, c)) = NAMED.iter().find(|(n, _)| *n == name) {
        return Some(*c);
    }
    let mut chars = name.chars();
    let base = chars.next()?;
    let accent = chars.as_str();
    let (_, bases, letters) = ACCENTS.iter().find(|(a, _, _)| *a == accent)?;
    let index = bases.chars().position(|b| b == base.to_ascii_lowercase())?;
    if !base.is_ascii_alphabetic() {
        return None;
    }
    let letter = letters.chars().nth(index)?;
    if base.is_ascii_uppercase() {
        letter.to_uppercase().next()
    } else {
        Some(letter)
    }
}
