//! Text fragments (`#:~:text=`) for positions in HTML, and positions for text fragments: the
//! location the HTML format reports. Formerly the `htmloc` crate
//! (<https://github.com/MasterTemple/htmloc>).

use line_col::LineColLookup;
use regex::Regex;
use urlencoding::{decode, encode};

const MAX_EXACT_MATCH_LENGTH: usize = 300;
const MIN_CONTEXT_WORDS: usize = 3;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ByteIndex {
    pub start: usize,
    pub end: usize,
}

impl<'h> From<regex::Match<'h>> for ByteIndex {
    fn from(value: regex::Match<'h>) -> Self {
        ByteIndex::new(value.start(), value.end())
    }
}
impl ByteIndex {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,   // 1-indexed
    pub column: usize, // 1-indexed
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start: Position,
    pub end: Position,
    pub bytes: ByteIndex,
}

impl Selection {
    pub fn new(start: Position, end: Position, bytes: ByteIndex) -> Self {
        Self { start, end, bytes }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GenerateOptions {
    pub min_prefix_words: usize,
    pub min_suffix_words: usize,
}

impl GenerateOptions {
    pub fn new(min_prefix_words: usize, min_suffix_words: usize) -> Self {
        Self {
            min_prefix_words,
            min_suffix_words,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFragment {
    pub prefix: Option<String>,
    pub text_start: String,
    pub text_end: Option<String>,
    pub suffix: Option<String>,
}

impl TextFragment {
    /// Serializes the fragment into the standard URL hash format
    pub fn to_hash_string(&self) -> String {
        let mut parts = Vec::new();
        if let Some(prefix) = &self.prefix {
            parts.push(format!("{}-", encode(prefix)));
        }
        parts.push(encode(&self.text_start).into_owned());
        if let Some(text_end) = &self.text_end {
            parts.push(encode(text_end).into_owned());
        }
        if let Some(suffix) = &self.suffix {
            parts.push(format!("-{}", encode(suffix)));
        }
        format!("#:~:text={}", parts.join(","))
    }

    /// Parses a Text Fragment from a URL hash string (e.g., "#:~:text=prefix-,start,end,-suffix")
    pub fn from_hash_string(hash: &str) -> Option<Self> {
        // Extract the payload after "#:~:text=" or "~:text="
        let payload = hash.split("~:text=").last().unwrap_or(hash);
        let payload = payload.split('&').next().unwrap_or(payload); // Ignore other hash params

        let parts: Vec<&str> = payload.split(',').collect();
        if parts.is_empty() {
            return None;
        }

        let mut prefix = None;

        let mut idx = 0;

        // 1. Check for prefix (ends with '-')
        if parts[idx].ends_with('-') {
            let p = &parts[idx][..parts[idx].len() - 1];
            prefix = Some(decode(p).ok()?.into_owned());
            idx += 1;
        }

        if idx >= parts.len() {
            return None;
        }

        // 2. Parse text_start
        let text_start = decode(parts[idx]).ok()?.into_owned();
        idx += 1;

        // 3. Check for text_end and suffix
        let mut suffix = None;
        let mut text_end = None;

        if idx < parts.len() {
            if parts[idx].starts_with('-') {
                let s = &parts[idx][1..];
                suffix = Some(decode(s).ok()?.into_owned());
            } else {
                text_end = Some(decode(parts[idx]).ok()?.into_owned());
                idx += 1;
                // If there's still a part left, it must be the suffix
                if idx < parts.len() && parts[idx].starts_with('-') {
                    let s = &parts[idx][1..];
                    suffix = Some(decode(s).ok()?.into_owned());
                }
            }
        }

        Some(TextFragment {
            prefix,
            text_start,
            text_end,
            suffix,
        })
    }
}

pub enum SourceType<'a> {
    PlainText(&'a str),
    HTML(&'a str),
}

pub struct Document<'a> {
    source: &'a str,
    plain_text: String,
    lookup: LineColLookup<'a>,
    source_to_plain_map: Option<Vec<usize>>,
    plain_to_source_map: Option<Vec<usize>>,
}

impl<'a> Document<'a> {
    pub fn source(&'a self) -> SourceType<'a> {
        if self.plain_text.len() == self.source.len() {
            SourceType::PlainText(&self.plain_text)
        } else {
            SourceType::HTML(self.source)
        }
    }

    pub fn plain_text(&'a self) -> &'a str {
        &self.plain_text
    }

    pub fn from_plain_text(text: &'a str) -> Self {
        Self {
            source: text,
            plain_text: text.to_string(),
            lookup: LineColLookup::new(text),
            source_to_plain_map: None,
            plain_to_source_map: None,
        }
    }

    pub fn from_html(html: &'a str) -> Self {
        let (plain_text, s2p, p2s) = Self::strip_html_and_map(html);
        Self {
            source: html,
            plain_text,
            lookup: LineColLookup::new(html),
            source_to_plain_map: Some(s2p),
            plain_to_source_map: Some(p2s),
        }
    }

    /**
    The text a browser shows, with maps from source bytes to text bytes and back
    - Tags are left out, and so is the content of `<script>` and `<style>`
    - Character references (`&amp;`, `&#8212;`, `&#x2014;`, `&nbsp;`, ...) are decoded
    - Block elements (`<p>`, `<div>`, `<br>`, `<li>`, headings, ...) start a new line, so words
      on either side don't run together
    */
    fn strip_html_and_map(html: &str) -> (String, Vec<usize>, Vec<usize>) {
        let mut plain_text = String::with_capacity(html.len());
        let mut source_to_plain_map = vec![0; html.len() + 1];
        let mut plain_to_source_map = Vec::with_capacity(html.len());
        // The end tag that ends `<script>` or `<style>` content
        let mut skip_until: Option<&str> = None;
        let mut i = 0;
        while i < html.len() {
            let rest = &html[i..];
            if let Some(close) = skip_until {
                let bytes = rest.as_bytes();
                if bytes.len() >= close.len()
                    && bytes[..close.len()].eq_ignore_ascii_case(close.as_bytes())
                {
                    skip_until = None;
                } else {
                    let len = rest.chars().next().map_or(1, char::len_utf8);
                    source_to_plain_map[i..i + len].fill(plain_text.len());
                    i += len;
                    continue;
                }
            }
            if rest.starts_with('<') {
                let len = rest.find('>').map_or(rest.len(), |end| end + 1);
                let tag = &rest[..len];
                source_to_plain_map[i..i + len].fill(plain_text.len());
                let name = tag_name(tag);
                if !tag.starts_with("</") && !tag.ends_with("/>") {
                    skip_until = match name.as_str() {
                        "script" => Some("</script"),
                        "style" => Some("</style"),
                        _ => None,
                    };
                }
                if BLOCK_TAGS.contains(&name.as_str())
                    && !plain_text.is_empty()
                    && !plain_text.ends_with(char::is_whitespace)
                {
                    plain_to_source_map.push(i);
                    plain_text.push('\n');
                }
                i += len;
                continue;
            }
            let (c, len) = match character_reference(rest) {
                Some(decoded) => decoded,
                None => match rest.chars().next() {
                    Some(c) => (c, c.len_utf8()),
                    None => break,
                },
            };
            source_to_plain_map[i..i + len].fill(plain_text.len());
            for byte_offset in 0..c.len_utf8() {
                // A decoded reference maps every byte to its `&`
                plain_to_source_map.push(if len == c.len_utf8() {
                    i + byte_offset
                } else {
                    i
                });
            }
            plain_text.push(c);
            i += len;
        }

        source_to_plain_map[html.len()] = plain_text.len();
        (plain_text, source_to_plain_map, plain_to_source_map)
    }

    /// The source byte where a plain-text byte comes from
    pub fn source_byte(&self, plain_text_byte: usize) -> Option<usize> {
        match &self.plain_to_source_map {
            Some(map) => map.get(plain_text_byte).copied(),
            None => (plain_text_byte <= self.source.len()).then_some(plain_text_byte),
        }
    }

    /**
    The source byte just after the plain text that ends at `plain_text_end_byte`, before any
    tags that follow it
    */
    pub fn source_end_byte(&self, plain_text_end_byte: usize) -> Option<usize> {
        if plain_text_end_byte == 0 {
            return Some(0);
        }
        let last_char = self.plain_text.get(..plain_text_end_byte)?.chars().last()?;
        let start = self.source_byte(plain_text_end_byte - last_char.len_utf8())?;
        // A decoded character reference (`&amp;`) ends at its `;`
        let len = match self.source.get(start..).and_then(character_reference) {
            Some((_, len)) => len,
            None => last_char.len_utf8(),
        };
        Some(start + len)
    }

    pub fn resolve_to_plain_text_offset(&self, pos: &Position) -> Option<usize> {
        let mut current_line = 1;
        let mut line_start_byte = 0;

        for (i, c) in self.source.char_indices() {
            if current_line == pos.line {
                line_start_byte = i;
                break;
            }
            if c == '\n' {
                current_line += 1;
            }
        }

        let source_byte = line_start_byte
            + self.source[line_start_byte..]
                .chars()
                .take(pos.column - 1)
                .map(|c| c.len_utf8())
                .sum::<usize>();

        if let Some(map) = &self.source_to_plain_map {
            map.get(source_byte).copied()
        } else {
            Some(source_byte)
        }
    }

    /// Converts a plain text start byte offset back into a line-column pair
    pub fn resolve_start_to_source_position(&self, plain_text_byte: usize) -> Option<Position> {
        let (line, col) = self.lookup.get(self.source_byte(plain_text_byte)?);
        Some(Position { line, column: col })
    }

    /// Converts a plain text end byte offset back into a line-column pair,
    /// explicitly excluding trailing HTML tags from the boundary.
    pub fn resolve_end_to_source_position(&self, plain_text_end_byte: usize) -> Option<Position> {
        let (line, col) = self.lookup.get(self.source_end_byte(plain_text_end_byte)?);
        Some(Position { line, column: col })
    }
}

/// Elements a browser shows on their own lines
const BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "br",
    "dd",
    "div",
    "dl",
    "dt",
    "figcaption",
    "figure",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "td",
    "th",
    "tr",
    "ul",
];

/// A tag's name in lowercase: `<div class="x">` and `</div>` are `div`
fn tag_name(tag: &str) -> String {
    tag.trim_start_matches('<')
        .trim_start_matches('/')
        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// A character reference at the start of `text` (`&amp;`, `&#8212;`, `&#x2014;`) and its length
fn character_reference(text: &str) -> Option<(char, usize)> {
    let body = text.strip_prefix('&')?;
    let semi = body.char_indices().take(12).find(|&(_, c)| c == ';')?.0;
    let name = &body[..semi];
    let c = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => '\u{a0}',
        _ => {
            let number = name.strip_prefix('#')?;
            let code = match number.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => number.parse().ok()?,
            };
            char::from_u32(code)?
        }
    };
    Some((c, semi + 2))
}

pub struct FragmentEngine<'a> {
    doc: Document<'a>,
}

impl<'a> FragmentEngine<'a> {
    pub fn new(doc: Document<'a>) -> Self {
        Self { doc }
    }

    pub fn from_html(html: &'a str) -> Self {
        Self::new(Document::from_html(html))
    }

    pub fn from_plain_text(text: &'a str) -> Self {
        Self::new(Document::from_plain_text(text))
    }

    pub fn plain_text(&'a self) -> &'a str {
        self.doc.plain_text()
    }

    pub fn source(&'a self) -> SourceType<'a> {
        self.doc.source()
    }

    pub fn doc(&'a self) -> &'a Document<'a> {
        &self.doc
    }

    /// Generates a fragment, with an optional robustness configuration to enforce prefix/suffix inclusion.
    pub fn generate(
        &self,
        selection: Selection,
        options: Option<GenerateOptions>,
    ) -> Option<TextFragment> {
        let start_byte = self.doc.resolve_to_plain_text_offset(&selection.start)?;
        let end_byte = self.doc.resolve_to_plain_text_offset(&selection.end)?;
        self.generate_for_plain_text(start_byte, end_byte, options)
    }

    /// A text fragment for a byte range of the plain text (as searching
    /// [`plain_text`](Self::plain_text) gives)
    pub fn generate_for_plain_text(
        &self,
        start_byte: usize,
        end_byte: usize,
        options: Option<GenerateOptions>,
    ) -> Option<TextFragment> {
        let opts = options.unwrap_or_default();

        if start_byte >= end_byte || end_byte > self.doc.plain_text.len() {
            return None;
        }

        let selected_text = &self.doc.plain_text[start_byte..end_byte];
        let cleaned_text = self.clean_text(selected_text);
        if cleaned_text.is_empty() {
            return None;
        }

        let (text_start, text_end) = self.determine_start_end(&cleaned_text);

        // Context calculation handles both uniqueness AND the robustness minimums
        let (prefix, suffix) = self.calculate_context(
            start_byte,
            end_byte,
            &text_start,
            text_end.as_deref(),
            &opts,
        );

        Some(TextFragment {
            prefix,
            text_start,
            text_end,
            suffix,
        })
    }

    fn clean_text(&self, text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn determine_start_end(&self, text: &str) -> (String, Option<String>) {
        if text.len() <= MAX_EXACT_MATCH_LENGTH {
            return (text.to_string(), None);
        }
        let words: Vec<&str> = text.split_whitespace().collect();
        let start_words = words
            .iter()
            .take(MIN_CONTEXT_WORDS)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        let end_words = words
            .iter()
            .rev()
            .take(MIN_CONTEXT_WORDS)
            .rev()
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        (start_words, Some(end_words))
    }

    fn calculate_context(
        &self,
        start_byte: usize,
        end_byte: usize,
        text_start: &str,
        // TODO: Why is this not used?
        _text_end: Option<&str>,
        opts: &GenerateOptions,
    ) -> (Option<String>, Option<String>) {
        let before_text = &self.doc.plain_text[..start_byte];
        let after_text = &self.doc.plain_text[end_byte..];

        let mut before_words: Vec<&str> = before_text.split_whitespace().rev().collect();
        let mut after_words: Vec<&str> = after_text.split_whitespace().collect();

        let mut prefix_words = Vec::new();
        let mut suffix_words = Vec::new();
        let mut is_unique = false;

        // Loop until unique AND robustness minimums are met
        while (!is_unique
            || prefix_words.len() < opts.min_prefix_words
            || suffix_words.len() < opts.min_suffix_words)
            && (!before_words.is_empty() || !after_words.is_empty())
        {
            let taken = prefix_words.len() + suffix_words.len();
            if let Some(w) = before_words.first()
                && (prefix_words.len() < opts.min_prefix_words
                    || (!is_unique && prefix_words.len() <= suffix_words.len()))
            {
                prefix_words.insert(0, *w);
                before_words.remove(0);
            }
            if let Some(w) = after_words.first()
                && (suffix_words.len() < opts.min_suffix_words
                    || (!is_unique && suffix_words.len() < prefix_words.len()))
            {
                suffix_words.push(*w);
                after_words.remove(0);
            }
            if prefix_words.len() + suffix_words.len() == taken {
                // One side has run out, so the other has to grow (this used to loop forever when
                // a match near the end of a document needed more context)
                if !before_words.is_empty() {
                    prefix_words.insert(0, before_words.remove(0));
                } else if !after_words.is_empty() {
                    suffix_words.push(after_words.remove(0));
                }
            }

            let p = prefix_words.join(" ");
            let _s = suffix_words.join(" ");

            // Check uniqueness if we aren't already unique
            if !is_unique {
                let pattern = format!("{} {}", p, text_start).trim().to_string();
                is_unique = self
                    .clean_text(&self.doc.plain_text)
                    .matches(&pattern)
                    .count()
                    == 1;
            }
        }

        (
            if prefix_words.is_empty() {
                None
            } else {
                Some(prefix_words.join(" "))
            },
            if suffix_words.is_empty() {
                None
            } else {
                Some(suffix_words.join(" "))
            },
        )
    }

    /// Converts text into a regex pattern that matches any sequence of whitespace (handling newlines/tabs).
    fn build_ws_regex(text: &str) -> String {
        text.split_whitespace()
            .map(regex::escape)
            .collect::<Vec<_>>()
            .join(r"\s+")
    }

    /// Resolves a Text Fragment back into a Line-Column Selection using robust regex whitespace folding.
    pub fn resolve_fragment(&self, fragment: &TextFragment) -> Option<Selection> {
        let mut pattern = String::new();

        // 1. Prefix group (optional match, but ensures context if present)
        if let Some(p) = &fragment.prefix {
            // Context can touch the match (`(John 3:16)`), as the Text Fragments spec allows
            pattern.push_str(&format!("(?P<prefix>{}\\s*)", Self::build_ws_regex(p)));
        }

        // 2. Core group (This captures the actual bounds of the returned text)
        let start_rx = Self::build_ws_regex(&fragment.text_start);
        if let Some(e) = &fragment.text_end {
            let end_rx = Self::build_ws_regex(e);
            // (?s:.*?) allows matching across newlines non-greedily between start and end
            pattern.push_str(&format!("(?P<core>{}(?s:.*?){})", start_rx, end_rx));
        } else {
            pattern.push_str(&format!("(?P<core>{})", start_rx));
        }

        // 3. Suffix group
        if let Some(s) = &fragment.suffix {
            pattern.push_str(&format!("(?P<suffix>\\s*{})", Self::build_ws_regex(s)));
        }

        // Compile and find the first match
        let re = Regex::new(&pattern).ok()?;
        let captures = re.captures(&self.doc.plain_text)?;

        // Extract the exact bounds of the 'core' match (excluding prefix/suffix spaces)
        let core_match = captures.name("core")?;

        let start_pos = self
            .doc
            .resolve_start_to_source_position(core_match.start())?;
        let end_pos = self.doc.resolve_end_to_source_position(core_match.end())?;

        // Where the text is in the HTML
        let bytes = ByteIndex::new(
            self.doc.source_byte(core_match.start())?,
            self.doc.source_end_byte(core_match.end())?,
        );

        Some(Selection {
            start: start_pos,
            end: end_pos,
            bytes,
        })
    }
}
