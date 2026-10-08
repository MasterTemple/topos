#![doc = include_str!("../README.md")]

mod classify;
mod entities;
pub mod filter;
pub mod flatten;
mod gbf;
pub mod html;
pub mod path;
pub mod refs;
pub mod text;
mod tree;
pub mod verses;
mod xml;

pub use entities::decode_entities;
pub use filter::{Action, Filter};
pub use flatten::{Flat, Flattener, Point, Span};
pub use html::{HtmlOptions, HtmlRenderer};
pub use path::NodePath;
pub use text::{TextOptions, plain_text};

use std::fmt;

/// The markup a text is written in
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Markup {
    Osis,
    Thml,
    Gbf,
    Tei,
    /// Paratext's XML form of USFM
    Usx,
    Plaintext,
}

impl Markup {
    /// A markup by name, ignoring case: `osis`, `thml`, `gbf`, `tei`, `usx`, `plaintext` or
    /// `plain`
    pub fn from_name(name: &str) -> Option<Markup> {
        Some(match name.to_ascii_lowercase().as_str() {
            "osis" => Markup::Osis,
            "thml" => Markup::Thml,
            "gbf" => Markup::Gbf,
            "tei" => Markup::Tei,
            "usx" => Markup::Usx,
            "plaintext" | "plain" => Markup::Plaintext,
            _ => return None,
        })
    }
}

/// A node in parsed markup
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Node {
    Text(String),
    Element(Element),
}

impl Node {
    pub fn as_element(&self) -> Option<&Element> {
        match self {
            Node::Element(e) => Some(e),
            Node::Text(_) => None,
        }
    }
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Node::Text(t) => Some(t),
            Node::Element(_) => None,
        }
    }
}

/// What an element means, the same across markup formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Kind {
    /// A word, usually annotated with Strong's numbers (`lemma`, USX `strong`) and morphology
    /// (`morph`)
    Word,
    /// Footnote, study note, or cross-reference note; see [`Element::note_type`]
    Note,
    /// Link to scripture or another entry; see [`Element::reference_target`]
    Reference,
    /// Heading or title; [`Element::title_type`] says which (`psalm`, USX `s1`, ...)
    Title,
    /// Words of Christ ("red letter"): OSIS `<q who="Jesus">`, GBF `<FR>`, USX `wj`
    WordsOfJesus,
    /// Other quotation (OSIS `<q>`, GBF `<FO>`, ThML `<q>`/`<blockquote>`, USX `qt`)
    Quote,
    /// Translator additions (KJV italics): OSIS `<transChange>`, USX `add`
    Added,
    /// LORD in small caps: OSIS `<divineName>`, USX `nd`
    DivineName,
    /**
    Visual emphasis; the style is normalised to `italic`, `bold`, `underline`, `super`, `sub`,
    `small-caps`, ... (see [`Element::style`])
    */
    Highlight,
    Foreign,
    Paragraph,
    LineGroup,
    /// A poetic line (OSIS `<l>`, USX `q1`)
    Line,
    LineBreak,
    /// A generic division (`<div>`) that is not a paragraph
    Division,
    /// A chapter (usually a milestone)
    Chapter,
    /// A verse (a milestone in USX and some OSIS)
    Verse,
    /// OSIS `<milestone>` (e.g. `type="x-p"` pilcrow, `type="pb"`)
    Milestone,
    Image,
    /// Dictionary entry container (TEI `<entryFree>`/`<entry>`)
    Entry,
    /// Content that isn't for reading, like USX's `<book>` identification line
    Metadata,
    /// Anything else; inspect [`Element::name`] and attributes
    Other,
}

impl Kind {
    /// The class name the HTML renderer gives this kind (before the prefix)
    pub fn class_name(self) -> &'static str {
        match self {
            Kind::Word => "w",
            Kind::Note => "note",
            Kind::Reference => "ref",
            Kind::Title => "title",
            Kind::WordsOfJesus => "woj",
            Kind::Quote => "quote",
            Kind::Added => "added",
            Kind::DivineName => "divine-name",
            Kind::Highlight => "hi",
            Kind::Foreign => "foreign",
            Kind::Paragraph => "p",
            Kind::LineGroup => "lg",
            Kind::Line => "l",
            Kind::LineBreak => "lb",
            Kind::Division => "div",
            Kind::Chapter => "chapter",
            Kind::Verse => "verse",
            Kind::Milestone => "milestone",
            Kind::Image => "image",
            Kind::Entry => "entry",
            Kind::Metadata => "metadata",
            Kind::Other => "other",
        }
    }
}

