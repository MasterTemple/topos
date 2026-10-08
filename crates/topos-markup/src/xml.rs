//! OSIS / ThML / TEI / USX fragments, tokenised with `quick-xml` (in a lenient
//! configuration) and assembled by the tolerant [`TreeBuilder`].

use super::classify;
use super::entities::decode_entities;
use super::tree::TreeBuilder;
use super::{Element, Node};
use crate::Markup;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// HTML-style void elements that ThML writes without a closing slash.
const THML_VOID: &[&str] = &[
    "br", "hr", "img", "sync", "pb", "meta", "link", "col", "area", "input",
];

fn element(start: &BytesStart<'_>) -> Element {
    let name = start.name().as_ref().to_string();
    let attrs = start
        .attributes()
        .with_checks(false)
        .filter_map(Result::ok)
        .map(|a| (a.key.as_ref().to_string(), decode_entities(&a.value)))
        .collect();
    Element::new(name, attrs)
}

pub(crate) fn parse(markup: Markup, text: &str) -> Vec<Node> {
    let classify_fn: fn(&mut Element) = match markup {
        Markup::Osis => classify::osis,
        Markup::Thml => classify::thml,
        Markup::Usx => classify::usx,
        _ => classify::tei,
    };
    let thml = markup == Markup::Thml;
    let mut b = TreeBuilder::new(classify_fn);
    let mut reader = Reader::from_str(text);
    {
        let cfg = reader.config_mut();
        cfg.check_end_names = false;
        cfg.allow_unmatched_ends = true;
        cfg.expand_empty_elements = false;
        cfg.trim_text(false);
    }
    loop {
        let pos = reader.buffer_position() as usize;
        match reader.read_event() {
            Ok(Event::Start(s)) => {
                let e = element(&s);
                if thml && THML_VOID.iter().any(|v| e.name.eq_ignore_ascii_case(v)) {
                    handle_empty(&mut b, e, thml);
                } else {
                    b.start(e);
                }
            }
            Ok(Event::Empty(s)) => handle_empty(&mut b, element(&s), thml),
            Ok(Event::End(e)) => {
                let name = e.name().as_ref().to_string();
                if thml && THML_VOID.iter().any(|v| name.eq_ignore_ascii_case(v)) {
                    continue; // `<br></br>` style
                }
                b.end(&name);
            }
            Ok(Event::Text(t)) => b.text(&decode_entities(t.as_ref())),
            Ok(Event::CData(t)) => b.text(t.as_ref()),
            Ok(Event::GeneralRef(r)) => {
                // `&name;` / `&#123;` arrive as separate events since quick-xml 0.38
                b.text(&decode_entities(&format!("&{};", r.as_ref() as &str)));
            }
            Ok(Event::Eof) => break,
            Ok(_) => {} // comments, PIs, doctype
            Err(_) => {
                // Not tokenisable as XML from here (e.g. a stray '<'): keep the rest as text.
                if let Some(rest) = text.get(pos..) {
                    b.text(&decode_entities(rest));
                }
                break;
            }
        }
    }
    b.finish()
}

fn handle_empty(b: &mut TreeBuilder, e: Element, thml: bool) {
    if thml && e.name.eq_ignore_ascii_case("sync") {
        let ty = e.attr("type").unwrap_or("").to_ascii_lowercase();
        if let Some(v) = e.attr("value") {
            match ty.as_str() {
                "strongs" => {
                    b.tag_previous_word("lemma", format!("strong:{v}"));
                    return;
                }
                "morph" => {
                    let class = e.attr("class").map(|c| c.to_ascii_lowercase());
                    let val = match class {
                        Some(c) => format!("{c}:{v}"),
                        None => v.to_string(),
                    };
                    b.tag_previous_word("morph", val);
                    return;
                }
                _ => {}
            }
        }
    }
    b.empty(e);
}
