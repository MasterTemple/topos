//! The golden-file format and the outputs it records (see `docs/spec/golden-format.md`).
//!
//! Everything here uses only the crate's public API.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use topos_bible::data::BibleData;
use topos_bible_markup::{
    Filter, Flat, Flattener, HtmlOptions, HtmlRenderer, Kind, Markup, Milestone, Node, TextOptions,
    refs::tagged_references,
    verses::{chapter_verses, slice, usx_chapter_verses, verses},
};

/// Escapes a string for one line of a dump: `\\`, `\"`, `\n`, `\r`, `\t`, and other control
/// characters as `\u{hex}`
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The tree as text: one line per node, two spaces of indent per level
pub fn dump_tree(nodes: &[Node]) -> String {
    let mut out = String::new();
    dump_nodes(nodes, 0, &mut out);
    out
}

fn dump_nodes(nodes: &[Node], depth: usize, out: &mut String) {
    for node in nodes {
        for _ in 0..depth {
            out.push_str("  ");
        }
        match node {
            Node::Text(t) => {
                out.push_str("text ");
                out.push_str(&escape(t));
                out.push('\n');
            }
            Node::Element(e) => {
                let _ = write!(out, "{:?} {}", e.kind, e.name);
                match e.milestone {
                    Some(Milestone::Start) => out.push_str(" @start"),
                    Some(Milestone::End) => out.push_str(" @end"),
                    None => {}
                }
                if e.unclosed {
                    out.push_str(" @unclosed");
                }
                for (k, v) in &e.attrs {
                    let _ = write!(out, " {k}={}", escape(v));
                }
                out.push('\n');
                dump_nodes(&e.children, depth + 1, out);
            }
        }
    }
}

/// The text options of the `text-all` section: everything kept, Strong's numbers, labels, and
/// templates
pub fn all_options() -> TextOptions {
    TextOptions {
        filter: Filter::all(),
        strongs: true,
        line_breaks: true,
        verse_label: Some(String::from("[{book} {chapter}:{verse}]")),
        chapter_label: Some(String::from("{book} {chapter}")),
        templates: [
            (Kind::WordsOfJesus, "“{text}”"),
            (Kind::Title, "## {text}"),
            (Kind::Note, "({text})"),
            (Kind::Added, "{text}*"),
            (Kind::DivineName, "^"),
        ]
        .into_iter()
        .map(|(kind, template)| (kind, template.to_string()))
        .collect::<BTreeMap<_, _>>(),
    }
}

fn flat(entries: &[Vec<Node>], opts: TextOptions) -> Flat {
    let mut f = Flattener::new(opts);
    for nodes in entries {
        f.push(nodes);
    }
    f.finish()
}

fn dump_spans(flat: &Flat) -> String {
    let mut out = String::from("entries:");
    for r in &flat.entries {
        let _ = write!(out, " {}..{}", r.start, r.end);
    }
    out.push('\n');
    for s in &flat.spans {
        let _ = writeln!(
            out,
            "{:?} {}..{} entry={} path={} {}",
            s.kind,
            s.range.start,
            s.range.end,
            s.entry,
            s.path,
            escape(
                flat.text
                    .get(s.range.clone())
                    .unwrap_or("<not a char boundary>")
            )
        );
    }
    out
}

/// Every byte of the text that has a source, as runs where the source is the same text node and
/// the offset goes up by one with each byte
fn dump_sources(flat: &Flat) -> String {
    struct Run {
        start: usize,
        end: usize,
        entry: usize,
        path: String,
        offset: usize,
    }
    let mut runs: Vec<Run> = Vec::new();
    for i in 0..flat.text.len() {
        let Some(p) = flat.source(i) else { continue };
        let path = p.path.to_string();
        if let Some(last) = runs.last_mut()
            && last.end == i
            && last.entry == p.entry
            && last.path == path
            && last.offset + (i - last.start) == p.offset
        {
            last.end = i + 1;
            continue;
        }
        runs.push(Run {
            start: i,
            end: i + 1,
            entry: p.entry,
            path,
            offset: p.offset,
        });
    }
    let mut out = String::new();
    for r in runs {
        let _ = writeln!(
            out,
            "{}..{} entry={} path={} offset={}",
            r.start, r.end, r.entry, r.path, r.offset
        );
    }
    out
}

