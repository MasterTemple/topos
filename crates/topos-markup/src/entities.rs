//! Tolerant entity decoding. XML's five entities plus the HTML entities that show
//! up in ThML modules; numeric references; anything unknown (including a bare `&`)
//! is left as-is instead of failing.

const NAMED: &[(&str, &str)] = &[
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

fn accented(name: &str) -> Option<char> {
    // &eacute; &Ouml; &ccedil; &atilde; … for the Latin-1 letters
    let (base, mark) = name.split_at(1);
    let base = base.chars().next()?;
    let table: &[(char, &str, char)] = &[
        ('a', "grave", 'à'),
        ('a', "acute", 'á'),
        ('a', "circ", 'â'),
        ('a', "tilde", 'ã'),
        ('a', "uml", 'ä'),
        ('a', "ring", 'å'),
        ('e', "grave", 'è'),
        ('e', "acute", 'é'),
        ('e', "circ", 'ê'),
        ('e', "uml", 'ë'),
        ('i', "grave", 'ì'),
        ('i', "acute", 'í'),
        ('i', "circ", 'î'),
        ('i', "uml", 'ï'),
        ('o', "grave", 'ò'),
        ('o', "acute", 'ó'),
        ('o', "circ", 'ô'),
        ('o', "tilde", 'õ'),
        ('o', "uml", 'ö'),
        ('o', "slash", 'ø'),
        ('u', "grave", 'ù'),
        ('u', "acute", 'ú'),
        ('u', "circ", 'û'),
        ('u', "uml", 'ü'),
        ('y', "acute", 'ý'),
        ('y', "uml", 'ÿ'),
        ('n', "tilde", 'ñ'),
        ('c', "cedil", 'ç'),
    ];
    let lower = base.to_ascii_lowercase();
    let c = table.iter().find(|(b, m, _)| *b == lower && *m == mark)?.2;
    Some(if base.is_ascii_uppercase() {
        c.to_uppercase().next()?
    } else {
        c
    })
}

/// Decode entities in `s`.
pub fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest[1..]
            .find(|c: char| c == ';' || c == '&' || c.is_whitespace() || c == '<')
            .map(|e| e + 1);
        if let Some(e) = end.filter(|&e| rest.as_bytes()[e] == b';' && e <= 32) {
            let name = &rest[1..e];
            let decoded = if let Some(num) = name.strip_prefix('#') {
                let cp = if let Some(hex) = num.strip_prefix(['x', 'X']) {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    num.parse::<u32>().ok()
                };
                cp.and_then(char::from_u32).map(String::from)
            } else {
                NAMED
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, v)| v.to_string())
                    .or_else(|| accented(name).map(String::from))
            };
            if let Some(d) = decoded {
                out.push_str(&d);
                rest = &rest[e + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::decode_entities as d;
    #[test]
    fn decodes() {
        assert_eq!(
            d("a &amp; b &lt;c&gt; &#8212; &#x41; &eacute;&Uuml; &bogus; R&D"),
            "a & b <c> \u{2014} A éÜ &bogus; R&D"
        );
    }
}
