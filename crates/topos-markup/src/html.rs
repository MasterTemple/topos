//! HTML with classes for applications to style.

use std::borrow::Cow;

use crate::{
    Element, Kind, Milestone, Node,
    filter::{Action, Filter},
    flatten::ends_woj,
};

/// Options for [`HtmlRenderer`]
#[derive(Debug, Clone)]
pub struct HtmlOptions {
    /**
    Put before every class name; default: `topos-`
    - The wrapper's class is the prefix without its trailing `-` or `_` (`topos`)
    - An empty prefix gives bare class names (`woj`) and a wrapper without a class
    - Data attributes are never prefixed
    */
    pub class_prefix: Cow<'static, str>,
    /// Which elements are rendered; default: [`Filter::all`] except metadata
    pub filter: Filter,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self {
            class_prefix: Cow::Borrowed("topos-"),
            filter: Filter::all().drop(Kind::Metadata),
        }
    }
}

/**
Renders HTML with classes and data attributes, never inline styles

Classes (with the prefix, `topos-` by default):

| Element | Tag | Classes | Data attributes |
|---|---|---|---|
| word | `span` | `w` | `data-strong` (`H7225 H853`), `data-lemma`, `data-morph` |
| note | `span` | `note`, and `crossref`, `footnote`, `study`, ... from its type | `data-note-type`, `data-n`, `data-osis-ref` |
| reference | `a` | `ref` | `data-osis-ref` |
| title | `div` | `title`, and its type (`psalm`, `s1`) | `data-title-type` |
| words of Jesus | `span` | `woj` | |
| quote, translator addition, divine name, foreign | `span` | `quote`, `added`, `divine-name`, `foreign` | |
| highlight | `i`, `b`, `sup`, `sub`, `span` | `hi`, and its style (`italic`, `small-caps`) | |
| paragraph, line group, line | `p`, `div`, `span` | `p`, `lg`, `l` | |
| verse, chapter (milestones) | `span` | `verse`, `chapter` | `data-osis-id`, `data-n` |
| dictionary entry | `div` | `entry` | |

Keep one renderer for a passage, so words of Jesus that continue from one verse into the next
(`sID`/`eID` milestones or an unclosed `<q>`) stay marked. [`HtmlRenderer::wrap`] puts a
passage in one element whose class is the bare prefix (`<div class="topos">`).

```
use topos_bible_markup::{HtmlRenderer, Markup, parse};

let mut html = HtmlRenderer::new();
let verse = html.render(&parse(Markup::Osis, r#"<w lemma="strong:G2424">Jesus</w> said, <q who="Jesus">Follow me</q>"#));
assert_eq!(
    verse,
    r#"<span class="topos-w" data-strong="G2424" data-lemma="strong:G2424">Jesus</span> said, <span class="topos-woj">Follow me</span>"#
);
assert_eq!(html.wrap(&verse), format!(r#"<div class="topos">{verse}</div>"#));
```
*/
#[derive(Debug, Default, Clone)]
pub struct HtmlRenderer {
    pub opts: HtmlOptions,
    /// Words of Jesus still open from a milestone or an unclosed `<q>`, with the start's `sID`
    woj: Option<Option<String>>,
}