/// A verse's part as written in a golden file: nothing for the whole verse, else `a` to `z`
fn part_letter(part: u8) -> String {
    match part {
        1..=26 => char::from(b'a' + part - 1).to_string(),
        0 => String::new(),
        _ => format!("?{part}"),
    }
}

fn dump_verses(nodes: &[Node]) -> String {
    let mut out = String::new();
    let spans = verses(nodes);
    let all = TextOptions {
        filter: Filter::all(),
        ..TextOptions::default()
    };
    for s in &spans {
        let text = topos_bible_markup::plain_text(&slice(nodes, &s.start, Some(&s.end)), &all);
        let _ = writeln!(
            out,
            "{}:{}{}-{} marker={} start={} end={} {}",
            s.chapter,
            s.verse,
            part_letter(s.part),
            s.last,
            s.marker,
            s.start,
            s.end,
            escape(&text)
        );
    }
    let _ = writeln!(out, "chapter_verses: {:?}", chapter_verses(&spans));
    out
}

fn dump_refs(nodes: &[Node], data: &BibleData) -> String {
    let mut out = String::new();
    for r in tagged_references(nodes, data) {
        let passages: Vec<String> = r
            .passages
            .iter()
            .map(|p| p.to_osis(data.books()).unwrap_or_else(|| String::from("?")))
            .collect();
        let _ = writeln!(
            out,
            "{} target={} passages=[{}]",
            r.path,
            r.target.as_deref().map_or(String::from("-"), escape),
            passages.join(", ")
        );
    }
    out
}

/// The output sections for a case, in the order a golden file has them
pub fn outputs(markup: Markup, inputs: &[String]) -> Vec<(String, String)> {
    let entries: Vec<Vec<Node>> = inputs
        .iter()
        .map(|i| topos_bible_markup::parse(markup, i))
        .collect();
    let mut out = Vec::new();
    for nodes in &entries {
        out.push((String::from("tree"), dump_tree(nodes)));
    }
    let default = flat(&entries, TextOptions::default());
    out.push((String::from("text"), default.text.clone()));
    out.push((String::from("text-all"), flat(&entries, all_options()).text));
    let nobreaks = TextOptions {
        line_breaks: false,
        ..TextOptions::default()
    };
    out.push((String::from("text-nobreaks"), flat(&entries, nobreaks).text));
    out.push((String::from("spans"), dump_spans(&default)));
    out.push((String::from("sources"), dump_sources(&default)));
    let mut html = HtmlRenderer::new();
    for nodes in &entries {
        out.push((String::from("html"), html.render(nodes)));
    }
    let mut bare = HtmlRenderer::with_options(HtmlOptions {
        class_prefix: "".into(),
        filter: Filter::all(),
    });
    for nodes in &entries {
        out.push((String::from("html-bare"), bare.render(nodes)));
    }
    for nodes in &entries {
        out.push((String::from("verses"), dump_verses(nodes)));
    }
    if markup == Markup::Usx {
        for input in inputs {
            out.push((
                String::from("usx-chapter-verses"),
                format!("{:?}\n", usx_chapter_verses(input)),
            ));
        }
    }
    let data = BibleData::default();
    for nodes in &entries {
        out.push((String::from("refs"), dump_refs(nodes, &data)));
    }
    out
}

/// Whether a line is a section header (`#### name ####`)
pub fn header(line: &str) -> Option<&str> {
    let name = line.strip_prefix("#### ")?.strip_suffix(" ####")?;
    (!name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '-')).then_some(name)
}

/// Reads a golden file's sections, in order
pub fn read(file: &str) -> Vec<(String, String)> {
    let mut lines: Vec<&str> = file.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    let mut sections: Vec<(String, Vec<&str>)> = Vec::new();
    for line in lines {
        match header(line) {
            Some(name) => sections.push((name.to_string(), Vec::new())),
            None => {
                if let Some(last) = sections.last_mut() {
                    last.1.push(line);
                }
            }
        }
    }
    sections
        .into_iter()
        .map(|(name, lines)| (name, lines.join("\n")))
        .collect()
}

/// Writes sections as a golden file
pub fn write(sections: &[(String, String)]) -> String {
    let mut out = String::new();
    for (name, content) in sections {
        let _ = writeln!(out, "#### {name} ####");
        out.push_str(content);
        out.push('\n');
    }
    out
}
