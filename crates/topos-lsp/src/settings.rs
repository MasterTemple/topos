//! Settings: the CLI's `config.toml` (`~/.config/topos/config.toml`), then the editor's settings
//! (initialization options, then `workspace/didChangeConfiguration`), each overriding the last.
//!
//! Keys are the CLI's option names (`psg-fmt`, `merge-data`, ...; `psg_fmt` works too), so one
//! config serves both. Keys the server has no use for (`mode`, `cache`, ...) are ignored.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value};
use topos_bible::{
    data::{
        bible_data::{BibleData, BibleDataInput},
        patch::DataPatch,
    },
    filter::bible_filter::BibleFilter,
    matcher::{BibleMatcher, context::BookContext},
    segments::formatter::{BookStyle, FormatOptions},
};

use topos_bible_markup::Kind;

use crate::bible::Bible;

/// What inlay hints show after each reference
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InlayHints {
    /// The reference as formatted, where it is written differently (`jn 3:16` → `John 3:16`)
    #[default]
    Changed,
    /// The reference as formatted, everywhere
    Always,
    /// The OSIS id (`John.3.16`)
    Osis,
    /// The USFM reference (`JHN 3:16`)
    Usfm,
    Never,
}

/// The diagnostic on each reference (alongside the warnings for ones that don't exist)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReferenceDiagnostics {
    /// An information diagnostic with the reference as formatted: `John 3:16`
    #[default]
    #[serde(alias = "information")]
    Info,
    /// The same, as a hint (editors show these more quietly)
    Hint,
    Never,
}

/// A line in the hover, in the order the `hover` setting lists them
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HoverField {
    /// The reference as written in the text: `jn 3:16`
    Written,
    /// With the book's full name: `John 3:16`
    Name,
    /// With the book's abbreviation: `Jn 3:16`
    #[serde(alias = "abbrev")]
    Abbreviation,
    /// The OSIS id: `John.3.16`
    Osis,
    /// The USFM reference: `JHN 3:16`
    Usfm,
    /// The book: its name, its number, and how many chapters it has
    Book,
    /// Old or New Testament
    Testament,
    /// The genres the book is in: `Gospels, Gospels And Acts`
    #[serde(alias = "genre")]
    Genres,
    /// How many verses the reference covers, and which: `36 (3:1-36)`
    Verses,
    /// The passage's text, from the `bible` setting
    Text,
    /// Integer keys (`book * 1_000_000 + chapter * 1_000 + verse`): `43003016`
    Bcv,
    /// Where the reference is in the document: line and columns
    Location,
}

impl HoverField {
    /// What the hover shows by default
    pub const DEFAULT: [HoverField; 9] = [
        HoverField::Text,
        HoverField::Name,
        HoverField::Abbreviation,
        HoverField::Osis,
        HoverField::Book,
        HoverField::Testament,
        HoverField::Genres,
        HoverField::Verses,
        HoverField::Location,
    ];
}

/**
How Bible text is written (hover, whole books, and insertions): `markup = { verse_numbers = {
show = true, display = "[{verse}]" }, ... }`
- Labels' `display` (verse and chapter numbers) can use `{verse}`, `{chapter}`, `{book}` (the
  USX code, `JHN`), and `{name}` (`John`)
- Other elements' `display` is a template with `{text}` for their text (and `{name}`), like
  `## {text}` for headings or `“{text}”` for words of Jesus
- Headings and notes that aren't shown are left out; the others are plain text
- Other keys, like the editor's `color`, are for the editor
*/
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct MarkupSettings {
    /// Shown by default, as `[{verse}]`
    #[serde(alias = "verse-numbers")]
    pub verse_numbers: MarkupElement,
    /// Shown by default, as `{name} {chapter}` on its own line (in whole books)
    #[serde(alias = "chapter-numbers")]
    pub chapter_numbers: MarkupElement,
    /// Section headings, shown by default
    pub headings: MarkupElement,
    /// Footnotes; hidden by default, and `[{text}]` when shown
    pub notes: MarkupElement,
    /// Words of Christ (red letters)
    #[serde(alias = "words-of-jesus")]
    pub words_of_jesus: MarkupElement,
    /// Translators' additions (KJV italics)
    pub added: MarkupElement,
    /// LORD in small capitals
    #[serde(alias = "divine-name")]
    pub divine_name: MarkupElement,
    /// Quotations other than words of Jesus
    pub quotes: MarkupElement,
    /// Lines longer than this many characters break at a space (none by default)
    pub width: Option<usize>,
}

