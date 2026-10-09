//! HTML with classes for applications to style.

use std::borrow::Cow;
use std::fmt::Write as _;

use crate::flatten::words_match;
use crate::{Element, Filter, Kind, Milestone, Node, Point, filter::Action};

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
        HtmlOptions {
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
| any other USX `char` (`pn`, `fr`), or table `cell` | `span` | `char`, `cell` | `data-style` |
| mark ([`HtmlRenderer::mark`]) | `mark` | `mark` | `data-mark` (`0` for the first mark) |

Every USX `para`, `char`, `note`, `cell`, and `sidebar` also gets its `style` as `data-style`
(`<p class="topos-p" data-style="m">`, `<span class="topos-l" data-style="q2">`), on its first
tag (a `para style="b"` is `<br class="topos-lb" data-style="b">`): a stylesheet can style every
USX marker, even the ones whose kind is shared (`[data-style="q2"]`, `[data-style^="f"]`).

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
    /// Words of Jesus open across milestones or entries, with the start's id
    words: Option<Option<String>>,
    /// Marks, numbered by their index
    marks: Vec<(Point, Point)>,
    /// How many entries have been rendered
    rendered: usize,
}

/// Escapes text and attribute values
fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
}

/// A value made safe for a class name: ASCII letters (lowercased), digits, and `-`; anything
/// else is `-`
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

/// What follows an element's opening tag
#[derive(Default)]
struct Plan {
    /// Its children are rendered
    children: bool,
    /// Its children are inside words of Jesus
    words: bool,
    /// The end tag
    close: Option<&'static str>,
    /// A line's `<br>` after its end tag
    line_break: bool,
    /// Words of Jesus stay open after it (an unclosed words-of-Jesus element)
    open_words: bool,
}

impl Plan {
    /// Only the children, with no tag of its own
    fn children() -> Self {
        Plan {
            children: true,
            ..Plan::default()
        }
    }
}

/// One entry being rendered
struct Render<'r> {
    renderer: &'r mut HtmlRenderer,
    out: String,
    entry: usize,
    /// How many words-of-Jesus elements the current node is in (kept or unwrapped)
    in_words: usize,
    /// The USX style of the element being opened, for its first tag's `data-style`
    style: Option<String>,
}

impl HtmlRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_options(opts: HtmlOptions) -> Self {
        HtmlRenderer {
            opts,
            ..Self::default()
        }
    }

    /// One entry's nodes; each call is the next entry, as in [`Flattener::push`](crate::Flattener::push)
    pub fn render(&mut self, nodes: &[Node]) -> String {
        let entry = self.rendered;
        self.rendered = self.rendered.saturating_add(1);
        let mut render = Render {
            renderer: self,
            out: String::new(),
            entry,
            in_words: 0,
            style: None,
        };
        let mut path = Vec::new();
        render.nodes(nodes, &mut path);
        render.out
    }

    /**
    Marks the text from `start` to `end` (exclusive) with `<mark class="topos-mark"
    data-mark="n">`, where `n` counts marks in the order they were added
    - The points are where a search match starts and ends, as `topos-bible-formats`'
      `MarkupLocation` gives them for a [`Flat`](crate::Flat) of the same entries (in the same
      order, with any filter): highlight search results without searching the HTML again
    - A mark across elements is one `<mark>` per text node, all with the same `data-mark`
    - Add marks before rendering the entries they are in

    ```
    use topos_bible_markup::{HtmlRenderer, Markup, TextOptions, flatten::flatten, parse};

    let nodes = parse(Markup::Osis, "See <hi type=\"italic\">John</hi> 3:16 today.");
    let flat = flatten(&nodes, &TextOptions::default());
    // A search match for `John 3:16` (`MarkupLocation` gives these points)
    let (start, end) = flat.source_range(4..13).unwrap();
    let mut html = HtmlRenderer::new();
    html.mark(start, end);
    assert_eq!(
        html.render(&nodes),
        r#"See <i class="topos-hi topos-italic"><mark class="topos-mark" data-mark="0">John</mark></i><mark class="topos-mark" data-mark="0"> 3:16</mark> today."#
    );
    ```
    */
    pub fn mark(&mut self, start: Point, end: Point) -> usize {
        self.marks.push((start, end));
        self.marks.len() - 1
    }

    /// Removes every mark
    pub fn clear_marks(&mut self) {
        self.marks.clear();
    }

    /// Wraps rendered HTML in a `div` whose class is the prefix without its trailing `-` or `_`
    pub fn wrap(&self, html: &str) -> String {
        let class = self.opts.class_prefix.trim_end_matches(['-', '_']);
        let mut out = String::from("<div");
        if !class.is_empty() {
            out.push_str(" class=\"");
            escape(class, &mut out);
            out.push('"');
        }
        out.push('>');
        out.push_str(html);
        out.push_str("</div>");
        out
    }
}

