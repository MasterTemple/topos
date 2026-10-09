//! Each markup's rules for what an element is: its kind, milestone, and default style.

use crate::{Element, Kind, Markup, Milestone};

/// Gives a complete element its kind (and milestone and default `type`, where the markup says so)
pub(crate) fn classify(markup: Markup, e: &mut Element) {
    if markup != Markup::Gbf && markup != Markup::Plaintext && e.milestone.is_none() {
        if e.attr("sID").is_some() {
            e.milestone = Some(Milestone::Start);
        } else if e.attr("eID").is_some() {
            e.milestone = Some(Milestone::End);
        }
    }
    let (kind, style) = match markup {
        Markup::Osis => (osis(e), None),
        Markup::Thml => thml(e),
        Markup::Tei => tei(e),
        Markup::Usx => usx(e),
        Markup::Gbf => gbf(&e.name),
        Markup::Plaintext => (
            if e.name == "br" {
                Kind::LineBreak
            } else {
                Kind::Other
            },
            None,
        ),
    };
    e.kind = kind;
    if let Some(style) = style {
        default_type(e, style);
    }
    if markup == Markup::Usx && e.kind == Kind::Title && e.name == "para" {
        let style = e.attr("style").unwrap_or("").to_string();
        default_type(e, &style);
    }
    if markup == Markup::Tei
        && e.kind == Kind::Highlight
        && e.attr("type").is_none()
        && let Some(rend) = e.attr("rend")
    {
        let rend = rend.to_string();
        e.attrs.push((String::from("type"), rend));
    }
}

/// Adds `type` when the element has none (in any case)
fn default_type(e: &mut Element, style: &str) {
    if e.attr("type").is_none() {
        e.attrs.push((String::from("type"), style.to_string()));
    }
}

fn osis(e: &Element) -> Kind {
    match e.name.as_str() {
        "w" => Kind::Word,
        "note" => Kind::Note,
        "reference" => Kind::Reference,
        "title" => Kind::Title,
        "q" => {
            if e.attr("who")
                .is_some_and(|w| w.eq_ignore_ascii_case("jesus"))
            {
                Kind::WordsOfJesus
            } else {
                Kind::Quote
            }
        }
        "transChange" => Kind::Added,
        "divineName" => Kind::DivineName,
        "hi" => Kind::Highlight,
        "foreign" => Kind::Foreign,
        "p" => Kind::Paragraph,
        "div" => match e.attr("type") {
            Some("paragraph" | "x-p") => Kind::Paragraph,
            _ => Kind::Division,
        },
        "lg" => Kind::LineGroup,
        "l" => Kind::Line,
        "lb" => Kind::LineBreak,
        "chapter" => Kind::Chapter,
        "verse" => Kind::Verse,
        "milestone" => Kind::Milestone,
        "figure" => Kind::Image,
        _ => Kind::Other,
    }
}

fn thml(e: &Element) -> (Kind, Option<&'static str>) {
    let name = e.name.to_ascii_lowercase();
    match name.as_str() {
        "scripref" | "a" => (Kind::Reference, None),
        "note" => (Kind::Note, None),
        "i" | "em" | "cite" => (Kind::Highlight, Some("italic")),
        "b" | "strong" => (Kind::Highlight, Some("bold")),
        "u" => (Kind::Highlight, Some("underline")),
        "sup" => (Kind::Highlight, Some("super")),
        "sub" => (Kind::Highlight, Some("sub")),
        "font" | "span" => (Kind::Highlight, None),
        "br" => (Kind::LineBreak, None),
        "p" => (Kind::Paragraph, None),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => (Kind::Title, None),
        "div" => {
            if e.attr("class").is_some_and(|c| c.contains("sechead")) {
                (Kind::Title, None)
            } else {
                (Kind::Division, None)
            }
        }
        "added" => (Kind::Added, None),
        "foreign" => (Kind::Foreign, None),
        "q" | "blockquote" => (Kind::Quote, None),
        "img" => (Kind::Image, None),
        "w" => (Kind::Word, None),
        _ => (Kind::Other, None),
    }
}

