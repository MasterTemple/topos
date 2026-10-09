//! The XML-like markups (OSIS, ThML, TEI, USX), read leniently with quick-xml.

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::tree::Builder;
use crate::{Markup, Node, decode_entities};

/// ThML elements written without an end tag
const THML_VOID: &[&str] = &[
    "br", "hr", "img", "sync", "pb", "meta", "link", "col", "area", "input",
];

/// XML's whitespace (the `S` production), which separates names and attributes in a tag
fn is_xml_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

pub(crate) fn parse(markup: Markup, text: &str) -> Vec<Node> {
    // quick-xml drops one byte order mark at the start, and counts positions after it
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut builder = Builder::new(markup);
    let mut reader = Reader::from_str(text);
    {
        let config = reader.config_mut();
        config.check_end_names = false;
        config.allow_unmatched_ends = true;
        config.expand_empty_elements = false;
        config.trim_text(false);
        config.check_comments = false;
        config.allow_dangling_amp = false;
    }
    // Text is held back until the markup after it is read, so that when that markup can't be
    // read, everything from the end of the last complete construct is kept as text instead
    let mut pending = String::new();
    let mut good_end = 0usize;
    loop {
        let event = reader.read_event();
        let position = usize::try_from(reader.buffer_position()).unwrap_or(usize::MAX);
        match event {
            Ok(Event::Text(t)) => {
                pending.push_str(&t);
                continue;
            }
            Ok(Event::Eof) => {
                builder.text(&pending);
                break;
            }
            Err(_) => {
                let rest = body.get(good_end..).unwrap_or("");
                builder.text(&decode_entities(rest));
                break;
            }
            Ok(event) => {
                builder.text(&pending);
                pending.clear();
                good_end = position;
                match event {
                    Event::GeneralRef(r) => {
                        let reference: &str = &r;
                        builder.text(&decode_entities(&format!("&{reference};")));
                    }
                    Event::CData(c) => builder.text(&c.into_inner()),
                    Event::Start(s) => start(&mut builder, markup, &s, false),
                    Event::Empty(s) => start(&mut builder, markup, &s, true),
                    Event::End(e) => {
                        let content: &str = &e;
                        let name = content.split(is_xml_space).next().unwrap_or("");
                        if markup == Markup::Thml && is_void(name) {
                            continue;
                        }
                        builder.end(name);
                    }
                    _ => {}
                }
            }
        }
    }
    builder.finish()
}

fn is_void(name: &str) -> bool {
    THML_VOID.iter().any(|v| v.eq_ignore_ascii_case(name))
}

fn start(builder: &mut Builder, markup: Markup, tag: &BytesStart, empty: bool) {
    let name = tag.name().0.to_string();
    let attrs: Vec<(String, String)> = tag
        .attributes()
        .with_checks(false)
        .filter_map(Result::ok)
        .map(|a| (a.key.0.to_string(), decode_entities(&a.value)))
        .collect();
    if markup == Markup::Thml && is_void(&name) {
        if name.eq_ignore_ascii_case("sync") && sync(builder, &attrs) {
            return;
        }
        builder.empty(name, attrs);
    } else if empty {
        builder.empty(name, attrs);
    } else {
        builder.start(name, attrs);
    }
}

/// ThML's `<sync>` tags the word before it with a Strong's number or morphology; false when it
/// is not one of those and is an ordinary element
fn sync(builder: &mut Builder, attrs: &[(String, String)]) -> bool {
    let probe = crate::Element {
        kind: crate::Kind::Other,
        name: String::new(),
        attrs: attrs.to_vec(),
        children: Vec::new(),
        milestone: None,
        unclosed: false,
    };
    let (Some(kind), Some(value)) = (probe.attr("type"), probe.attr("value")) else {
        return false;
    };
    if kind.eq_ignore_ascii_case("Strongs") {
        builder.tag_previous_word("lemma", &format!("strong:{value}"));
        true
    } else if kind.eq_ignore_ascii_case("morph") {
        let morph = match probe.attr("class") {
            Some(class) => format!("{}:{value}", class.to_lowercase()),
            None => value.to_string(),
        };
        builder.tag_previous_word("morph", &morph);
        true
    } else {
        false
    }
}