/// Which half of a split element this is
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Milestone {
    /// `sID` milestone: the element starts here and continues past this point
    Start,
    /// `eID` milestone, or an end tag whose start was in an earlier entry
    End,
}

/// An element of parsed markup
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Element {
    pub kind: Kind,
    /// Original tag name (GBF: the token code such as `FR`, `RF`, `CM`; ThML/GBF Strong's-tagged
    /// words are synthesised as `w`; USX `para`/`char` elements keep their tag, and their
    /// `style` attribute says which)
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    pub milestone: Option<Milestone>,
    /// Opened in this entry but not closed before its end
    pub unclosed: bool,
}

impl Element {
    pub(crate) fn new(name: impl Into<String>, attrs: Vec<(String, String)>) -> Element {
        Element {
            kind: Kind::Other,
            name: name.into(),
            attrs,
            children: Vec::new(),
            milestone: None,
            unclosed: false,
        }
    }

    /// Attribute value by name (exact match first, then ASCII case-insensitive)
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .or_else(|| {
                self.attrs
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(name))
            })
            .map(|(_, v)| v.as_str())
    }

    pub(crate) fn set_attr(&mut self, name: &str, value: String) {
        if let Some(a) = self.attrs.iter_mut().find(|(k, _)| k == name) {
            a.1 = value;
        } else {
            self.attrs.push((name.to_string(), value));
        }
    }

    /// Concatenated text of all descendants (notes included)
    pub fn text(&self) -> String {
        let mut s = String::new();
        collect_text(&self.children, &mut s);
        s
    }

    /// Strong's numbers from `lemma` (OSIS `strong:H07225`, synthesised for ThML and GBF) and
    /// USX's `strong`
    pub fn strongs(&self) -> Vec<Strongs> {
        let mut out: Vec<Strongs> = self
            .attr("lemma")
            .map(Strongs::parse_list)
            .unwrap_or_default();
        if let Some(strong) = self.attr("strong") {
            out.extend(strong.split([' ', ',']).filter_map(Strongs::parse));
        }
        out
    }

    /// Morphology codes from `morph`, e.g. `robinson:V-AAI-3S`; the scheme prefix is kept
    pub fn morph(&self) -> Vec<&str> {
        self.attr("morph")
            .map(|m| m.split_whitespace().collect())
            .unwrap_or_default()
    }

    /// For notes: `type` (`crossReference`, `explanation`, `study`, `translation`, `footnote`, ...)
    pub fn note_type(&self) -> Option<&str> {
        self.attr("type")
    }

    /// Whether this note is a list of cross references
    pub fn is_cross_reference(&self) -> bool {
        self.kind == Kind::Note
            && self
                .note_type()
                .is_some_and(|t| t.eq_ignore_ascii_case("crossReference"))
    }

    /// For titles: `type` (OSIS `psalm`, `chapter`, ...; USX's paragraph style, like `s1`)
    pub fn title_type(&self) -> Option<&str> {
        self.attr("type")
    }

    /// Link target: `osisRef`, ThML `passage`, TEI `target`, USX `loc` or `link-href`, or `href`
    pub fn reference_target(&self) -> Option<&str> {
        ["osisRef", "passage", "target", "loc", "link-href", "href"]
            .into_iter()
            .find_map(|name| self.attr(name))
    }

    /// Normalised highlight style (`italic`, `bold`, ...) for [`Kind::Highlight`]
    pub fn style(&self) -> Option<&str> {
        self.attr("type").or_else(|| self.attr("rend"))
    }

    /// This element and all descendant elements, depth-first (pre-order)
    pub fn descendants(&self) -> Vec<&Element> {
        let mut out = vec![self];
        walk(&self.children, &mut |e| out.push(e));
        out
    }
}

fn collect_text(nodes: &[Node], out: &mut String) {
    for n in nodes {
        match n {
            Node::Text(t) => out.push_str(t),
            Node::Element(e) => collect_text(&e.children, out),
        }
    }
}