impl Render<'_> {
    fn nodes(&mut self, nodes: &[Node], path: &mut Vec<usize>) {
        for (i, node) in nodes.iter().enumerate() {
            path.push(i);
            match node {
                Node::Text(t) => self.text(t, path),
                Node::Element(e) => self.element(e, path),
            }
            path.pop();
        }
    }

    /// The marked parts of a text node, merged, with each part's mark number
    fn marked_parts(&self, text: &str, path: &[usize]) -> Vec<(usize, usize, usize)> {
        let here = (self.entry, path);
        let mut parts = Vec::new();
        for (n, (start, end)) in self.renderer.marks.iter().enumerate() {
            let starts_here = (start.entry, start.path.0.as_slice());
            let ends_here = (end.entry, end.path.0.as_slice());
            if starts_here > here || ends_here < here {
                continue;
            }
            let from = if starts_here == here { start.offset } else { 0 };
            let to = if ends_here == here {
                end.offset.min(text.len())
            } else {
                text.len()
            };
            if from >= to || !text.is_char_boundary(from) || !text.is_char_boundary(to) {
                continue;
            }
            parts.push((from, to, n));
        }
        parts.sort();
        let mut merged: Vec<(usize, usize, usize)> = Vec::new();
        for part in parts {
            match merged.last_mut() {
                Some(last) if part.0 <= last.1 => last.1 = last.1.max(part.1),
                _ => merged.push(part),
            }
        }
        merged
    }

    fn text(&mut self, text: &str, path: &[usize]) {
        let words = self.renderer.words.is_some() && self.in_words == 0 && !text.trim().is_empty();
        if words {
            self.open_tag("span", &[Kind::WordsOfJesus.class_name()], &[]);
        }
        let parts = self.marked_parts(text, path);
        let prefix = self.renderer.opts.class_prefix.clone();
        let mut at = 0;
        for (from, to, n) in parts {
            escape(text.get(at..from).unwrap_or(""), &mut self.out);
            let _ = write!(
                self.out,
                "<mark class=\"{}\" data-mark=\"{n}\">",
                escaped(&format!("{prefix}mark"))
            );
            escape(text.get(from..to).unwrap_or(""), &mut self.out);
            self.out.push_str("</mark>");
            at = to;
        }
        escape(text.get(at..).unwrap_or(""), &mut self.out);
        if words {
            self.out.push_str("</span>");
        }
    }

    /// Writes `<tag class="…" data-…>`, then the opening element's `data-style`
    fn open_tag(&mut self, tag: &str, classes: &[&str], data: &[(&str, Option<String>)]) {
        let prefix = self.renderer.opts.class_prefix.clone();
        let classes: Vec<String> = classes
            .iter()
            .filter(|c| !c.is_empty())
            .map(|c| format!("{prefix}{c}"))
            .collect();
        self.out.push('<');
        self.out.push_str(tag);
        if !classes.is_empty() {
            self.out.push_str(" class=\"");
            escape(&classes.join(" "), &mut self.out);
            self.out.push('"');
        }
        for (name, value) in data {
            if let Some(value) = value {
                let _ = write!(self.out, " {name}=\"");
                escape(value, &mut self.out);
                self.out.push('"');
            }
        }
        if let Some(style) = self.style.take() {
            self.out.push_str(" data-style=\"");
            escape(&style, &mut self.out);
            self.out.push('"');
        }
        self.out.push('>');
    }

    fn close_tag(&mut self, tag: &str) {
        let _ = write!(self.out, "</{tag}>");
    }

    fn element(&mut self, e: &Element, path: &mut Vec<usize>) {
        match self.renderer.opts.filter.action(e) {
            Action::Drop => {}
            Action::Unwrap => {
                // An unwrapped words-of-Jesus element still holds its text: open words of Jesus
                // don't mark it again
                let words = e.kind == Kind::WordsOfJesus;
                if words {
                    self.in_words += 1;
                }
                self.nodes(&e.children, path);
                if words {
                    self.in_words -= 1;
                }
            }
            Action::Keep => self.keep(e, path),
        }
    }

    fn keep(&mut self, e: &Element, path: &mut Vec<usize>) {
        self.style = usx_style(e).map(str::to_string);
        let plan = self.open(e);
        self.style = None;
        if plan.children {
            if plan.words {
                self.in_words += 1;
            }
            self.nodes(&e.children, path);
            if plan.words {
                self.in_words -= 1;
            }
        }
        self.close(&plan);
    }

    /// Writes what comes after an element's contents
    #[inline(never)]
    fn close(&mut self, plan: &Plan) {
        if let Some(tag) = plan.close {
            self.close_tag(tag);
        }
        if plan.line_break {
            self.open_tag("br", &["l"], &[]);
        }
        if plan.open_words {
            self.renderer.words = Some(None);
        }
    }

    /// Writes an opening tag; the contents and the end tag follow
    fn open_wrapped(
        &mut self,
        tag: &'static str,
        classes: &[&str],
        data: &[(&str, Option<String>)],
    ) -> Plan {
        self.open_tag(tag, classes, data);
        Plan {
            children: true,
            close: Some(tag),
            ..Plan::default()
        }
    }

    /// Writes an element's opening tag (or the whole of an element without contents) and says
    /// what follows; kept apart from the recursion so each level of nesting takes little stack
    #[inline(never)]
    fn open(&mut self, e: &Element) -> Plan {
        let attr = |name: &str| e.attr(name).map(str::to_string);
        match e.kind {
            Kind::WordsOfJesus | Kind::Quote if e.milestone.is_some() => match e.milestone {
                Some(Milestone::Start) if e.kind == Kind::WordsOfJesus => {
                    self.renderer.words = Some(e.attr("sID").map(str::to_string));
                }
                Some(Milestone::End) => {
                    if let Some(open) = &self.renderer.words
                        && words_match(open.as_deref(), e)
                    {
                        self.renderer.words = None;
                    }
                }
                _ => {}
            },
            Kind::WordsOfJesus => {
                let mut plan = self.open_wrapped("span", &["woj"], &[]);
                plan.words = true;
                plan.open_words = e.unclosed;
                return plan;
            }
            Kind::Word => {
                let strongs: Vec<String> = e.strongs().iter().map(ToString::to_string).collect();
                let strong = (!strongs.is_empty()).then(|| strongs.join(" "));
                let data = [
                    ("data-strong", strong),
                    ("data-lemma", attr("lemma")),
                    ("data-morph", attr("morph")),
                ];
                return self.open_wrapped("span", &["w"], &data);
            }
            Kind::Note if !e.children.is_empty() => {
                let class = e.note_type().map(|t| {
                    if t.eq_ignore_ascii_case("crossReference") {
                        String::from("crossref")
                    } else {
                        class_safe(t)
                    }
                });
                let data = [
                    ("data-note-type", attr("type")),
                    ("data-n", attr("n").or_else(|| attr("caller"))),
                    ("data-osis-ref", attr("osisRef")),
                ];
                let classes = ["note", class.as_deref().unwrap_or("")];
                return self.open_wrapped("span", &classes, &data);
            }
            Kind::Reference => {
                let data = [("data-osis-ref", e.reference_target().map(str::to_string))];
                return self.open_wrapped("a", &["ref"], &data);
            }
            Kind::Title if !e.children.is_empty() && e.milestone.is_none() => {
                let class = e.title_type().map(class_safe);
                let data = [("data-title-type", attr("type"))];
                let classes = ["title", class.as_deref().unwrap_or("")];
                return self.open_wrapped("div", &classes, &data);
            }
            Kind::Quote | Kind::Added | Kind::DivineName | Kind::Foreign => {
                return self.open_wrapped("span", &[e.kind.class_name()], &[]);
            }
            Kind::Highlight => {
                let style = e.style().map(class_safe);
                let tag = match style.as_deref() {
                    Some("italic") => "i",
                    Some("bold") => "b",
                    Some("super") => "sup",
                    Some("sub") => "sub",
                    _ => "span",
                };
                let classes = ["hi", style.as_deref().unwrap_or("")];
                return self.open_wrapped(tag, &classes, &[]);
            }
            Kind::LineBreak => self.open_tag("br", &["lb"], &[]),
            Kind::Paragraph => match e.milestone {
                Some(Milestone::End) => {}
                None if !e.children.is_empty() => return self.open_wrapped("p", &["p"], &[]),
                _ => self.open_tag("br", &["p"], &[]),
            },
            Kind::LineGroup if e.milestone.is_none() => {
                return self.open_wrapped("div", &["lg"], &[]);
            }
            Kind::Line => match e.milestone {
                Some(Milestone::End) => self.open_tag("br", &["l"], &[]),
                Some(Milestone::Start) => {}
                None => {
                    let mut plan = self.open_wrapped("span", &["l"], &[]);
                    plan.line_break = true;
                    return plan;
                }
            },
            Kind::Verse | Kind::Chapter if e.children.is_empty() => {
                if e.milestone != Some(Milestone::End) {
                    let data = [
                        ("data-osis-id", attr("osisID").or_else(|| attr("sID"))),
                        ("data-n", attr("n").or_else(|| attr("number"))),
                    ];
                    self.open_tag("span", &[e.kind.class_name()], &data);
                    self.close_tag("span");
                }
            }
            Kind::Entry => return self.open_wrapped("div", &["entry"], &[]),
            Kind::Division if self.style.is_some() => {
                return self.open_wrapped("div", &["div"], &[]);
            }
            Kind::Other if self.style.is_some() && !e.children.is_empty() => {
                let class = class_safe(&e.name);
                return self.open_wrapped("span", &[&class], &[]);
            }
            _ => return Plan::children(),
        }
        Plan::default()
    }
}

/// A USX element's `style` (`q2`, `pn`): of `para`, `char`, `cell`, `sidebar`, and notes (which
/// have a `caller`, unlike other markups' `note`s)
fn usx_style(e: &Element) -> Option<&str> {
    let usx = match e.name.as_str() {
        "para" | "char" | "cell" | "sidebar" => true,
        "note" => e.attr("caller").is_some(),
        _ => false,
    };
    usx.then(|| e.attr("style")).flatten().filter(|s| !s.is_empty())
}

/// An escaped copy
fn escaped(s: &str) -> String {
    let mut out = String::new();
    escape(s, &mut out);
    out
}