impl MarkupSettings {
    /// The elements with a `{text}` template: their kind, settings, and whether they're shown by
    /// default
    pub fn elements(&self) -> [(Kind, &MarkupElement, bool); 6] {
        [
            (Kind::Title, &self.headings, true),
            (Kind::Note, &self.notes, false),
            (Kind::WordsOfJesus, &self.words_of_jesus, true),
            (Kind::Added, &self.added, true),
            (Kind::DivineName, &self.divine_name, true),
            (Kind::Quote, &self.quotes, true),
        ]
    }

    /// These settings with `over`'s on top (each `show` and `display` it sets)
    pub fn merged(&self, over: &MarkupSettings) -> MarkupSettings {
        let merge = |base: &MarkupElement, over: &MarkupElement| MarkupElement {
            show: over.show.or(base.show),
            display: over.display.clone().or_else(|| base.display.clone()),
        };
        MarkupSettings {
            verse_numbers: merge(&self.verse_numbers, &over.verse_numbers),
            chapter_numbers: merge(&self.chapter_numbers, &over.chapter_numbers),
            headings: merge(&self.headings, &over.headings),
            notes: merge(&self.notes, &over.notes),
            words_of_jesus: merge(&self.words_of_jesus, &over.words_of_jesus),
            added: merge(&self.added, &over.added),
            divine_name: merge(&self.divine_name, &over.divine_name),
            quotes: merge(&self.quotes, &over.quotes),
            width: over.width.or(self.width),
        }
    }
}

/// One kind of element in [`MarkupSettings`]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct MarkupElement {
    pub show: Option<bool>,
    pub display: Option<String>,
}

impl MarkupElement {
    pub fn shown(&self, default: bool) -> bool {
        self.show.unwrap_or(default)
    }
}

/**
A way to insert a passage's text, offered as a code action on references (`Insert Callout`):
`{ name = "Callout", display = "> [!bible] {reference}\n{text}", line_prefix = "> " }`
*/
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Insertion {
    /// The action's name: `Insert {name}`
    pub name: String,
    /// What is inserted on the lines after the reference: `{text}` (the passage), `{reference}`
    /// (as formatted, like `John 3:16`), and `{osis}` (`John.3.16`); default `{text}`
    pub display: String,
    /// Written before each line of `{text}`, like `> ` for a quote
    #[serde(alias = "line-prefix")]
    pub line_prefix: String,
    /// The `markup` settings for this insertion, over the others
    pub markup: MarkupSettings,
}

impl Default for Insertion {
    fn default() -> Self {
        Insertion {
            name: String::from("Passage"),
            display: String::from("{text}"),
            line_prefix: String::new(),
            markup: MarkupSettings::default(),
        }
    }
}

/// One value or a list (`merge-data = "a.json"` or `["a.json", "b.json"]`)
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany {
    #[default]
    None,
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn values(&self) -> Vec<String> {
        match self {
            OneOrMany::None => vec![],
            OneOrMany::One(value) => vec![value.clone()],
            OneOrMany::Many(values) => values.clone(),
        }
    }
}

/// `format = "abbreviation"` (the book style, like `-f`), or a whole format like `psg-fmt`
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum FormatSetting {
    Style(BookStyle),
    Options(FormatOptions),
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Settings {
    pub format: Option<FormatSetting>,
    pub psg_fmt: Option<FormatOptions>,
    pub fmt_book_separator: Option<String>,
    pub fmt_chapter_verse: Option<String>,
    pub fmt_range: Option<String>,
    pub fmt_verse_separator: Option<String>,
    pub fmt_chapter_separator: Option<String>,
    pub fmt_join_adjacent: Option<bool>,
    pub fmt_omit_first_verse_of_chapter_range: Option<bool>,
    pub fmt_chapter_in_single_chapter_books: Option<bool>,
    pub data: Option<String>,
    pub merge_data: OneOrMany,
    pub remove_data: OneOrMany,
    pub context_book: Option<String>,
    pub context_heading: Option<String>,
    /// Extensions searched for workspace references (`"md,txt"` or a list; empty means every
    /// text file)
    pub ext: OneOrMany,
    pub inlay_hints: InlayHints,
    pub reference_diagnostics: ReferenceDiagnostics,
    /// What the hover shows below the reference (`"osis,book"` or a list; `[]` for none)
    pub hover: Option<OneOrMany>,
    /// A folder of USX files (one book each), for passage text in hover and go-to-definition
    pub bible: Option<String>,
    pub markup: MarkupSettings,
    /// Ways to insert a passage's text, offered as code actions (with a `bible`)
    pub insertions: Vec<Insertion>,
    /// Read this config file instead of `~/.config/topos/config.toml`
    pub config: Option<String>,
    /// Don't read a config file
    pub no_config: bool,
}

/// What the server works with, built from [`Settings`]
pub struct Configured {
    pub matcher: BibleMatcher,
    pub format: FormatOptions,
    pub inlay_hints: InlayHints,
    pub reference_diagnostics: ReferenceDiagnostics,
    pub hover: Vec<HoverField>,
    pub extensions: Vec<String>,
    pub bible: Option<Bible>,
    pub insertions: Vec<Insertion>,
}

/// `psg_fmt` and `psg-fmt` are the same key (only at the top level: the format's own fields are
/// snake_case, like `join_adjacent`)
fn normalize(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map
            .into_iter()
            .map(|(key, value)| (key.replace('_', "-"), value))
            .collect(),
        _ => Map::new(),
    }
}