/// Visit every element in a node list depth-first (pre-order)
pub fn walk<'a>(nodes: &'a [Node], f: &mut impl FnMut(&'a Element)) {
    for n in nodes {
        if let Node::Element(e) = n {
            f(e);
            walk(&e.children, f);
        }
    }
}

/// All elements of a kind, depth-first
pub fn find_all(nodes: &[Node], kind: Kind) -> Vec<&Element> {
    let mut out = Vec::new();
    walk(nodes, &mut |e| {
        if e.kind == kind {
            out.push(e)
        }
    });
    out
}

/// All Strong's numbers in a node list, in reading order (duplicates kept)
pub fn strongs_numbers(nodes: &[Node]) -> Vec<Strongs> {
    let mut out = Vec::new();
    walk(nodes, &mut |e| {
        if e.kind == Kind::Word {
            out.extend(e.strongs())
        }
    });
    out
}

/**
Parses text in the given markup; this never fails
- Fragments are fine: an end tag without a start becomes a [`Milestone::End`] element, and a
  start tag still open at the end is closed and marked [`Element::unclosed`]
- Text that can't be read as XML from some point on is kept as text
- In plain text, line breaks become [`Kind::LineBreak`] elements
*/
pub fn parse(markup: Markup, text: &str) -> Vec<Node> {
    match markup {
        Markup::Plaintext => plaintext(text),
        Markup::Gbf => gbf::parse(text),
        Markup::Osis | Markup::Thml | Markup::Tei | Markup::Usx => xml::parse(markup, text),
    }
}

fn plaintext(text: &str) -> Vec<Node> {
    let mut nodes = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            let mut br = Element::new("br", Vec::new());
            br.kind = Kind::LineBreak;
            nodes.push(Node::Element(br));
        }
        let line = line.strip_suffix('\r').unwrap_or(line);
        if !line.is_empty() {
            nodes.push(Node::Text(line.to_string()));
        }
    }
    nodes
}

/// Strong's language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StrongsLang {
    Hebrew,
    Greek,
}

/// A Strong's number such as `H7225` or `G25`, with an optional suffix (`H0430a` → `a`)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Strongs {
    pub lang: StrongsLang,
    pub number: u32,
    pub suffix: Option<String>,
}

impl Strongs {
    /// Parses one token: `strong:H07225`, `x-Strongs:G25`, `G25`, `H0430a`
    pub fn parse(token: &str) -> Option<Strongs> {
        let t = token
            .strip_prefix("strong:")
            .or_else(|| token.strip_prefix("x-Strongs:"))
            .or_else(|| token.strip_prefix("Strongs:"))
            .unwrap_or(token);
        let mut chars = t.chars();
        let lang = match chars.next()? {
            'G' | 'g' => StrongsLang::Greek,
            'H' | 'h' => StrongsLang::Hebrew,
            _ => return None,
        };
        let rest = chars.as_str();
        let digits = rest.chars().take_while(char::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        let number = rest[..digits].parse().ok()?;
        let suffix = &rest[digits..];
        if !suffix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '!')
        {
            return None;
        }
        Some(Strongs {
            lang,
            number,
            suffix: (!suffix.is_empty()).then(|| suffix.to_string()),
        })
    }

    /// Parses a whitespace-separated `lemma` attribute, keeping only Strong's tokens
    pub fn parse_list(lemma: &str) -> Vec<Strongs> {
        lemma
            .split_whitespace()
            .filter_map(Strongs::parse)
            .collect()
    }

    /// Key for looking the number up in SWORD's StrongsGreek/StrongsHebrew modules
    pub fn dictionary_key(&self) -> String {
        format!("{:05}", self.number)
    }

    /// Conventional dictionary module name: `StrongsGreek` / `StrongsHebrew`
    pub fn dictionary_module(&self) -> &'static str {
        match self.lang {
            StrongsLang::Greek => "StrongsGreek",
            StrongsLang::Hebrew => "StrongsHebrew",
        }
    }
}

impl fmt::Display for Strongs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let l = match self.lang {
            StrongsLang::Greek => 'G',
            StrongsLang::Hebrew => 'H',
        };
        write!(f, "{l}{}", self.number)?;
        if let Some(s) = &self.suffix {
            write!(f, "{s}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