fn tei(e: &Element) -> (Kind, Option<&'static str>) {
    match e.name.as_str() {
        "entryFree" | "entry" | "superentry" => (Kind::Entry, None),
        "ref" | "xr" => (Kind::Reference, None),
        "hi" => (Kind::Highlight, None),
        "emph" | "title" => (Kind::Highlight, Some("italic")),
        "lb" => (Kind::LineBreak, None),
        "p" => (Kind::Paragraph, None),
        "note" => (Kind::Note, None),
        "foreign" => (Kind::Foreign, None),
        "q" | "quote" => (Kind::Quote, None),
        "w" => (Kind::Word, None),
        "figure" | "graphic" => (Kind::Image, None),
        _ => (Kind::Other, None),
    }
}

/// A USFM marker without its trailing digits (`q1` → `q`)
fn marker(style: &str) -> &str {
    style.trim_end_matches(|c: char| c.is_ascii_digit())
}

fn usx(e: &Element) -> (Kind, Option<&'static str>) {
    let style = e.attr("style").unwrap_or("");
    match e.name.as_str() {
        "book" => (Kind::Metadata, None),
        "chapter" => (Kind::Chapter, None),
        "verse" => (Kind::Verse, None),
        "note" => (
            Kind::Note,
            Some(match style {
                "x" | "ex" => "crossReference",
                "ef" => "study",
                _ => "footnote",
            }),
        ),
        "ref" => (Kind::Reference, None),
        "figure" => (Kind::Image, None),
        "optbreak" => (Kind::LineBreak, None),
        "para" => match marker(style) {
            "s" | "ms" | "mt" | "mte" | "mr" | "sr" | "r" | "d" | "sp" | "cl" | "cd" | "is"
            | "imt" | "sd" => (Kind::Title, None),
            "q" | "qr" | "qc" | "qm" | "qa" | "qd" => (Kind::Line, None),
            "b" => (Kind::LineBreak, None),
            "rem" | "h" | "toc" | "toca" | "ide" | "sts" | "restore" | "cat" | "usfm" => {
                (Kind::Metadata, None)
            }
            _ => (Kind::Paragraph, None),
        },
        "char" => match marker(style) {
            "wj" => (Kind::WordsOfJesus, None),
            "add" => (Kind::Added, None),
            "nd" => (Kind::DivineName, None),
            "w" => (Kind::Word, None),
            "xt" => (Kind::Reference, None),
            "qt" => (Kind::Quote, None),
            "tl" => (Kind::Foreign, None),
            "it" | "em" | "bk" => (Kind::Highlight, Some("italic")),
            "bd" => (Kind::Highlight, Some("bold")),
            "bdit" => (Kind::Highlight, Some("bold-italic")),
            "sc" => (Kind::Highlight, Some("small-caps")),
            "sup" => (Kind::Highlight, Some("super")),
            "no" => (Kind::Highlight, Some("normal")),
            _ => (Kind::Other, None),
        },
        "table" | "sidebar" | "periph" => (Kind::Division, None),
        "row" => (Kind::Paragraph, None),
        _ => (Kind::Other, None),
    }
}

/// The paired and single GBF codes
fn gbf(name: &str) -> (Kind, Option<&'static str>) {
    match name {
        "FI" => (Kind::Highlight, Some("italic")),
        "FB" => (Kind::Highlight, Some("bold")),
        "FU" => (Kind::Highlight, Some("underline")),
        "FS" => (Kind::Highlight, Some("super")),
        "FV" => (Kind::Highlight, Some("sub")),
        "FC" => (Kind::Highlight, Some("small-caps")),
        "FN" => (Kind::Highlight, None),
        "FR" => (Kind::WordsOfJesus, None),
        "FO" => (Kind::Quote, None),
        "RF" => (Kind::Note, None),
        "RX" => (Kind::Reference, None),
        "TS" | "TT" | "TH" => (Kind::Title, None),
        "PP" => (Kind::LineGroup, None),
        "CM" => (Kind::Paragraph, None),
        "CL" => (Kind::LineBreak, None),
        "w" => (Kind::Word, None),
        _ => (Kind::Other, None),
    }
}
