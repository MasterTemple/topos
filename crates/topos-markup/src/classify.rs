//! Maps format-specific tags to the shared [`Kind`]s.

use super::{Element, Kind, Milestone};

fn milestone_attrs(e: &mut Element) {
    if e.milestone.is_none() {
        if e.attr("sID").is_some() {
            e.milestone = Some(Milestone::Start);
        } else if e.attr("eID").is_some() {
            e.milestone = Some(Milestone::End);
        }
    }
}

fn set_style(e: &mut Element, style: &str) {
    if e.attr("type").is_none() {
        e.set_attr("type", style.to_string());
    }
}

pub(crate) fn osis(e: &mut Element) {
    milestone_attrs(e);
    e.kind = match e.name.as_str() {
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
    };
}

pub(crate) fn thml(e: &mut Element) {
    milestone_attrs(e);
    let lname = e.name.to_ascii_lowercase();
    e.kind = match lname.as_str() {
        "scripref" => Kind::Reference,
        "a" => Kind::Reference,
        "note" => Kind::Note,
        "i" | "em" | "cite" => {
            set_style(e, "italic");
            Kind::Highlight
        }
        "b" | "strong" => {
            set_style(e, "bold");
            Kind::Highlight
        }
        "u" => {
            set_style(e, "underline");
            Kind::Highlight
        }
        "sup" => {
            set_style(e, "super");
            Kind::Highlight
        }
        "sub" => {
            set_style(e, "sub");
            Kind::Highlight
        }
        "font" | "span" => Kind::Highlight,
        "br" => Kind::LineBreak,
        "p" => Kind::Paragraph,
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Kind::Title,
        "div" => {
            if e.attr("class").is_some_and(|c| c.contains("sechead")) {
                Kind::Title
            } else {
                Kind::Division
            }
        }
        "added" => Kind::Added,
        "foreign" => Kind::Foreign,
        "q" | "blockquote" => Kind::Quote,
        "img" => Kind::Image,
        "w" => Kind::Word,
        _ => Kind::Other,
    };
}

pub(crate) fn tei(e: &mut Element) {
    milestone_attrs(e);
    e.kind = match e.name.as_str() {
        "entryFree" | "entry" | "superentry" => Kind::Entry,
        "ref" | "xr" => Kind::Reference,
        "hi" => Kind::Highlight,
        "emph" => {
            set_style(e, "italic");
            Kind::Highlight
        }
        "lb" => Kind::LineBreak,
        "p" => Kind::Paragraph,
        "note" => Kind::Note,
        "foreign" => Kind::Foreign,
        "q" | "quote" => Kind::Quote,
        "w" => Kind::Word,
        "figure" | "graphic" => Kind::Image,
        "title" => {
            set_style(e, "italic");
            Kind::Highlight
        }
        _ => Kind::Other,
    };
    if e.kind == Kind::Highlight
        && e.attr("type").is_none()
        && let Some(r) = e.attr("rend").map(str::to_string)
    {
        e.set_attr("type", r);
    }
}

/// GBF: names are token codes; paired tokens are opened by `XY` and closed by `Xy`.
pub(crate) fn gbf(e: &mut Element) {
    e.kind = match e.name.as_str() {
        "FI" => {
            set_style(e, "italic");
            Kind::Highlight
        }
        "FB" => {
            set_style(e, "bold");
            Kind::Highlight
        }
        "FU" => {
            set_style(e, "underline");
            Kind::Highlight
        }
        "FS" => {
            set_style(e, "super");
            Kind::Highlight
        }
        "FV" => {
            set_style(e, "sub");
            Kind::Highlight
        }
        "FC" => {
            set_style(e, "small-caps");
            Kind::Highlight
        }
        "FN" => Kind::Highlight,
        "FR" => Kind::WordsOfJesus,
        "FO" => Kind::Quote,
        "RF" => Kind::Note,
        "RX" => Kind::Reference,
        "TS" | "TT" | "TH" => Kind::Title,
        "PP" => Kind::LineGroup,
        "CM" => Kind::Paragraph,
        "CL" => Kind::LineBreak,
        "w" => Kind::Word,
        _ => Kind::Other,
    };
}

/// USX: `para` and `char` say what they are with `style` (USFM markers)
pub(crate) fn usx(e: &mut Element) {
    milestone_attrs(e);
    let style = e.attr("style").unwrap_or("").to_string();
    // `q1`, `s2`, `mt1` are their marker with a level
    let marker = style.trim_end_matches(|c: char| c.is_ascii_digit());
    e.kind = match e.name.as_str() {
        "book" => Kind::Metadata,
        "chapter" => Kind::Chapter,
        "verse" => Kind::Verse,
        "note" => {
            let kind = match style.as_str() {
                "x" | "ex" => "crossReference",
                "ef" => "study",
                _ => "footnote",
            };
            set_style(e, kind);
            Kind::Note
        }
        "ref" => Kind::Reference,
        "figure" => Kind::Image,
        "optbreak" => Kind::LineBreak,
        "para" => match marker {
            "s" | "ms" | "mt" | "mte" | "mr" | "sr" | "r" | "d" | "sp" | "cl" | "cd" | "is"
            | "imt" | "sd" => {
                set_style(e, &style);
                Kind::Title
            }
            "q" | "qr" | "qc" | "qm" | "qa" | "qd" => Kind::Line,
            "b" => Kind::LineBreak,
            "rem" | "h" | "toc" | "toca" | "ide" | "sts" | "restore" | "cat" | "usfm" => {
                Kind::Metadata
            }
            _ => Kind::Paragraph,
        },
        "char" => match marker {
            "wj" => Kind::WordsOfJesus,
            "add" => Kind::Added,
            "nd" => Kind::DivineName,
            "w" => Kind::Word,
            "xt" => Kind::Reference,
            "qt" => Kind::Quote,
            "tl" => Kind::Foreign,
            "it" | "em" | "bk" => {
                set_style(e, "italic");
                Kind::Highlight
            }
            "bd" => {
                set_style(e, "bold");
                Kind::Highlight
            }
            "bdit" => {
                set_style(e, "bold-italic");
                Kind::Highlight
            }
            "sc" => {
                set_style(e, "small-caps");
                Kind::Highlight
            }
            "sup" => {
                set_style(e, "super");
                Kind::Highlight
            }
            "no" => {
                set_style(e, "normal");
                Kind::Highlight
            }
            _ => Kind::Other,
        },
        "table" | "sidebar" | "periph" => Kind::Division,
        "row" => Kind::Paragraph,
        _ => Kind::Other,
    };
}