impl HtmlRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_options(opts: HtmlOptions) -> Self {
        Self { opts, woj: None }
    }

    /// One entry's nodes
    pub fn render(&mut self, nodes: &[Node]) -> String {
        let mut out = String::new();
        self.nodes(nodes, &mut out, false);
        out
    }

    /// Wraps rendered HTML in a `div` whose class is the prefix without its trailing `-` or `_`
    pub fn wrap(&self, html: &str) -> String {
        let class = self.opts.class_prefix.trim_end_matches(['-', '_']);
        if class.is_empty() {
            format!("<div>{html}</div>")
        } else {
            let mut out = String::from("<div class=\"");
            escape(class, &mut out);
            out.push_str("\">");
            out.push_str(html);
            out.push_str("</div>");
            out
        }
    }

    fn class(&self, names: &[&str]) -> String {
        let mut out = String::new();
        for name in names.iter().filter(|n| !n.is_empty()) {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&self.opts.class_prefix);
            out.push_str(name);
        }
        out
    }

    fn nodes(&mut self, nodes: &[Node], out: &mut String, in_woj: bool) {
        for n in nodes {
            match n {
                Node::Text(t) => {
                    if self.woj.is_some() && !in_woj && !t.trim().is_empty() {
                        let class = self.class(&[Kind::WordsOfJesus.class_name()]);
                        open(out, "span", &class, &[]);
                        escape(t, out);
                        out.push_str("</span>");
                    } else {
                        escape(t, out);
                    }
                }
                Node::Element(e) => self.element(e, out, in_woj),
            }
        }
    }

    fn wrap_element(
        &mut self,
        tag: &str,
        classes: &[&str],
        data: &[(&str, Option<&str>)],
        e: &Element,
        out: &mut String,
        in_woj: bool,
    ) {
        let class = self.class(classes);
        open(out, tag, &class, data);
        self.nodes(&e.children, out, in_woj);
        out.push_str("</");
        out.push_str(tag);
        out.push('>');
    }

    fn empty(&self, tag: &str, classes: &[&str], data: &[(&str, Option<&str>)], out: &mut String) {
        let class = self.class(classes);
        open(out, tag, &class, data);
        if tag != "br" {
            out.push_str("</");
            out.push_str(tag);
            out.push('>');
        }
    }

    fn element(&mut self, e: &Element, out: &mut String, in_woj: bool) {
        match self.opts.filter.action(e) {
            Action::Drop => return,
            Action::Unwrap => return self.nodes(&e.children, out, in_woj),
            Action::Keep => {}
        }
        let kind = e.kind.class_name();
        match e.kind {
            Kind::WordsOfJesus | Kind::Quote if e.milestone.is_some() => match e.milestone {
                Some(Milestone::Start) if e.kind == Kind::WordsOfJesus => {
                    self.woj = Some(e.attr("sID").map(str::to_string));
                }
                Some(Milestone::End)
                    if self
                        .woj
                        .as_ref()
                        .is_some_and(|id| ends_woj(id.as_deref(), e)) =>
                {
                    self.woj = None;
                }
                _ => {}
            },
            Kind::WordsOfJesus => {
                self.wrap_element("span", &[kind], &[], e, out, true);
                if e.unclosed {
                    // Continues into the next entry
                    self.woj = Some(None);
                }
            }
            Kind::Word => {
                let strongs: Vec<String> = e.strongs().iter().map(ToString::to_string).collect();
                let strongs = strongs.join(" ");
                let data = [
                    (
                        "data-strong",
                        (!strongs.is_empty()).then_some(strongs.as_str()),
                    ),
                    ("data-lemma", e.attr("lemma")),
                    ("data-morph", e.attr("morph")),
                ];
                self.wrap_element("span", &[kind], &data, e, out, in_woj);
            }
            Kind::Note if !e.children.is_empty() => {
                let note_type = e.note_type().map(note_class).unwrap_or_default();
                let data = [
                    ("data-note-type", e.note_type()),
                    ("data-n", e.attr("n").or_else(|| e.attr("caller"))),
                    ("data-osis-ref", e.attr("osisRef")),
                ];
                self.wrap_element("span", &[kind, &note_type], &data, e, out, in_woj);
            }
            Kind::Reference => {
                let data = [("data-osis-ref", e.reference_target())];
                self.wrap_element("a", &[kind], &data, e, out, in_woj);
            }
            Kind::Title if !e.children.is_empty() && e.milestone.is_none() => {
                let title_type = e.title_type().map(class_safe).unwrap_or_default();
                let data = [("data-title-type", e.title_type())];
                self.wrap_element("div", &[kind, &title_type], &data, e, out, in_woj);
            }
            Kind::Quote | Kind::Added | Kind::DivineName | Kind::Foreign => {
                self.wrap_element("span", &[kind], &[], e, out, in_woj)
            }
            Kind::Highlight => {
                let style = e.style().map(class_safe).unwrap_or_default();
                let tag = match style.as_str() {
                    "italic" => "i",
                    "bold" => "b",
                    "super" => "sup",
                    "sub" => "sub",
                    _ => "span",
                };
                self.wrap_element(tag, &[kind, &style], &[], e, out, in_woj);
            }
            Kind::LineBreak => self.empty("br", &[kind], &[], out),
            Kind::Paragraph => match e.milestone {
                Some(Milestone::End) => {}
                None if !e.children.is_empty() => {
                    self.wrap_element("p", &[kind], &[], e, out, in_woj)
                }
                _ => self.empty("br", &[kind], &[], out),
            },
            Kind::LineGroup if e.milestone.is_none() => {
                self.wrap_element("div", &[kind], &[], e, out, in_woj)
            }
            Kind::Line => match e.milestone {
                Some(Milestone::End) => self.empty("br", &[kind], &[], out),
                Some(Milestone::Start) => {}
                None => {
                    self.wrap_element("span", &[kind], &[], e, out, in_woj);
                    self.empty("br", &[kind], &[], out);
                }
            },
            Kind::Verse | Kind::Chapter if e.children.is_empty() => {
                if e.milestone != Some(Milestone::End) {
                    let data = [
                        ("data-osis-id", e.attr("osisID").or_else(|| e.attr("sID"))),
                        ("data-n", e.attr("n").or_else(|| e.attr("number"))),
                    ];
                    self.empty("span", &[kind], &data, out);
                }
            }
            Kind::Entry => self.wrap_element("div", &[kind], &[], e, out, in_woj),
            _ => self.nodes(&e.children, out, in_woj),
        }
    }
}

/// A note type as a class: `crossReference` is `crossref`, others are lowercased
fn note_class(note_type: &str) -> String {
    if note_type.eq_ignore_ascii_case("crossReference") {
        "crossref".to_string()
    } else {
        class_safe(note_type)
    }
}

/// Lowercase letters, digits, and `-`, with anything else as `-`
fn class_safe(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn open(out: &mut String, tag: &str, class: &str, data: &[(&str, Option<&str>)]) {
    out.push('<');
    out.push_str(tag);
    if !class.is_empty() {
        out.push_str(" class=\"");
        escape(class, out);
        out.push('"');
    }
    for (name, value) in data {
        if let Some(value) = value {
            out.push(' ');
            out.push_str(name);
            out.push_str("=\"");
            escape(value, out);
            out.push('"');
        }
    }
    out.push('>');
}

fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}