/// Editor settings may be the settings themselves or nested under `topos`
/// (`settings = { topos = { ... } }` in Neovim)
pub fn editor_layer(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(mut map) if map.contains_key("topos") => {
            normalize(map.remove("topos").unwrap_or_default())
        }
        value => normalize(value),
    }
}

/// `$XDG_CONFIG_HOME/topos/config.toml`, else `~/.config/topos/config.toml`
fn default_config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("topos").join("config.toml"))
}

fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(path),
    }
}

/// The config file's settings (none if there is no file, or the editor turns it off)
pub fn config_layer(editor: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    if editor.get("no-config").and_then(Value::as_bool) == Some(true) {
        return Ok(Map::new());
    }
    let path = match editor.get("config").and_then(Value::as_str) {
        Some(path) => expand_home(path),
        None => match default_config_path().filter(|path| path.exists()) {
            Some(path) => path,
            None => return Ok(Map::new()),
        },
    };
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let table: toml::Table = text
        .parse()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let value = serde_json::to_value(table).map_err(|e| e.to_string())?;
    Ok(normalize(value))
}

/// The config file, then the editor's settings on top
pub fn settings(
    config: Map<String, Value>,
    editor: &Map<String, Value>,
) -> Result<Settings, String> {
    let mut merged = config;
    merged.extend(editor.clone());
    serde_json::from_value(Value::Object(merged)).map_err(|e| format!("topos settings: {e}"))
}

impl Settings {
    /// The defaults, then `psg-fmt`, then each `fmt-*`, then `format` (like the CLI)
    pub fn format_options(&self) -> FormatOptions {
        let mut options = self.psg_fmt.clone().unwrap_or_default();
        if let Some(FormatSetting::Options(whole)) = &self.format {
            options = whole.clone();
        }
        let text = |value: &Option<String>, field: &mut String| {
            if let Some(value) = value {
                field.clone_from(value);
            }
        };
        text(&self.fmt_book_separator, &mut options.book_separator);
        text(&self.fmt_chapter_verse, &mut options.chapter_verse);
        text(&self.fmt_range, &mut options.range);
        text(&self.fmt_verse_separator, &mut options.verse_separator);
        text(&self.fmt_chapter_separator, &mut options.chapter_separator);
        if let Some(join) = self.fmt_join_adjacent {
            options.join_adjacent = join;
        }
        if let Some(omit) = self.fmt_omit_first_verse_of_chapter_range {
            options.omit_first_verse_of_chapter_range = omit;
        }
        if let Some(chapter) = self.fmt_chapter_in_single_chapter_books {
            options.chapter_in_single_chapter_books = chapter;
        }
        if let Some(FormatSetting::Style(style)) = self.format {
            options.book = style;
        }
        options
    }

    /// `data` (or the defaults), then each `merge-data`, then each `remove-data`, and the book
    /// context (like the CLI)
    pub fn configure(&self) -> Result<Configured, String> {
        fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
            let text =
                std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
        }
        let mut input = match &self.data {
            Some(path) => read::<BibleDataInput>(&expand_home(path))?,
            None => BibleDataInput::defaults(),
        };
        for path in self.merge_data.values() {
            let path = expand_home(&path);
            input
                .merge(read::<DataPatch>(&path)?)
                .map_err(|e| format!("{}: {e}", path.display()))?;
        }
        for path in self.remove_data.values() {
            let path = expand_home(&path);
            input
                .remove(read::<DataPatch>(&path)?)
                .map_err(|e| format!("{}: {e}", path.display()))?;
        }
        let data = BibleData::new(input).map_err(|e| e.to_string())?;
        let context = match (&self.context_book, &self.context_heading) {
            (Some(book), _) => {
                Some(BookContext::Book(data.books().search(book).ok_or_else(
                    || format!("context-book: unknown book `{book}`"),
                )?))
            }
            (None, Some(pattern)) => Some(
                BookContext::headings(data.books(), pattern)
                    .map_err(|e| format!("context-heading: {e}"))?,
            ),
            (None, None) => None,
        };
        let bible = match &self.bible {
            Some(folder) => Some(Bible::load(
                &expand_home(folder),
                data.clone(),
                self.markup.clone(),
            )?),
            None => None,
        };
        let matcher = BibleFilter::new(data).create_matcher();
        let extensions = self
            .ext
            .values()
            .iter()
            .flat_map(|ext| ext.split(','))
            .map(|ext| ext.trim().trim_start_matches('.').to_ascii_lowercase())
            .filter(|ext| !ext.is_empty())
            .collect();
        let hover = match &self.hover {
            None => HoverField::DEFAULT.to_vec(),
            Some(fields) => fields
                .values()
                .iter()
                .flat_map(|field| field.split(','))
                .map(str::trim)
                .filter(|field| !field.is_empty())
                .map(|field| {
                    serde_json::from_value(Value::String(field.to_ascii_lowercase()))
                        .map_err(|_| format!("hover: unknown field `{field}`"))
                })
                .collect::<Result<_, _>>()?,
        };
        Ok(Configured {
            matcher: match context {
                Some(context) => matcher.with_context(context),
                None => matcher,
            },
            format: self.format_options(),
            inlay_hints: self.inlay_hints,
            reference_diagnostics: self.reference_diagnostics,
            hover,
            extensions,
            bible,
            insertions: self.insertions.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn parse(config: &str, editor: Value) -> Settings {
        let table: toml::Table = config.parse().unwrap();
        let config = normalize(serde_json::to_value(table).unwrap());
        settings(config, &editor_layer(editor)).unwrap()
    }

    #[test]
    fn config_then_editor() {
        let settings = parse(
            "mode = \"grouped\"\nformat = \"abbreviation\"\npsg-fmt = { join_adjacent = true }\nfmt-range = \"–\"\n",
            json!({ "topos": { "fmt_range": "~", "inlay-hints": "osis" } }),
        );
        let options = settings.format_options();
        assert_eq!(options.book, BookStyle::Abbreviation);
        assert!(options.join_adjacent);
        // The editor's setting wins, and snake_case keys work
        assert_eq!(options.range, "~");
        assert_eq!(settings.inlay_hints, InlayHints::Osis);
    }

    #[test]
    fn format_as_a_whole_object() {
        let settings = parse(
            "",
            json!({ "format": { "book": "osis", "chapter_verse": "." } }),
        );
        let options = settings.format_options();
        assert_eq!(
            (options.book, options.chapter_verse.as_str()),
            (BookStyle::Osis, ".")
        );
    }

    #[test]
    fn extensions_and_bad_values() {
        let settings = parse("ext = \"md, .TXT\"", json!({}));
        assert_eq!(settings.configure().unwrap().extensions, ["md", "txt"]);
        let table: toml::Table = "inlay-hints = \"sometimes\"".parse().unwrap();
        let config = normalize(serde_json::to_value(table).unwrap());
        assert!(super::settings(config, &Map::new()).is_err());
    }

    #[test]
    fn hover_fields() {
        let default = parse("", json!({})).configure().unwrap().hover;
        assert_eq!(default, HoverField::DEFAULT);
        let listed = parse("hover = \"OSIS, bcv\"", json!({}))
            .configure()
            .unwrap();
        assert_eq!(listed.hover, [HoverField::Osis, HoverField::Bcv]);
        let none = parse("", json!({ "hover": [] })).configure().unwrap();
        assert!(none.hover.is_empty());
        let unknown = parse("hover = [\"chapter\"]", json!({})).configure();
        assert_eq!(unknown.err().unwrap(), "hover: unknown field `chapter`");
    }
}
