use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    str::FromStr,
    time::{Duration, Instant},
};

use lsp_types::{
    CodeAction, CodeActionOrCommand, CodeActionParams, CodeActionProviderCapability, Command,
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse,
    ExecuteCommandOptions, ExecuteCommandParams, GotoDefinitionParams, GotoDefinitionResponse,
    Hover, HoverContents, HoverParams, HoverProviderCapability, InlayHint, InlayHintLabel,
    InlayHintParams, Location, MarkupContent, MarkupKind, OneOf, Position,
    PublishDiagnosticsParams, Range, ReferenceParams, SemanticToken, SemanticTokenType,
    SemanticTokens, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensParams, SemanticTokensServerCapabilities, ServerCapabilities, SymbolKind,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Uri,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use topos_bible::{
    data::{books::BookId, translations::Translation},
    filter::{bible_filter::BibleFilter, filters::testament::TestamentFilter},
    matcher::{BibleMatch, BibleMatcher, LineIndex, TranslatedMatch},
    segments::{
        Passage,
        autocomplete::{CompleteOptions, CompletionKind},
        coverage::VerseRange,
        formatter::{BookStyle, FormatOptions},
    },
};

use crate::{
    bible::{self, Bible, Bibles, MAX_HOVER_TEXT, Part, Token},
    settings::{
        self, Configured, HoverField, InlayHints, Insertion, ReferenceDiagnostics, Replace,
    },
    workspace,
};

/// How long hovering a verse in a whole book searches for its references before giving up
const HOVER_SEARCH_TIME: Duration = Duration::from_millis(500);

/// The request for a whole book's text: `{ "uri": "topos-bible://BSB/JHN" }` gives
/// [`BookText`]
pub const BOOK_REQUEST: &str = "topos/book";

#[derive(Debug, Deserialize)]
pub struct BookParams {
    pub uri: String,
}

/// The request for the Bibles: no parameters, gives [`BiblesList`]
pub const BIBLES_REQUEST: &str = "topos/bibles";

/// The Bibles from the `bible` and `bibles` settings, the default first
#[derive(Debug, Serialize)]
pub struct BiblesList {
    pub bibles: Vec<BibleInfo>,
}

#[derive(Debug, Serialize)]
pub struct BibleInfo {
    /// Its name in `topos-bible://` URIs and after references: `BSB`
    pub name: String,
    /// Other ways to write it after a reference
    pub aliases: Vec<String>,
    /// The books it has, as USFM codes (`JHN`), in order
    pub books: Vec<String>,
}

/**
The request for the same verse in another Bible: `{ "uri": "topos-bible://BSB/JHN", "position":
{ "line": 3, "character": 0 }, "bible": "KJV" }` gives the [`Location`] of the verse at that
position (or the chapter, for a chapter's label) in the KJV's John, or `null` if it lacks the book
*/
pub const SAME_VERSE_REQUEST: &str = "topos/sameVerse";

#[derive(Debug, Deserialize)]
pub struct SameVerseParams {
    pub uri: String,
    pub position: Position,
    /// The other Bible's name or alias
    pub bible: String,
}

/// A whole book, for the editor to show as a read-only document
#[derive(Debug, Serialize)]
pub struct BookText {
    /// The book's name: `John`
    pub title: String,
    pub text: String,
}

/// The command code actions run; its result is the matching locations
pub const SEARCH_COMMAND: &str = "topos.search";

/**
The command reformat actions use in unnamed documents: `{ "edits": [TextEdit] }` for the
editor to apply to the document the action came from
- A workspace edit names its document by URI, and an unnamed document's (`file://`) names no
  buffer an editor can find (Neovim makes a new one), so the edits go with the action instead
*/
pub const APPLY_EDITS_COMMAND: &str = "topos.applyEdits";

/// Whether a URI names no file (an unnamed buffer's `file://`)
pub fn is_unnamed(uri: &Uri) -> bool {
    workspace::uri_to_path(uri).is_none_or(|path| path.as_os_str().is_empty())
}

/// The kinds of workspace search, like the CLI's passage filters
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SearchMode {
    /// `-o`: references that name a verse of the passage (whole chapters don't count)
    ExplicitOverlap,
    /// `--any-overlap`: references that share any verse with it
    AnyOverlap,
    /// `--exact-overlap`: references that are exactly it (go-to-references)
    ExactOverlap,
    /// `-i`: references entirely inside it
    Inside,
}

impl SearchMode {
    const ALL: [SearchMode; 4] = [
        SearchMode::ExplicitOverlap,
        SearchMode::AnyOverlap,
        SearchMode::ExactOverlap,
        SearchMode::Inside,
    ];

    fn title(self, reference: &str) -> String {
        match self {
            SearchMode::ExplicitOverlap => format!("Search \"{reference}\" for explicit overlap"),
            SearchMode::AnyOverlap => format!("Search \"{reference}\" for any overlap"),
            SearchMode::ExactOverlap => format!("Search \"{reference}\" for exact overlap"),
            SearchMode::Inside => format!("Search inside \"{reference}\""),
        }
    }
}

/// The arguments of [`SEARCH_COMMAND`]
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchArguments {
    pub mode: SearchMode,
    /// The passage, written so it parses back (`John 3:16`)
    pub passage: String,
}

/// Open documents and the matcher, with one method per LSP request or notification
pub struct Server {
    matcher: BibleMatcher,
    /// How references are written: completions, hover, symbols, hints, and action titles
    format: FormatOptions,
    inlay_hints: InlayHints,
    reference_diagnostics: ReferenceDiagnostics,
    /// What the hover shows below the reference
    hover: Vec<HoverField>,
    /// Extensions searched in the workspace (empty for every text file)
    extensions: Vec<String>,
    documents: HashMap<Uri, String>,
    /// The workspace folders, for workspace-wide searches
    roots: Vec<PathBuf>,
    /// The editor's settings (initialization options, then the latest configuration)
    editor: Map<String, Value>,
    /// Each workspace file's references, reused while it is unchanged
    cache: workspace::Cache,
    /// The `bible` and `bibles` settings' text, the default first
    bibles: Bibles,
    /// The Bibles whose text hover shows, after the one a reference names
    hover_bibles: Vec<String>,
    /// Ways to insert a passage's text (code actions)
    insertions: Vec<Insertion>,
}

impl Server {
    pub fn new(matcher: BibleMatcher) -> Self {
        Self {
            matcher,
            format: FormatOptions::default(),
            inlay_hints: InlayHints::default(),
            reference_diagnostics: ReferenceDiagnostics::default(),
            hover: HoverField::DEFAULT.to_vec(),
            extensions: vec![],
            documents: HashMap::new(),
            roots: vec![],
            editor: Map::new(),
            cache: workspace::Cache::default(),
            bibles: Bibles::default(),
            hover_bibles: vec![],
            insertions: vec![],
        }
    }

    pub fn set_roots(&mut self, roots: Vec<PathBuf>) {
        self.roots = roots;
    }

    /**
    Applies the config file and the editor's settings (`initializationOptions` or
    `workspace/didChangeConfiguration`'s `settings`, either may nest them under `topos`)
    - On an error the previous settings stay, and the error says why
    */
    pub fn configure(&mut self, editor: Value) -> Result<(), String> {
        let editor = settings::editor_layer(editor);
        let config = settings::config_layer(&editor)?;
        let Configured {
            matcher,
            format,
            inlay_hints,
            reference_diagnostics,
            hover,
            extensions,
            bibles,
            hover_bibles,
            insertions,
        } = settings::settings(config, &editor)?.configure()?;
        self.bibles = bibles;
        self.hover_bibles = hover_bibles;
        self.insertions = insertions;
        self.matcher = matcher;
        self.format = format;
        self.inlay_hints = inlay_hints;
        self.reference_diagnostics = reference_diagnostics;
        self.hover = hover;
        self.extensions = extensions;
        self.editor = editor;
        // New data can change what every file contains
        self.cache.clear();
        Ok(())
    }

    /// The open documents (whose diagnostics change with the settings)
    pub fn documents(&self) -> Vec<Uri> {
        self.documents.keys().cloned().collect()
    }

    pub fn capabilities() -> ServerCapabilities {
        ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some([" ", ":", "-", ",", ";", "."].map(String::from).to_vec()),
                // The passage's text, for chapters and verses
                resolve_provider: Some(true),
                ..CompletionOptions::default()
            }),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            inlay_hint_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Left(true)),
            definition_provider: Some(OneOf::Left(true)),
            declaration_provider: Some(lsp_types::DeclarationCapability::Simple(true)),
            // Only whole books have tokens: their verse numbers, words of Jesus, ...
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    legend: SemanticTokensLegend {
                        token_types: Token::LEGEND.map(SemanticTokenType::new).to_vec(),
                        token_modifiers: vec![],
                    },
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                    ..SemanticTokensOptions::default()
                }),
            ),
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            execute_command_provider: Some(ExecuteCommandOptions {
                commands: vec![SEARCH_COMMAND.into()],
                ..ExecuteCommandOptions::default()
            }),
            ..ServerCapabilities::default()
        }
    }

    pub fn did_open(&mut self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        self.documents.insert(doc.uri, doc.text);
    }

    /// Only full syncs are advertised, so the last change is the whole document
    pub fn did_change(&mut self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents.insert(params.text_document.uri, change.text);
        }
    }

    pub fn did_close(&mut self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }

    pub fn completion(&self, params: CompletionParams) -> Option<CompletionResponse> {
        let position = params.text_document_position;
        let text = self.documents.get(&position.text_document.uri)?;
        let index = LineIndex::new(text);
        let cursor = index.offset_of_utf16(
            position.position.line as usize,
            position.position.character as usize,
        )?;
        let options = CompleteOptions {
            format: self.format.clone(),
            limit: Some(100),
        };
        let translations = &self.bibles.translations;
        let items = self
            .matcher
            .complete_with_translations(text, cursor, &options, translations)
            .into_iter()
            .enumerate()
            .map(|(idx, completion)| {
                let range = completion.edit.range;
                CompletionItem {
                    label: completion.label.clone(),
                    kind: Some(match completion.kind {
                        CompletionKind::Book => CompletionItemKind::FOLDER,
                        CompletionKind::Chapter => CompletionItemKind::MODULE,
                        CompletionKind::Verse => CompletionItemKind::REFERENCE,
                        CompletionKind::Translation => CompletionItemKind::ENUM_MEMBER,
                    }),
                    /*
                    What was typed, then the label (`jn 3:1 John 3:16`):
                    - Editors that match from the start (Neovim's own completion) see what was
                      typed, so `jn 3:1` still shows `John 3:16`
                    - Editors that match the word before the cursor (blink.cmp) find it in the
                      label, even against a list made a keystroke ago: with only what was
                      typed, `1 C`'s list filtered `Co` to nothing, so `1 Co` showed none
                    */
                    filter_text: Some(format!("{} {}", &text[range.clone()], completion.label)),
                    // Keep the order: chapters and verses are in reading order
                    sort_text: Some(format!("{idx:05}")),
                    // For `completionItem/resolve` to add the passage's text (a translation's
                    // in that Bible)
                    data: completion
                        .passage
                        .as_ref()
                        .filter(|_| !self.bibles.is_empty())
                        .map(|p| match completion.kind {
                            CompletionKind::Translation => serde_json::json!({
                                "passage": self.parseable(p),
                                "bible": completion.label,
                            }),
                            _ => serde_json::json!({ "passage": self.parseable(p) }),
                        }),
                    text_edit: Some(lsp_types::CompletionTextEdit::Edit(TextEdit {
                        range: lsp_range(&index, range.start, range.end),
                        new_text: completion.edit.text,
                    })),
                    ..CompletionItem::default()
                }
            })
            .collect();
        // Incomplete: the editor asks again as you type, instead of filtering this list itself
        // (which filtered `1 Ti` against what was typed when it was made, `1 T`, and emptied it)
        Some(CompletionResponse::List(lsp_types::CompletionList {
            is_incomplete: true,
            items,
        }))
    }

    /// A completion with its passage's text as documentation (with a `bible`), and what to color
    /// in it as `data.tokens` (see [`bible::Text::fence`])
    pub fn resolve_completion(&self, mut item: CompletionItem) -> CompletionItem {
        let data = item.data.as_ref();
        let bible = data
            .and_then(|data| data.get("bible")?.as_str())
            .and_then(|name| self.bibles.get(name))
            .or_else(|| self.bibles.default_bible());
        let text = data
            .and_then(|data| data.get("passage")?.as_str())
            .and_then(|passage| self.matcher.data().parse(passage))
            .and_then(|passage| bible?.passage(&passage, None, Some(MAX_HOVER_TEXT)));
        if let Some(text) = text {
            // What to color, for editors that draw the documentation themselves
            if let Some(Value::Object(data)) = &mut item.data {
                data.insert("tokens".into(), Value::String(text.token_info()));
            }
            item.documentation = Some(lsp_types::Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: text.fence(false),
            }));
        }
        item
    }

    /**
    The document's diagnostics:
    - Warnings for references that do not exist, like `John 3:99` (code `missing`)
    - An information diagnostic on each reference: how it is written in the configured format,
      like `John 3:16` (code `reference`; a hint, or none, with `reference-diagnostics`)
    */
    pub fn diagnostics(&self, uri: &Uri) -> PublishDiagnosticsParams {
        // Whole books are Bible text, not notes
        let text = (!bible::is_book_uri(uri.as_str()))
            .then(|| self.documents.get(uri))
            .flatten();
        let diagnostics = text.map_or_else(Vec::new, |text| {
            let index = LineIndex::new(text);
            let diagnostic = |range, severity, code: &str, message| Diagnostic {
                range,
                severity: Some(severity),
                code: Some(lsp_types::NumberOrString::String(code.into())),
                source: Some(String::from("topos")),
                message,
                ..Diagnostic::default()
            };
            let mut diagnostics: Vec<Diagnostic> = self
                .matcher
                .problems_with_translations(text, &self.bibles.translations)
                .into_iter()
                .map(|problem| {
                    let range = lsp_range(&index, problem.bytes.start, problem.bytes.end);
                    diagnostic(
                        range,
                        DiagnosticSeverity::WARNING,
                        "missing",
                        problem.message,
                    )
                })
                .collect();
            let severity = match self.reference_diagnostics {
                ReferenceDiagnostics::Info => Some(DiagnosticSeverity::INFORMATION),
                ReferenceDiagnostics::Hint => Some(DiagnosticSeverity::HINT),
                ReferenceDiagnostics::Never => None,
            };
            if let Some(severity) = severity {
                for m in self.matcher.search(text) {
                    let bytes = m.location.bytes;
                    let range = lsp_range(&index, bytes.start, bytes.end);
                    diagnostics.push(diagnostic(range, severity, "reference", self.reference(&m)));
                }
                // In document order, so they read top to bottom
                diagnostics.sort_by_key(|d| (d.range.start.line, d.range.start.character));
            }
            diagnostics
        });
        PublishDiagnosticsParams::new(uri.clone(), diagnostics, None)
    }

    /// The document's diagnostics when the editor asks for them (pull diagnostics)
    pub fn pull_diagnostics(
        &self,
        params: lsp_types::DocumentDiagnosticParams,
    ) -> lsp_types::DocumentDiagnosticReportResult {
        let items = self.diagnostics(&params.text_document.uri).diagnostics;
        lsp_types::DocumentDiagnosticReportResult::Report(
            lsp_types::DocumentDiagnosticReport::Full(
                lsp_types::RelatedFullDocumentDiagnosticReport {
                    related_documents: None,
                    full_document_diagnostic_report: lsp_types::FullDocumentDiagnosticReport {
                        result_id: None,
                        items,
                    },
                },
            ),
        )
    }

    /// The reference at a position (or at the translation named after it), with the document's
    /// line index
    fn reference_at(
        &self,
        uri: &Uri,
        position: Position,
    ) -> Option<(TranslatedMatch, LineIndex<'_>)> {
        let text = self.documents.get(uri)?;
        let index = LineIndex::new(text);
        let cursor = index.offset_of_utf16(position.line as usize, position.character as usize)?;
        let found = self
            .matcher
            .search_with_translations(text, &self.bibles.translations)
            .into_iter()
            .find(|m| {
                let bytes = m.reference.location.bytes;
                let end = m.translation.as_ref().map_or(bytes.end, |t| t.span.end);
                bytes.start <= cursor && cursor <= end
            })?;
        Some((found, index))
    }

    /// The Bibles whose text hover shows for a reference: the one it names (or the default), then
    /// the `hover-bibles` setting's
    fn hover_texts(&self, translation: Option<&Translation>) -> Vec<&Bible> {
        let mut shown: Vec<&Bible> = self.bibles.named(translation).into_iter().collect();
        for bible in self
            .hover_bibles
            .iter()
            .filter_map(|name| self.bibles.get(name))
        {
            if !shown.iter().any(|b| b.name == bible.name) {
                shown.push(bible);
            }
        }
        shown
    }

    /// The reference under the cursor, normalized, with what the `hover` setting lists about it
    /// (its OSIS id, book, genres, ...); or, on a reference that doesn't exist, what does
    /// (`John 3 has 36 verses`)
    pub fn hover(&self, params: HoverParams) -> Option<Hover> {
        let position = params.text_document_position_params;
        let uri = &position.text_document.uri;
        if let Some(hover) = self.problem_hover(uri, position.position) {
            return Some(hover);
        }
        let Some((
            TranslatedMatch {
                reference: found,
                translation,
            },
            index,
        )) = self.reference_at(uri, position.position)
        else {
            return self.verse_hover(uri, position.position);
        };
        let bytes = found.location.bytes;
        let written = self
            .documents
            .get(uri)
            .and_then(|text| text.get(bytes.start..bytes.end))
            .unwrap_or_default();
        let title = self.reference(&found);
        let mut value = match &translation {
            Some(translation) => format!("**{title}** ({})", translation.name),
            None => format!("**{title}**"),
        };
        // Lists of lines, and the passage's text between them
        let mut sections: Vec<String> = vec![];
        let mut lines: Vec<String> = vec![];
        for &field in &self.hover {
            if field == HoverField::Text {
                let bibles = self.hover_texts(translation.as_ref());
                // Each Bible's name labels its text when there are several
                let labeled = self.bibles.len() > 1;
                for bible in bibles {
                    let Some(text) = bible.passage(&found.psg, None, Some(MAX_HOVER_TEXT)) else {
                        continue;
                    };
                    if !lines.is_empty() {
                        sections.push(lines.join("\n"));
                        lines.clear();
                    }
                    sections.push(match labeled {
                        true => format!("*{}*\n{}", bible.name, text.fence(true)),
                        false => text.fence(true),
                    });
                }
                continue;
            }
            let Some((label, detail)) = self.hover_line(field, &found, written) else {
                continue;
            };
            // The full name or abbreviation is left out when it is the title already
            let repeats =
                matches!(field, HoverField::Name | HoverField::Abbreviation) && detail == title;
            if !repeats {
                lines.push(format!("- **{label}**: {detail}"));
            }
        }
        if !lines.is_empty() {
            sections.push(lines.join("\n"));
        }
        for section in sections {
            value.push_str("\n\n");
            value.push_str(&section);
        }
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(lsp_range(&index, bytes.start, bytes.end)),
        })
    }

    /// One hover line's label and text, or [`None`] when there is nothing to say
    fn hover_line(
        &self,
        field: HoverField,
        found: &BibleMatch,
        written: &str,
    ) -> Option<(&'static str, String)> {
        let data = self.matcher.data();
        let psg = &found.psg;
        let styled = |book| {
            FormatOptions {
                book,
                ..self.format.clone()
            }
            .passage(psg, data)
        };
        let versification = data.chapter_verses().get_chapter_verses(&psg.book);
        Some(match field {
            HoverField::Text => return None,
            HoverField::Written => ("Written", format!("`{written}`")),
            HoverField::Name => ("Name", styled(BookStyle::Name)?),
            HoverField::Abbreviation => ("Abbreviation", styled(BookStyle::Abbreviation)?),
            HoverField::Osis => ("OSIS", format!("`{}`", psg.to_osis(data.books())?)),
            HoverField::Usfm => ("USFM", format!("`{}`", psg.to_usfm(data.books())?)),
            HoverField::Book => {
                let name = data.books().get_name(psg.book)?;
                let books = data.books().ids().count();
                let mut detail = format!("{name}, book {} of {books}", psg.book.0);
                if let Some(chapters) = versification.map(|v| v.get_chapter_count()) {
                    let plural = if chapters == 1 { "" } else { "s" };
                    detail.push_str(&format!(", {chapters} chapter{plural}"));
                }
                ("Book", detail)
            }
            HoverField::Testament => {
                let testament = if TestamentFilter::Old.contains(psg.book) {
                    "Old Testament"
                } else if TestamentFilter::New.contains(psg.book) {
                    "New Testament"
                } else {
                    return None;
                };
                ("Testament", testament.to_string())
            }
            HoverField::Genres => {
                let genres: Vec<&str> = data
                    .genres()
                    .iter()
                    .filter(|genre| genre.books().contains(&psg.book))
                    .map(|genre| genre.name())
                    .collect();
                let label = if genres.len() == 1 { "Genre" } else { "Genres" };
                (label, (!genres.is_empty()).then(|| genres.join(", "))?)
            }
            HoverField::Verses => {
                let mut verses = psg.verses(versification);
                verses.sort();
                verses.dedup();
                let ranges: Vec<String> = psg
                    .ranges(versification)
                    .iter()
                    .map(|range| {
                        let (start, end) = (range.start, range.end);
                        let cv = &self.format.chapter_verse;
                        let to = &self.format.range;
                        if end.verse == VerseRange::END_OF_CHAPTER {
                            start.chapter.to_string()
                        } else if start == end {
                            format!("{}{cv}{}", start.chapter, start.verse)
                        } else if start.chapter == end.chapter {
                            format!("{}{cv}{}{to}{}", start.chapter, start.verse, end.verse)
                        } else {
                            format!(
                                "{}{cv}{}{to}{}{cv}{}",
                                start.chapter, start.verse, end.chapter, end.verse
                            )
                        }
                    })
                    .collect();
                let count = if verses.is_empty() {
                    String::from("?")
                } else {
                    verses.len().to_string()
                };
                (
                    "Verses",
                    format!("{count} ({})", ranges.join(&self.format.chapter_separator)),
                )
            }
            HoverField::Bcv => {
                let keys: Vec<String> = psg
                    .bcv_ranges()
                    .into_iter()
                    .map(|range| match range.start() == range.end() {
                        true => format!("`{}`", range.start()),
                        false => format!("`{}-{}`", range.start(), range.end()),
                    })
                    .collect();
                ("BCV", keys.join(", "))
            }
            HoverField::Location => {
                let (start, end) = (found.location.start, found.location.end);
                // Columns are 1-based characters; the end is the last character's
                let last = end.char_column.saturating_sub(1).max(1);
                let detail = if start.line == end.line {
                    format!("line {}, columns {}-{last}", start.line, start.char_column)
                } else {
                    format!(
                        "line {}, column {} to line {}, column {last}",
                        start.line, start.char_column, end.line
                    )
                };
                ("Location", detail)
            }
        })
    }

    fn problem_hover(&self, uri: &Uri, position: Position) -> Option<Hover> {
        let text = self.documents.get(uri)?;
        let index = LineIndex::new(text);
        let cursor = index.offset_of_utf16(position.line as usize, position.character as usize)?;
        let problem = self
            .matcher
            .problems_with_translations(text, &self.bibles.translations)
            .into_iter()
            .find(|p| p.bytes.start <= cursor && cursor <= p.bytes.end)?;
        let value = match &problem.detail {
            Some(detail) => format!("**{}**\n\n{detail}", problem.message),
            None => format!("**{}**", problem.message),
        };
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(lsp_range(&index, problem.bytes.start, problem.bytes.end)),
        })
    }

    /// Every reference in the document, for outlines and pickers
    pub fn document_symbols(&self, params: DocumentSymbolParams) -> Option<DocumentSymbolResponse> {
        let text = self.documents.get(&params.text_document.uri)?;
        let index = LineIndex::new(text);
        let symbols = self
            .matcher
            .search(text)
            .into_iter()
            .map(|m| {
                let bytes = m.location.bytes;
                let range = lsp_range(&index, bytes.start, bytes.end);
                #[allow(deprecated)]
                DocumentSymbol {
                    name: self.reference(&m),
                    detail: m.psg.to_osis(self.matcher.data().books()),
                    kind: SymbolKind::KEY,
                    tags: None,
                    deprecated: None,
                    range,
                    selection_range: range,
                    children: None,
                }
            })
            .collect();
        Some(DocumentSymbolResponse::Nested(symbols))
    }

    /// A hint after each reference in the range: how it is written in the configured format
    /// (where that differs, by default), or its OSIS id
    pub fn inlay_hints(&self, params: InlayHintParams) -> Option<Vec<InlayHint>> {
        if self.inlay_hints == InlayHints::Never
            || bible::is_book_uri(params.text_document.uri.as_str())
        {
            return Some(vec![]);
        }
        let text = self.documents.get(&params.text_document.uri)?;
        let index = LineIndex::new(text);
        let hints = self
            .matcher
            .search(text)
            .into_iter()
            .filter_map(|m| {
                let bytes = m.location.bytes;
                let end = lsp_position(&index, bytes.end);
                let start = lsp_position(&index, bytes.start);
                if end < params.range.start || start > params.range.end {
                    return None;
                }
                let label = match self.inlay_hints {
                    InlayHints::Osis => m.psg.to_osis(self.matcher.data().books())?,
                    InlayHints::Usfm => m.psg.to_usfm(self.matcher.data().books())?,
                    InlayHints::Changed if self.reference(&m) == text[bytes.start..bytes.end] => {
                        return None;
                    }
                    _ => self.reference(&m),
                };
                Some(InlayHint {
                    position: end,
                    label: InlayHintLabel::String(label),
                    kind: None,
                    text_edits: None,
                    tooltip: None,
                    padding_left: Some(true),
                    padding_right: None,
                    data: None,
                })
            })
            .collect();
        Some(hints)
    }

    /**
    For the reference under the cursor:
    - Reformat it, if it isn't written the way completion would write it (and, when several
      references in the document are, reformat them all)
    - One search of the workspace per [`SearchMode`]
    */
    pub fn code_actions(&self, params: CodeActionParams) -> Option<Vec<CodeActionOrCommand>> {
        let uri = &params.text_document.uri;
        let (
            TranslatedMatch {
                reference: found,
                translation,
            },
            index,
        ) = self.reference_at(uri, params.range.start)?;
        let reference = self.reference(&found);
        let passage = self.parseable(&found.psg);
        let mut actions = self.reformat_actions(uri, &found, &index);
        actions.extend(self.insert_actions(uri, &found, translation.as_ref(), &index));
        let searches = SearchMode::ALL
            .into_iter()
            .map(|mode| {
                let title = mode.title(&reference);
                let arguments = SearchArguments {
                    mode,
                    passage: passage.clone(),
                };
                CodeActionOrCommand::CodeAction(CodeAction {
                    title: title.clone(),
                    command: Some(Command {
                        title,
                        command: SEARCH_COMMAND.into(),
                        arguments: Some(vec![serde_json::to_value(arguments).ok()?]),
                    }),
                    ..CodeAction::default()
                })
                .into()
            })
            .collect::<Option<Vec<CodeActionOrCommand>>>()?;
        actions.extend(searches);
        Some(actions)
    }

    /// `Reformat as "John 3:16"` for this reference, and for every reference in the document
    /// that is written differently, when there are several
    // `WorkspaceEdit::changes` is keyed by `Uri`, which clippy counts as a mutable key
    #[allow(clippy::mutable_key_type)]
    fn reformat_actions(
        &self,
        uri: &Uri,
        found: &BibleMatch,
        index: &LineIndex,
    ) -> Vec<CodeActionOrCommand> {
        let Some(text) = self.documents.get(uri) else {
            return vec![];
        };
        // Whole books are read-only
        if bible::is_book_uri(uri.as_str()) {
            return vec![];
        }
        // Each reference written differently from the configured format, with its rewrite
        let rewrite = |m: &BibleMatch| {
            let bytes = m.location.bytes;
            let formatted = self.reference(m);
            (text[bytes.start..bytes.end] != formatted).then(|| TextEdit {
                range: lsp_range(index, bytes.start, bytes.end),
                new_text: formatted,
            })
        };
        let action = |title: String, edits: Vec<TextEdit>| {
            edit_action(
                uri,
                title,
                edits,
                lsp_types::CodeActionKind::REFACTOR_REWRITE,
            )
        };
        let mut actions = vec![];
        if let Some(edit) = rewrite(found) {
            actions.push(action(
                format!("Reformat as \"{}\"", edit.new_text),
                vec![edit],
            ));
        }
        let all: Vec<TextEdit> = self
            .matcher
            .search(text)
            .iter()
            .filter_map(rewrite)
            .collect();
        if all.len() > 1 {
            let title = format!("Reformat all {} references in this file", all.len());
            actions.push(action(title, all));
        }
        actions
    }

    /**
    `Insert Callout` for each insertion: the passage's text on the lines after the reference, or
    in place of its line or of the reference itself (`replace`)
    - From the Bible the reference names; else one action per Bible, the default first
    - With several Bibles, titles name theirs: `Insert Callout (KJV)`
    */
    fn insert_actions(
        &self,
        uri: &Uri,
        found: &BibleMatch,
        translation: Option<&Translation>,
        index: &LineIndex,
    ) -> Vec<CodeActionOrCommand> {
        let Some(text) = self.documents.get(uri) else {
            return vec![];
        };
        if bible::is_book_uri(uri.as_str()) {
            return vec![];
        }
        let bibles: Vec<&Bible> = match translation {
            Some(_) => self.bibles.named(translation).into_iter().collect(),
            None => self.bibles.iter().collect(),
        };
        // The reference, its line, and where the line ends
        let bytes = found.location.bytes;
        let line_start = text[..bytes.start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[bytes.end..]
            .find('\n')
            .map_or(text.len(), |i| bytes.end + i);
        // Titles name the Bible when there are several
        let several = self.bibles.len() > 1;
        self.insertions
            .iter()
            .flat_map(|insertion| bibles.iter().map(move |bible| (insertion, *bible)))
            .filter_map(|(insertion, bible)| {
                let inserted = self.insertion_text(bible, insertion, found)?;
                let (range, new_text) = match insertion.replace {
                    None => (line_end..line_end, format!("\n{inserted}")),
                    Some(Replace::Line) => (line_start..line_end, inserted),
                    Some(Replace::Inline) => (bytes.start..bytes.end, inserted),
                };
                let at = lsp_range(index, range.start, range.end);
                let title = match several {
                    true => format!("Insert {} ({})", insertion.name, bible.name),
                    false => format!("Insert {}", insertion.name),
                };
                let edits = vec![TextEdit::new(at, new_text)];
                Some(edit_action(
                    uri,
                    title,
                    edits,
                    lsp_types::CodeActionKind::REFACTOR,
                ))
            })
            .collect()
    }

    /// What an insertion writes for a reference, or [`None`] if the Bible doesn't have it
    fn insertion_text(
        &self,
        bible: &Bible,
        insertion: &Insertion,
        found: &BibleMatch,
    ) -> Option<String> {
        let markup = bible.markup.merged(&insertion.markup);
        let text = bible.passage(&found.psg, Some(&markup), None)?.text;
        let text: Vec<String> = text
            .lines()
            .map(|line| {
                format!("{}{line}", insertion.line_prefix)
                    .trim_end()
                    .to_string()
            })
            .collect();
        let osis = found
            .psg
            .to_osis(self.matcher.data().books())
            .unwrap_or_default();
        Some(
            insertion
                .display
                .replace("{reference}", &self.reference(found))
                .replace("{osis}", &osis)
                .replace("{bible}", &bible.name)
                .replace("{text}", &text.join("\n")),
        )
    }

    /// Go to references: every reference in the workspace that is exactly the one under the
    /// cursor (`--exact-overlap`), however it is written
    pub fn references(&self, params: ReferenceParams) -> Option<Vec<Location>> {
        let position = params.text_document_position;
        let uri = &position.text_document.uri;
        let Some((found, _)) = self.reference_at(uri, position.position) else {
            // In a whole book, the verse under the cursor
            let (mode, passage) = self.verse_at(uri, position.position)?;
            return self.search(mode, &passage).ok();
        };
        self.search(
            SearchMode::ExactOverlap,
            &self.parseable(&found.reference.psg),
        )
        .ok()
    }

    /// The Bible and book of a whole-book document
    fn book_document(&self, uri: &Uri) -> Option<(&Bible, BookId)> {
        self.bibles.book_of(uri.as_str())
    }

    /**
    In a whole book, how to search for what is at a position:
    - In a verse, references that name it (explicit overlap: `John 3` doesn't count)
    - On a chapter's label, references that share any verse with the chapter
    */
    fn verse_at(&self, uri: &Uri, position: Position) -> Option<(SearchMode, String)> {
        let (bible, book) = self.book_document(uri)?;
        let rendered = bible.render(book)?;
        let index = LineIndex::new(&rendered.text);
        let cursor = index
            .offset_of_utf16(position.line as usize, position.character as usize)
            .unwrap_or(rendered.text.len());
        let name = bible.book_name(book)?;
        match rendered.part_at(cursor)? {
            Part::Verse {
                chapter,
                verse,
                last,
            } => {
                let passage = match last > verse {
                    true => format!("{name} {chapter}:{verse}-{last}"),
                    false => format!("{name} {chapter}:{verse}"),
                };
                Some((SearchMode::ExplicitOverlap, passage))
            }
            Part::Chapter(chapter) => Some((SearchMode::AnyOverlap, format!("{name} {chapter}"))),
        }
    }

    /**
    Hovering a verse in a whole book (not on a reference): how many references name it, and in
    how many files, or `???` if searching takes longer than [`HOVER_SEARCH_TIME`] (files read
    until then are cached, so the next hover gets further)
    */
    fn verse_hover(&self, uri: &Uri, position: Position) -> Option<Hover> {
        let (mode, passage) = self.verse_at(uri, position)?;
        let deadline = Instant::now() + HOVER_SEARCH_TIME;
        let count = match self.search_until(mode, &passage, Some(deadline)) {
            Ok((locations, true)) => {
                let files: HashSet<&str> = locations.iter().map(|l| l.uri.as_str()).collect();
                let plural = |n: usize| if n == 1 { "" } else { "s" };
                format!(
                    "{} reference{} in {} file{}",
                    locations.len(),
                    plural(locations.len()),
                    files.len(),
                    plural(files.len())
                )
            }
            _ => String::from("??? references (searching took too long)"),
        };
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("**{passage}**\n\n{count}"),
            }),
            range: None,
        })
    }

    /// Go to definition (and declaration): the reference's text in its whole book (one location
    /// per part of the reference), in the Bible it names or the default
    pub fn definition(&self, params: GotoDefinitionParams) -> Option<GotoDefinitionResponse> {
        let position = params.text_document_position_params;
        let (
            TranslatedMatch {
                reference: found,
                translation,
            },
            _,
        ) = self.reference_at(&position.text_document.uri, position.position)?;
        let bible = self.bibles.named(translation.as_ref())?;
        let rendered = bible.render(found.psg.book)?;
        let uri = Uri::from_str(&bible.uri(found.psg.book)?).ok()?;
        let index = LineIndex::new(&rendered.text);
        let mut locations: Vec<Location> = rendered
            .passage_ranges(&found.psg)
            .into_iter()
            .map(|r| Location::new(uri.clone(), lsp_range(&index, r.start, r.end)))
            .collect();
        match locations.len() {
            0 => None,
            1 => locations.pop().map(GotoDefinitionResponse::Scalar),
            _ => Some(GotoDefinitionResponse::Array(locations)),
        }
    }

    /// [`BOOK_REQUEST`]: a whole book's text
    pub fn book(&self, params: BookParams) -> Result<BookText, String> {
        if self.bibles.is_empty() {
            return Err("no Bible is set (the `bible` or `bibles` setting)".into());
        }
        let (bible, book) = self.bibles.book_of(&params.uri).ok_or_else(|| {
            let names: Vec<&str> = self.bibles.iter().map(|b| b.name.as_str()).collect();
            format!("{} is not a book of {}", params.uri, names.join(", "))
        })?;
        let rendered = bible
            .render(book)
            .ok_or_else(|| format!("{} can't be read", params.uri))?;
        Ok(BookText {
            title: bible.book_name(book).unwrap_or_default().to_string(),
            text: rendered.text.clone(),
        })
    }

    /// [`SAME_VERSE_REQUEST`]: where the verse at a position in a whole book is in another
    /// Bible's book (its verse with that number, else its chapter, else the next chapter it has,
    /// else the book's start)
    pub fn same_verse(&self, params: SameVerseParams) -> Result<Option<Location>, String> {
        let (from, book) = self
            .bibles
            .book_of(&params.uri)
            .ok_or_else(|| format!("{} is not a whole book", params.uri))?;
        let target = self
            .bibles
            .get(&params.bible)
            .ok_or_else(|| format!("no Bible is named `{}`", params.bible))?;
        let (Some(rendered), Some(other)) = (from.render(book), target.render(book)) else {
            return Ok(None);
        };
        let index = LineIndex::new(&rendered.text);
        let position = params.position;
        let cursor = index
            .offset_of_utf16(position.line as usize, position.character as usize)
            .unwrap_or(rendered.text.len());
        let (chapter, verse) = match rendered.part_at(cursor) {
            Some(Part::Verse { chapter, verse, .. }) => (chapter, Some(verse)),
            Some(Part::Chapter(chapter)) => (chapter, None),
            None => (0, None),
        };
        let chapter_of = |part: &Part| match *part {
            Part::Verse { chapter, .. } | Part::Chapter(chapter) => chapter,
        };
        let parts = &other.parts;
        let found = verse
            .and_then(|verse| {
                parts.iter().find(|(_, part)| {
                    matches!(*part, Part::Verse { chapter: c, verse: v, last }
                        if c == chapter && v <= verse && verse <= last)
                })
            })
            .or_else(|| (parts.iter()).find(|(_, part)| *part == Part::Chapter(chapter)))
            .or_else(|| (parts.iter()).find(|(_, part)| chapter_of(part) >= chapter))
            .map_or(0, |(range, _)| range.start);
        let uri = Uri::from_str(&target.uri(book).ok_or("the book has no USFM code")?)
            .map_err(|e| e.to_string())?;
        let index = LineIndex::new(&other.text);
        Ok(Some(Location::new(uri, lsp_range(&index, found, found))))
    }

    /// [`BIBLES_REQUEST`]: the Bibles, the default first
    pub fn bibles(&self) -> BiblesList {
        let bibles = self
            .bibles
            .iter()
            .map(|bible| BibleInfo {
                name: bible.name.clone(),
                aliases: bible.aliases.clone(),
                books: bible.book_codes(),
            })
            .collect();
        BiblesList { bibles }
    }

    /// Whole books' verse numbers, chapter labels, words of Jesus, headings, and notes (split at
    /// line breaks); other documents have none
    pub fn semantic_tokens(&self, params: SemanticTokensParams) -> SemanticTokens {
        let Some(rendered) = self
            .book_document(&params.text_document.uri)
            .and_then(|(bible, book)| bible.render(book))
        else {
            return SemanticTokens::default();
        };
        let text = &rendered.text;
        let index = LineIndex::new(text);
        let mut data = vec![];
        let (mut line, mut character) = (0, 0);
        for (range, token) in &rendered.tokens {
            let mut start = range.start;
            for piece in text[range.clone()].split('\n') {
                let end = start + piece.len();
                if !piece.trim().is_empty() {
                    let position = lsp_position(&index, start);
                    let delta_line = position.line - line;
                    let delta_start = match delta_line {
                        0 => position.character - character,
                        _ => position.character,
                    };
                    data.push(SemanticToken {
                        delta_line,
                        delta_start,
                        length: piece.encode_utf16().count() as u32,
                        token_type: *token as u32,
                        token_modifiers_bitset: 0,
                    });
                    (line, character) = (position.line, position.character);
                }
                start = end + 1;
            }
        }
        SemanticTokens {
            result_id: None,
            data,
        }
    }

    /// Runs [`SEARCH_COMMAND`], returning the locations it finds
    pub fn execute_command(&self, params: ExecuteCommandParams) -> Result<Vec<Location>, String> {
        if params.command != SEARCH_COMMAND {
            return Err(format!("unknown command {}", params.command));
        }
        let argument = params
            .arguments
            .into_iter()
            .next()
            .ok_or("topos.search needs { mode, passage }")?;
        let arguments: SearchArguments =
            serde_json::from_value(argument).map_err(|e| e.to_string())?;
        self.search(arguments.mode, &arguments.passage)
    }

    /// The references in the workspace that a search keeps, in file order
    pub fn search(&self, mode: SearchMode, passage: &str) -> Result<Vec<Location>, String> {
        self.search_until(mode, passage, None)
            .map(|(locations, _)| locations)
    }

    /// [`Server::search`], stopping at a deadline; also whether it searched every file
    fn search_until(
        &self,
        mode: SearchMode,
        passage: &str,
        deadline: Option<Instant>,
    ) -> Result<(Vec<Location>, bool), String> {
        let mut filter = BibleFilter::new(self.matcher.data().clone());
        match mode {
            SearchMode::ExplicitOverlap => filter.filter_explicit_overlap(passage),
            SearchMode::AnyOverlap => filter.filter_any_overlap(passage),
            SearchMode::ExactOverlap => filter.filter_exact_overlap(passage),
            SearchMode::Inside => filter.filter_inside(passage),
        }
        .map_err(|e| e.to_string())?;
        let query = filter.create_matcher();
        let (mut files, complete) = workspace::references(
            &self.roots,
            &self.documents,
            &self.extensions,
            &self.matcher,
            &self.cache,
            &self.bibles.folders(),
            deadline,
        );
        files.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        let locations = files
            .iter()
            .flat_map(|(uri, found)| {
                found
                    .iter()
                    .filter(|f| query.keeps(&f.passage))
                    .map(|f| Location::new(uri.clone(), f.range))
            })
            .collect();
        Ok((locations, complete))
    }

    /// The reference in the configured format (OSIS references are written as OSIS ids)
    fn reference(&self, found: &BibleMatch) -> String {
        let data = self.matcher.data();
        let written = match self.format.book {
            BookStyle::Osis => found.psg.to_osis(data.books()),
            BookStyle::Usfm => found.psg.to_usfm(data.books()),
            _ => self.format.passage(&found.psg, data),
        };
        written.unwrap_or_else(|| found.psg.segments.to_string())
    }

    /// The passage written so the filters can read it back, whatever the configured format
    fn parseable(&self, passage: &Passage) -> String {
        FormatOptions::default()
            .passage(passage, self.matcher.data())
            .unwrap_or_else(|| passage.segments.to_string())
    }
}

/**
A code action that makes edits in a document
- In unnamed documents, as the [`APPLY_EDITS_COMMAND`]: a workspace edit's URI (`file://`) would
  name no buffer
*/
// `WorkspaceEdit::changes` is keyed by `Uri`, which clippy counts as a mutable key
#[allow(clippy::mutable_key_type)]
fn edit_action(
    uri: &Uri,
    title: String,
    edits: Vec<TextEdit>,
    kind: lsp_types::CodeActionKind,
) -> CodeActionOrCommand {
    let (edit, command) = if is_unnamed(uri) {
        let arguments = serde_json::json!({ "edits": edits });
        let command = Command::new(
            title.clone(),
            APPLY_EDITS_COMMAND.into(),
            Some(vec![arguments]),
        );
        (None, Some(command))
    } else {
        let changes = HashMap::from([(uri.clone(), edits)]);
        (Some(lsp_types::WorkspaceEdit::new(changes)), None)
    };
    CodeActionOrCommand::CodeAction(CodeAction {
        title,
        kind: Some(kind),
        edit,
        command,
        ..CodeAction::default()
    })
}

fn lsp_position(index: &LineIndex, offset: usize) -> Position {
    let position = index.position(offset);
    Position::new(position.line as u32 - 1, position.utf16_column as u32 - 1)
}

pub fn lsp_range(index: &LineIndex, start: usize, end: usize) -> Range {
    Range::new(lsp_position(index, start), lsp_position(index, end))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use std::path::Path;

    use lsp_types::{
        PartialResultParams, TextDocumentIdentifier, TextDocumentItem, TextDocumentPositionParams,
        WorkDoneProgressParams,
    };

    use serde_json::json;

    use super::*;

    fn server(text: &str) -> (Server, Uri) {
        let uri = Uri::from_str("file:///notes.md").unwrap();
        let mut server = Server::new(BibleMatcher::default());
        server.did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(uri.clone(), "markdown".into(), 1, text.into()),
        });
        (server, uri)
    }

    fn at(uri: &Uri, line: u32, character: u32) -> TextDocumentPositionParams {
        TextDocumentPositionParams::new(
            TextDocumentIdentifier::new(uri.clone()),
            Position::new(line, character),
        )
    }

    /// The shared completion cases (topos-lib's tests/cases/complete.txt), as an editor asks
    #[test]
    fn completion_cases() {
        let cases = include_str!("../../topos-lib/tests/cases/complete.txt");
        let mut failures = vec![];
        for line in cases
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .filter(|l| !l.starts_with("apply: "))
        {
            let (input, expected) = line.split_once(" =>").unwrap();
            let (mut server, uri) = server(
                input
                    .trim_start_matches("[join] ")
                    .trim_start_matches("[abbreviation] "),
            );
            let format = serde_json::json!({
                "join_adjacent": input.starts_with("[join] "),
                "book": if input.starts_with("[abbreviation] ") { "abbreviation" } else { "name" },
            });
            server
                .configure(serde_json::json!({ "no-config": true, "psg-fmt": format }))
                .unwrap();
            let input = input
                .trim_start_matches("[join] ")
                .trim_start_matches("[abbreviation] ");
            let character = input.encode_utf16().count() as u32;
            let labels: Vec<String> = match server.completion(CompletionParams {
                text_document_position: at(&uri, 0, character),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            }) {
                Some(CompletionResponse::List(list)) => {
                    let items = list.items;
                    items.into_iter().map(|item| item.label).collect()
                }
                _ => vec![],
            };
            let expected: Vec<&str> = expected
                .split(" | ")
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .collect();
            let ok = if expected.is_empty() {
                labels.is_empty()
            } else {
                labels.len() >= expected.len() && labels[..expected.len()] == expected[..]
            };
            if !ok {
                failures.push(format!("{input:?}: expected {expected:?}, got {labels:?}"));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// The shared `apply:` completion cases (`|` is the cursor), as an editor asks
    #[test]
    fn applied_completion_cases() {
        let cases = include_str!("../../topos-lib/tests/cases/complete.txt");
        let mut failures = vec![];
        for case in cases.lines().filter_map(|l| l.strip_prefix("apply: ")) {
            let (input, expected) = case.split_once(" => ").unwrap();
            let (input, expected) = (input.replace("\\n", "\n"), expected.replace("\\n", "\n"));
            let (input, expected) = (input.as_str(), expected.as_str());
            let character = input[..input.find('|').unwrap()].encode_utf16().count() as u32;
            let text = input.replacen('|', "", 1);
            let (server, uri) = server(&text);
            let first = match server.completion(CompletionParams {
                text_document_position: at(&uri, 0, character),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            }) {
                Some(CompletionResponse::List(list)) => list.items.into_iter().next(),
                _ => None,
            };
            let applied = first.and_then(|item| match item.text_edit? {
                lsp_types::CompletionTextEdit::Edit(edit) => {
                    let index = LineIndex::new(&text);
                    let start = index.offset_of_utf16(0, edit.range.start.character as usize)?;
                    let end = index.offset_of_utf16(0, edit.range.end.character as usize)?;
                    let mut applied = text.clone();
                    applied.replace_range(start..end, &edit.new_text);
                    Some(applied)
                }
                lsp_types::CompletionTextEdit::InsertAndReplace(_) => None,
            });
            if applied.as_deref() != Some(expected) {
                failures.push(format!("{input:?}: expected {expected:?}, got {applied:?}"));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn completes_with_utf16_ranges() {
        // `𝄞` is 2 UTF-16 units, so `jn` starts at character 3
        let (server, uri) = server("intro\n𝄞 jn 3:1");
        let Some(CompletionResponse::List(lsp_types::CompletionList {
            items,
            is_incomplete: true,
        })) = server.completion(CompletionParams {
            text_document_position: at(&uri, 1, 9),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        })
        else {
            panic!("no completions");
        };
        let first = &items[0];
        assert_eq!(first.label, "John 3:1");
        assert_eq!(first.filter_text.as_deref(), Some("jn 3:1 John 3:1"));
        let Some(lsp_types::CompletionTextEdit::Edit(edit)) = &first.text_edit else {
            panic!("no edit");
        };
        assert_eq!(
            edit.range,
            Range::new(Position::new(1, 3), Position::new(1, 9))
        );
    }

    #[test]
    fn diagnostics_for_references_and_missing_verses() {
        let (mut server, uri) = server("Fine: jn 3:16\nBad: John 3:99");
        let summary = |server: &Server| -> Vec<(Option<DiagnosticSeverity>, String, Range)> {
            server
                .diagnostics(&uri)
                .diagnostics
                .into_iter()
                .map(|d| (d.severity, d.message, d.range))
                .collect()
        };
        assert_eq!(
            summary(&server),
            [
                (
                    Some(DiagnosticSeverity::INFORMATION),
                    String::from("John 3:16"),
                    Range::new(Position::new(0, 6), Position::new(0, 13))
                ),
                (
                    Some(DiagnosticSeverity::WARNING),
                    String::from("John 3:99 does not exist"),
                    Range::new(Position::new(1, 10), Position::new(1, 14))
                ),
            ]
        );
        // Codes tell them apart
        let codes: Vec<_> = server
            .diagnostics(&uri)
            .diagnostics
            .into_iter()
            .map(|d| d.code)
            .collect();
        assert_eq!(
            codes,
            [
                Some(lsp_types::NumberOrString::String("reference".into())),
                Some(lsp_types::NumberOrString::String("missing".into()))
            ]
        );

        server
            .configure(serde_json::json!({ "no-config": true, "reference-diagnostics": "hint" }))
            .unwrap();
        assert_eq!(summary(&server)[0].0, Some(DiagnosticSeverity::HINT));
        server
            .configure(serde_json::json!({ "no-config": true, "reference_diagnostics": "never" }))
            .unwrap();
        assert_eq!(summary(&server).len(), 1);
    }

    /// A workspace folder with notes on disk, plus one open document (`name` keeps tests that
    /// run at once apart)
    fn workspace(open: &str, name: &str) -> (Server, Uri, PathBuf) {
        let dir = std::env::temp_dir().join(format!("topos-lsp-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("My Notes")).unwrap();
        std::fs::write(
            dir.join("My Notes/a.md"),
            "Jn 3:16 again\nand John 3:14-18\n",
        )
        .unwrap();
        std::fs::write(dir.join("b.txt"), "John 3, John 2; 3:16, Rom 8:28").unwrap();
        std::fs::write(dir.join("image.png"), b"John 3:16\x00").unwrap();
        let uri = workspace::path_to_uri(&dir.join("open.md")).unwrap();
        let mut server = Server::new(BibleMatcher::default());
        server.set_roots(vec![dir.clone()]);
        server.did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(uri.clone(), "markdown".into(), 1, open.into()),
        });
        (server, uri, dir)
    }

    /// `file name: reference text` for each location
    fn found(dir: &Path, locations: &[Location]) -> Vec<String> {
        locations
            .iter()
            .map(|location| {
                let path = workspace::uri_to_path(&location.uri).unwrap();
                let name = path.strip_prefix(dir).unwrap().display().to_string();
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|_| String::from("See John 3:16 here"));
                let index = LineIndex::new(&text);
                let range = location.range;
                let start = index
                    .offset_of_utf16(range.start.line as usize, range.start.character as usize)
                    .unwrap();
                let end = index
                    .offset_of_utf16(range.end.line as usize, range.end.character as usize)
                    .unwrap();
                format!("{name}: {}", &text[start..end])
            })
            .collect()
    }

    #[test]
    fn searches_the_workspace() {
        let (server, uri, dir) = workspace("See John 3:16 here", "search");
        // Go to references: exactly John 3:16, however it is written
        let references = server
            .references(ReferenceParams {
                text_document_position: at(&uri, 0, 6),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: lsp_types::ReferenceContext {
                    include_declaration: true,
                },
            })
            .unwrap();
        assert_eq!(
            found(&dir, &references),
            ["My Notes/a.md: Jn 3:16", "open.md: John 3:16"]
        );

        // One code action per search, for the reference under the cursor
        let actions = server
            .code_actions(CodeActionParams {
                text_document: TextDocumentIdentifier::new(uri.clone()),
                range: Range::new(Position::new(0, 8), Position::new(0, 8)),
                context: lsp_types::CodeActionContext::default(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .unwrap();
        let titles: Vec<_> = actions
            .iter()
            .map(|a| match a {
                CodeActionOrCommand::CodeAction(action) => action.title.clone(),
                CodeActionOrCommand::Command(command) => command.title.clone(),
            })
            .collect();
        assert_eq!(
            titles,
            [
                "Search \"John 3:16\" for explicit overlap",
                "Search \"John 3:16\" for any overlap",
                "Search \"John 3:16\" for exact overlap",
                "Search inside \"John 3:16\"",
            ]
        );

        // Running an action's command finds that search's references
        let run = |index: usize| {
            let CodeActionOrCommand::CodeAction(action) = &actions[index] else {
                panic!("not an action")
            };
            let command = action.command.clone().unwrap();
            let locations = server
                .execute_command(ExecuteCommandParams {
                    command: command.command,
                    arguments: command.arguments.unwrap(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .unwrap();
            found(&dir, &locations)
        };
        assert_eq!(
            run(0),
            [
                "My Notes/a.md: Jn 3:16",
                "My Notes/a.md: John 3:14-18",
                "b.txt: John 2; 3:16",
                "open.md: John 3:16"
            ]
        );
        let any = run(1);
        assert!(any.contains(&"b.txt: John 3".to_string()), "{any:?}");
        assert_eq!(any.len(), 5);

        // The files' references are cached (the binary one too, as having none)
        assert_eq!(server.cache.len(), 3);
        let exact = |server: &Server| {
            let locations = server
                .search(SearchMode::ExactOverlap, "Romans 8:28")
                .unwrap();
            locations.len()
        };
        assert_eq!(exact(&server), 1);
        // Same size and time: the cache is used, so the edit isn't seen yet
        let b = dir.join("b.txt");
        let modified = std::fs::metadata(&b).unwrap().modified().unwrap();
        std::fs::write(&b, "John 3, John 2; 3:16, Rom 8:29").unwrap();
        let file = std::fs::File::options().write(true).open(&b).unwrap();
        file.set_modified(modified).unwrap();
        assert_eq!(exact(&server), 1);
        // A real change is read again
        std::fs::write(&b, "Nothing here now").unwrap();
        assert_eq!(exact(&server), 0);
        // Deleted files are forgotten
        std::fs::remove_file(&b).unwrap();
        exact(&server);
        assert_eq!(server.cache.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_format_completions_hints_and_symbols() {
        let (mut server, uri) = server("See jn 3:16, 17 and Jude 1:5");
        server
            .configure(serde_json::json!({
                "no-config": true,
                "topos": {
                    "no_config": true,
                    "psg-fmt": { "join_adjacent": true, "chapter_in_single_chapter_books": false },
                    "format": "abbreviation"
                }
            }))
            .unwrap();
        let hints = server
            .inlay_hints(InlayHintParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                text_document: TextDocumentIdentifier::new(uri.clone()),
                range: Range::new(Position::new(0, 0), Position::new(1, 0)),
            })
            .unwrap();
        let labels: Vec<_> = hints
            .iter()
            .map(|hint| match &hint.label {
                InlayHintLabel::String(label) => (hint.position, label.clone()),
                InlayHintLabel::LabelParts(_) => panic!("parts"),
            })
            .collect();
        assert_eq!(
            labels,
            [
                (Position::new(0, 15), String::from("Jn 3:16-17")),
                (Position::new(0, 28), String::from("Jude 5"))
            ]
        );
        // Unchanged references get no hint by default; "always" and "osis" show one
        server
            .configure(serde_json::json!({ "no-config": true, "inlay-hints": "osis" }))
            .unwrap();
        let hints = server
            .inlay_hints(InlayHintParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                text_document: TextDocumentIdentifier::new(uri.clone()),
                range: Range::new(Position::new(0, 0), Position::new(1, 0)),
            })
            .unwrap();
        assert_eq!(hints.len(), 2);

        // Bad settings are reported, and the previous settings stay
        let err = server
            .configure(serde_json::json!({ "no-config": true, "psg-fmt": { "joins": true } }))
            .unwrap_err();
        assert!(err.contains("joins"), "{err}");
    }

    #[test]
    fn reformats_and_explains_missing_verses() {
        let (server, uri) = server("See jn 3:16 and rom 8:28, John 1:1\nBad: John 3:99");
        let actions = |line: u32, character: u32| -> Vec<(String, Vec<TextEdit>)> {
            server
                .code_actions(CodeActionParams {
                    text_document: TextDocumentIdentifier::new(uri.clone()),
                    range: Range::new(
                        Position::new(line, character),
                        Position::new(line, character),
                    ),
                    context: lsp_types::CodeActionContext::default(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .unwrap_or_default()
                .into_iter()
                .filter_map(|a| match a {
                    CodeActionOrCommand::CodeAction(action) => {
                        let edits = action
                            .edit
                            .and_then(|e| e.changes)
                            .map(|changes| changes.into_values().flatten().collect())?;
                        Some((action.title, edits))
                    }
                    CodeActionOrCommand::Command(_) => None,
                })
                .collect()
        };
        let on_jn = actions(0, 5);
        assert_eq!(on_jn.len(), 2);
        assert_eq!(on_jn[0].0, "Reformat as \"John 3:16\"");
        assert_eq!(
            on_jn[0].1,
            [TextEdit::new(
                Range::new(Position::new(0, 4), Position::new(0, 11)),
                "John 3:16".into()
            )]
        );
        // Every reference written differently (John 1:1 already is)
        assert_eq!(on_jn[1].0, "Reformat all 2 references in this file");
        assert_eq!(on_jn[1].1.len(), 2);
        // A reference already in the format gets no reformat for itself
        let on_john = actions(0, 28);
        assert_eq!(on_john.len(), 1);
        assert_eq!(on_john[0].0, "Reformat all 2 references in this file");

        // Hovering a verse that doesn't exist says what does
        let hover = server
            .hover(HoverParams {
                text_document_position_params: at(&uri, 1, 11),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .unwrap();
        let HoverContents::Markup(markup) = hover.contents else {
            panic!("no markup");
        };
        assert_eq!(
            markup.value,
            "**John 3:99 does not exist**\n\nJohn 3 has 36 verses"
        );
    }

    #[test]
    fn hovers_and_lists_symbols() {
        let (server, uri) = server("See Rom 8:28 and\nJude 5.");
        let hover = server
            .hover(HoverParams {
                text_document_position_params: at(&uri, 0, 6),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .unwrap();
        let HoverContents::Markup(markup) = hover.contents else {
            panic!("no markup");
        };
        assert_eq!(
            markup.value,
            "**Romans 8:28**\n\n\
             - **Abbreviation**: Rom 8:28\n\
             - **OSIS**: `Rom.8.28`\n\
             - **Book**: Romans, book 45 of 66, 16 chapters\n\
             - **Testament**: New Testament\n\
             - **Genres**: Pauline Epistles, Epistles\n\
             - **Verses**: 1 (8:28)\n\
             - **Location**: line 1, columns 5-12"
        );
        assert_eq!(
            hover.range,
            Some(Range::new(Position::new(0, 4), Position::new(0, 12)))
        );

        let Some(DocumentSymbolResponse::Nested(symbols)) =
            server.document_symbols(DocumentSymbolParams {
                text_document: TextDocumentIdentifier::new(uri),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
        else {
            panic!("no symbols");
        };
        let names: Vec<_> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Romans 8:28", "Jude 1:5"]);
    }

    /// The `hover` setting picks the lines and their order; a full name that is the title already
    /// is left out
    #[test]
    fn hover_lines_follow_the_setting() {
        let hover = |text: &str, settings: Value| {
            let (mut server, uri) = server(text);
            server.configure(settings).unwrap();
            let hover = server
                .hover(HoverParams {
                    text_document_position_params: at(&uri, 0, 1),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .unwrap();
            let HoverContents::Markup(markup) = hover.contents else {
                panic!("no markup");
            };
            markup.value
        };
        let settings = json!({ "no-config": true, "hover": "written,name,bcv,verses", "format": "abbreviation" });
        assert_eq!(
            hover("jn 3; 4:1-2", settings),
            "**Jn 3; 4:1-2**\n\n\
             - **Written**: `jn 3; 4:1-2`\n\
             - **Name**: John 3; 4:1-2\n\
             - **BCV**: `43003001-43003999`, `43004001-43004002`\n\
             - **Verses**: 38 (3:1-36; 4:1-2)"
        );
        let settings = json!({ "no-config": true, "hover": ["name", "testament"] });
        assert_eq!(
            hover("Gen 1:1", settings),
            "**Genesis 1:1**\n\n- **Testament**: Old Testament"
        );
        let settings = json!({ "no-config": true, "hover": [] });
        assert_eq!(hover("Gen 1:1", settings), "**Genesis 1:1**");
    }

    const JHN: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<usx version="3.0"><book code="JHN" style="id">x</book><para style="usfm">3.1</para>
<para style="mt1">John</para>
<chapter number="3" style="c" sid="JHN 3"/>
<para style="s1">Jesus and Nicodemus</para>
<para style="p"><verse number="16" style="v" sid="JHN 3:16"/>For God so loved the world. <verse eid="JHN 3:16"/><verse number="17" style="v" sid="JHN 3:17"/>Jesus said, <char style="wj">I did not come
to condemn.</char> <verse eid="JHN 3:17"/></para>
<chapter eid="JHN 3"/>
<chapter number="4" style="c" sid="JHN 4"/>
<para style="p"><verse number="1" style="v" sid="JHN 4:1"/>Now Jesus learned<verse eid="JHN 4:1"/></para>
<chapter eid="JHN 4"/></usx>"#;

    /// A workspace with notes and a Bible (a folder of USX files) set
    fn with_bible(open: &str, markup: Value, name: &str) -> (Server, Uri, PathBuf) {
        let (mut server, uri, dir) = workspace(open, name);
        let bible = dir.join("Bibles/BSB");
        std::fs::create_dir_all(&bible).unwrap();
        std::fs::write(bible.join("JHN.usx"), JHN).unwrap();
        let settings = json!({
            "no-config": true,
            "bible": bible.to_str().unwrap(),
            "markup": markup,
        });
        server.configure(settings).unwrap();
        (server, uri, dir)
    }

    fn book_uri() -> Uri {
        Uri::from_str("topos-bible://BSB/JHN").unwrap()
    }

    #[test]
    fn bible_text_in_hover_and_whole_books() {
        let (server, uri, dir) = with_bible("See John 3:16-17 and John 4", json!({}), "bible");
        let hover = server
            .hover(HoverParams {
                text_document_position_params: at(&uri, 0, 6),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })
            .unwrap();
        let HoverContents::Markup(markup) = hover.contents else {
            panic!("no markup");
        };
        // The heading before 3:16 belongs to it; the info string says what to color: the heading,
        // verse numbers, and words of Jesus
        assert!(
            markup.value.starts_with(
                "**John 3:16-17**\n\n```topos-bible 0:0:19:3 1:0:4:0 1:33:4:0 1:50:26:2\nJesus and Nicodemus\n[16] For God so loved the world. [17] Jesus said, I did not come to condemn.\n```\n\n- **Abbreviation**"
            ),
            "{}",
            markup.value
        );

        // The whole book, with chapter labels; its title is a heading
        let book = server
            .book(BookParams {
                uri: book_uri().to_string(),
            })
            .unwrap();
        assert_eq!(book.title, "John");
        assert_eq!(
            book.text,
            "John\nJohn 3\nJesus and Nicodemus\n[16] For God so loved the world. [17] Jesus said, I did not come to condemn.\nJohn 4\n[1] Now Jesus learned"
        );
        assert!(
            server
                .book(BookParams {
                    uri: "topos-bible://KJV/JHN".into()
                })
                .is_err()
        );

        // Go to definition: from the first verse's number to the last one's end
        let definition = |character| {
            server.definition(GotoDefinitionParams {
                text_document_position_params: at(&uri, 0, character),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
        };
        let Some(GotoDefinitionResponse::Scalar(location)) = definition(6) else {
            panic!("no definition");
        };
        assert_eq!(location.uri, book_uri());
        assert_eq!(
            location.range,
            Range::new(Position::new(3, 0), Position::new(3, 76))
        );
        let Some(GotoDefinitionResponse::Scalar(chapter)) = definition(23) else {
            panic!("no definition");
        };
        assert_eq!(chapter.range.start, Position::new(5, 0));

        // Tokens: chapter labels, headings, verse numbers, and words of Jesus (each on its line)
        let tokens = server
            .semantic_tokens(SemanticTokensParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                text_document: TextDocumentIdentifier::new(book_uri()),
            })
            .data;
        let mut absolute = vec![];
        let (mut line, mut character) = (0, 0);
        for t in &tokens {
            line += t.delta_line;
            character = if t.delta_line == 0 {
                character + t.delta_start
            } else {
                t.delta_start
            };
            absolute.push((
                line,
                character,
                t.length,
                Token::LEGEND[t.token_type as usize],
            ));
        }
        assert_eq!(
            absolute,
            [
                (0, 0, 4, "heading"),
                (1, 0, 6, "chapterNumber"),
                (2, 0, 19, "heading"),
                (3, 0, 4, "verseNumber"),
                (3, 33, 4, "verseNumber"),
                (3, 50, 26, "wordsOfJesus"),
                (4, 0, 6, "chapterNumber"),
                (5, 0, 3, "verseNumber"),
            ]
        );
        // Notes have none
        let notes = server.semantic_tokens(SemanticTokensParams {
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            text_document: TextDocumentIdentifier::new(uri.clone()),
        });
        assert!(notes.data.is_empty());

        // References from a verse in the book: every reference that names it
        let references = |line, character| {
            server.references(ReferenceParams {
                text_document_position: at(&book_uri(), line, character),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: lsp_types::ReferenceContext {
                    include_declaration: true,
                },
            })
        };
        let verse = references(3, 10).unwrap();
        assert_eq!(
            found(&dir, &verse[..3]),
            [
                "My Notes/a.md: Jn 3:16",
                "My Notes/a.md: John 3:14-18",
                "b.txt: John 2; 3:16",
            ]
        );
        // The open note's `John 3:16-17` (the book itself isn't searched)
        assert_eq!((verse.len(), verse[3].uri.clone()), (4, uri.clone()));
        assert_eq!(verse[3].range.end, Position::new(0, 16));
        // On the chapter's label, anything in the chapter
        assert_eq!(references(1, 2).unwrap().len(), 5);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The markup settings: labels, what is shown, and words of Jesus left uncolored
    #[test]
    fn markup_settings() {
        let markup = json!({
            "verse_numbers": { "display": "{chapter}:{verse}", "color": "blue" },
            "chapter-numbers": { "show": false },
            "words_of_jesus": { "show": false },
            "headings": { "show": false },
        });
        let (server, _, dir) = with_bible("John 3:16", markup, "markup");
        let book = server
            .book(BookParams {
                uri: book_uri().to_string(),
            })
            .unwrap();
        assert_eq!(
            book.text,
            "3:16 For God so loved the world. 3:17 Jesus said, I did not come to condemn.\n4:1 Now Jesus learned"
        );
        let tokens = server.semantic_tokens(SemanticTokensParams {
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            text_document: TextDocumentIdentifier::new(book_uri()),
        });
        assert!(
            tokens
                .data
                .iter()
                .all(|t| t.token_type == Token::VerseNumber as u32)
        );
        assert_eq!(tokens.data.len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Templates for headings and words of Jesus, and elements that aren't shown
    #[test]
    fn markup_templates() {
        let markup = json!({
            "headings": { "display": "## {text}" },
            "words_of_jesus": { "display": "“{text}”" },
            "verse_numbers": { "show": false },
            "chapter_numbers": { "display": "Chapter {chapter}" },
        });
        let (server, _, dir) = with_bible("John 3:16", markup, "templates");
        let book = server
            .book(BookParams {
                uri: book_uri().to_string(),
            })
            .unwrap();
        assert_eq!(
            book.text,
            "## John\nChapter 3\n## Jesus and Nicodemus\nFor God so loved the world. Jesus said, “I did not come to condemn.”\nChapter 4\nNow Jesus learned"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Hovering a verse in a whole book counts its references; go to declaration opens the book
    /// like definition; completions resolve to the passage's text
    #[test]
    fn verse_hover_declaration_and_completion_text() {
        let (server, uri, dir) = with_bible("See John 3:16", json!({}), "verse-hover");
        let hover = |uri: &Uri, line, character| {
            let hover = server.hover(HoverParams {
                text_document_position_params: at(uri, line, character),
                work_done_progress_params: WorkDoneProgressParams::default(),
            })?;
            match hover.contents {
                HoverContents::Markup(markup) => Some(markup.value),
                _ => None,
            }
        };
        assert_eq!(
            hover(&book_uri(), 3, 10).unwrap(),
            "**John 3:16**\n\n4 references in 3 files"
        );
        assert_eq!(
            hover(&book_uri(), 1, 1).unwrap(),
            "**John 3**\n\n5 references in 3 files"
        );
        // Out of time: the hover says `???`
        let (_, complete) = server
            .search_until(
                SearchMode::ExplicitOverlap,
                "John 3:16",
                Some(Instant::now()),
            )
            .unwrap();
        assert!(!complete);

        let declaration = server.definition(GotoDefinitionParams {
            text_document_position_params: at(&uri, 0, 6),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        });
        assert!(
            matches!(declaration, Some(GotoDefinitionResponse::Scalar(l)) if l.uri == book_uri())
        );

        let (mut server, uri) = server_with("John 3:1", &server);
        server.did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem::new(
                uri.clone(),
                "markdown".into(),
                1,
                "John 3:1".into(),
            ),
        });
        let Some(CompletionResponse::List(list)) = server.completion(CompletionParams {
            text_document_position: at(&uri, 0, 8),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        }) else {
            panic!("no completions");
        };
        let item = list
            .items
            .into_iter()
            .find(|i| i.label == "John 3:16")
            .unwrap();
        let resolved = server.resolve_completion(item);
        let Some(lsp_types::Documentation::MarkupContent(doc)) = resolved.documentation else {
            panic!("no documentation");
        };
        // No token list for completion plugins to show; it is in the data instead
        assert_eq!(
            doc.value,
            "```topos-bible\nJesus and Nicodemus\n[16] For God so loved the world.\n```"
        );
        assert_eq!(resolved.data.unwrap()["tokens"], "0:0:19:3 1:0:4:0");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The same settings, for another document
    fn server_with(_text: &str, other: &Server) -> (Server, Uri) {
        let mut server = Server::new(BibleMatcher::default());
        server
            .configure(Value::Object(other.editor.clone()))
            .unwrap();
        (server, Uri::from_str("file:///typing.md").unwrap())
    }

    /// Insertions: a code action per insertion, with its own markup over the others
    #[test]
    fn insertions() {
        let (mut server, uri, dir) =
            with_bible("Read John 3:16-17 today\nnext", json!({}), "insert");
        let mut settings = server.editor.clone();
        settings.insert(
            "insertions".into(),
            json!([
                {
                    "name": "Callout",
                    "display": "> [!bible] {reference}\n{text}",
                    "line_prefix": "> ",
                    "markup": { "headings": { "show": false }, "verse_numbers": { "display": "{verse}" } }
                },
                { "name": "Plain", "markup": { "verse_numbers": { "show": false }, "headings": { "show": false } } },
                { "name": "Line", "replace": "line", "display": "> {reference}: {text}", "markup": { "verse_numbers": { "show": false }, "headings": { "show": false } } },
                { "name": "Inline", "replace": "inline", "display": "“{text}” ({reference})", "markup": { "verse_numbers": { "show": false }, "headings": { "show": false } } }
            ]),
        );
        server.configure(Value::Object(settings)).unwrap();
        let actions = server
            .code_actions(CodeActionParams {
                text_document: TextDocumentIdentifier::new(uri.clone()),
                range: Range::new(Position::new(0, 6), Position::new(0, 6)),
                context: lsp_types::CodeActionContext::default(),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .unwrap();
        let inserts: Vec<(String, TextEdit)> = actions
            .into_iter()
            .filter_map(|a| match a {
                CodeActionOrCommand::CodeAction(a) if a.title.starts_with("Insert") => {
                    let edit = a.edit?.changes?.into_values().next()?.pop()?;
                    Some((a.title, edit))
                }
                _ => None,
            })
            .collect();
        let at = Range::new(Position::new(0, 23), Position::new(0, 23));
        assert_eq!(
            inserts,
            [
                (
                    String::from("Insert Callout"),
                    TextEdit::new(at, "\n> [!bible] John 3:16-17\n> 16 For God so loved the world. 17 Jesus said, I did not come to condemn.".into())
                ),
                (
                    String::from("Insert Plain"),
                    TextEdit::new(at, "\nFor God so loved the world. Jesus said, I did not come to condemn.".into())
                ),
                // The whole line
                (
                    String::from("Insert Line"),
                    TextEdit::new(
                        Range::new(Position::new(0, 0), Position::new(0, 23)),
                        "> John 3:16-17: For God so loved the world. Jesus said, I did not come to condemn.".into()
                    )
                ),
                // Just the reference: `Read ` and ` today` stay
                (
                    String::from("Insert Inline"),
                    TextEdit::new(
                        Range::new(Position::new(0, 5), Position::new(0, 17)),
                        "“For God so loved the world. Jesus said, I did not come to condemn.” (John 3:16-17)".into()
                    )
                ),
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `markup.width` breaks long lines at spaces, keeping every byte offset
    #[test]
    fn width_wraps_at_spaces() {
        assert_eq!(bible::wrap("aa bb cc dd\nee ff", 5), "aa bb\ncc dd\nee ff");
        assert_eq!(bible::wrap("a verylongword b", 4), "a\nverylongword\nb");
        assert_eq!(bible::wrap("“é” ü ö", 3), "“é”\nü ö");
        let (server, uri, dir) = with_bible("See John 3:16", json!({ "width": 30 }), "width");
        let book = server
            .book(BookParams {
                uri: book_uri().to_string(),
            })
            .unwrap();
        assert!(
            book.text.lines().all(|l| l.chars().count() <= 30),
            "{}",
            book.text
        );
        assert!(
            book.text.contains("\n[16] For God so loved the\nworld."),
            "{}",
            book.text
        );
        // Go to definition still finds the verse
        let Some(GotoDefinitionResponse::Scalar(location)) =
            server.definition(GotoDefinitionParams {
                text_document_position_params: at(&uri, 0, 6),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
        else {
            panic!("no definition");
        };
        assert_eq!(location.range.start, Position::new(3, 0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A Bible's `JHN.usx` with only John 3:16, worded like the KJV
    const KJV_JHN: &str = r#"<usx version="3.0"><book code="JHN" style="id">John</book>
<chapter number="3" style="c" sid="JHN 3"/>
<para style="p"><verse number="16" style="v" sid="JHN 3:16"/>For God so loved the world, that he gave his only begotten Son.<verse eid="JHN 3:16"/></para>
<chapter eid="JHN 3"/></usx>"#;

    /// Several Bibles: a reference followed by one's name or alias uses it, others the default
    #[test]
    fn several_bibles() {
        let open = "See John 3:16 KJV and John 3:16.\nJohn 3:17 AV\nJohn 3:16 K";
        let (mut server, uri, dir) = with_bible(open, json!({}), "several");
        let kjv = dir.join("Bibles/KJV");
        std::fs::create_dir_all(&kjv).unwrap();
        std::fs::write(kjv.join("JHN.usx"), KJV_JHN).unwrap();
        let mut settings = server.editor.clone();
        settings.insert(
            "bibles".into(),
            json!({ "KJV": { "path": kjv.to_str().unwrap(), "aliases": ["AV"] } }),
        );
        let plain = json!({ "verse_numbers": { "show": false }, "headings": { "show": false } });
        settings.insert(
            "insertions".into(),
            json!([{ "name": "Plain", "markup": plain }]),
        );
        server.configure(Value::Object(settings.clone())).unwrap();

        let hover = |server: &Server, line, character| {
            let hover = server.hover(HoverParams {
                text_document_position_params: at(&uri, line, character),
                work_done_progress_params: WorkDoneProgressParams::default(),
            });
            match hover.map(|h| h.contents) {
                Some(HoverContents::Markup(markup)) => markup.value,
                _ => panic!("no hover"),
            }
        };
        // The Bible a reference names, labeled, else the default (the `bible` setting's)
        let named = hover(&server, 0, 6);
        assert!(
            named.starts_with("**John 3:16** (KJV)\n\n*KJV*\n```topos-bible"),
            "{named}"
        );
        assert!(
            named.contains("that he gave") && !named.contains("*BSB*"),
            "{named}"
        );
        // On the translation's name too
        assert_eq!(hover(&server, 0, 15), named);
        let default = hover(&server, 0, 24);
        assert!(default.starts_with("**John 3:16**\n\n*BSB*\n"), "{default}");
        assert!(!default.contains("that he gave"), "{default}");

        // The KJV's own verse counts (only 3:16 here), with its name in the message
        let messages: Vec<String> = server
            .diagnostics(&uri)
            .diagnostics
            .into_iter()
            .filter(|d| d.severity == Some(DiagnosticSeverity::WARNING))
            .map(|d| d.message)
            .collect();
        assert_eq!(messages, ["John 3:17 does not exist in the KJV"]);

        // Go to definition opens the named Bible's book
        let Some(GotoDefinitionResponse::Scalar(location)) =
            server.definition(GotoDefinitionParams {
                text_document_position_params: at(&uri, 0, 6),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
        else {
            panic!("no definition");
        };
        assert_eq!(location.uri.as_str(), "topos-bible://KJV/JHN");
        let book = server
            .book(BookParams {
                uri: "topos-bible://KJV/JHN".into(),
            })
            .unwrap();
        assert!(book.text.contains("that he gave"), "{}", book.text);

        // Completing a translation, whose documentation is that Bible's text
        let Some(CompletionResponse::List(list)) = server.completion(CompletionParams {
            text_document_position: at(&uri, 2, 11),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        }) else {
            panic!("no completions");
        };
        let item = list.items.into_iter().next().unwrap();
        assert_eq!(item.label, "KJV");
        let resolved = server.resolve_completion(item);
        let Some(lsp_types::Documentation::MarkupContent(doc)) = resolved.documentation else {
            panic!("no documentation");
        };
        assert!(doc.value.contains("that he gave"), "{}", doc.value);

        // The Bibles, the default first
        let bibles = server.bibles().bibles;
        let names: Vec<(&str, &[String])> = bibles
            .iter()
            .map(|b| (b.name.as_str(), b.aliases.as_slice()))
            .collect();
        assert_eq!(names, [("BSB", &[][..]), ("KJV", &["AV".to_string()][..])]);
        assert_eq!(bibles[1].books, ["JHN"]);

        // The same verse in the other Bible's book
        let same = |line, bible: &str| {
            server
                .same_verse(SameVerseParams {
                    uri: "topos-bible://BSB/JHN".into(),
                    position: Position::new(line, 1),
                    bible: bible.into(),
                })
                .unwrap()
                .map(|l| (l.uri.to_string(), l.range.start))
        };
        let bsb = server
            .book(BookParams {
                uri: "topos-bible://BSB/JHN".into(),
            })
            .unwrap();
        let line_of =
            |text: &str, find: &str| text.lines().position(|l| l.contains(find)).unwrap() as u32;
        let kjv_book = server
            .book(BookParams {
                uri: "topos-bible://KJV/JHN".into(),
            })
            .unwrap();
        let kjv_316 = line_of(&kjv_book.text, "[16]");
        assert_eq!(
            same(line_of(&bsb.text, "[16]"), "av"),
            Some(("topos-bible://KJV/JHN".into(), Position::new(kjv_316, 0)))
        );
        // A chapter it lacks (and nothing after it) goes to the book's start
        assert_eq!(
            same(line_of(&bsb.text, "John 4"), "KJV").map(|(_, p)| p),
            Some(Position::new(0, 0))
        );
        assert!(
            server
                .same_verse(SameVerseParams {
                    uri: "topos-bible://BSB/JHN".into(),
                    position: Position::new(0, 0),
                    bible: "NIV".into(),
                })
                .is_err()
        );

        // Insertions: from the named Bible, else one per Bible
        let inserts = |server: &Server, character| -> Vec<String> {
            server
                .code_actions(CodeActionParams {
                    text_document: TextDocumentIdentifier::new(uri.clone()),
                    range: Range::new(Position::new(0, character), Position::new(0, character)),
                    context: lsp_types::CodeActionContext::default(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .unwrap()
                .into_iter()
                .filter_map(|a| match a {
                    CodeActionOrCommand::CodeAction(a) if a.title.starts_with("Insert") => {
                        Some(a.title)
                    }
                    _ => None,
                })
                .collect()
        };
        assert_eq!(inserts(&server, 6), ["Insert Plain (KJV)"]);
        assert_eq!(
            inserts(&server, 24),
            ["Insert Plain (BSB)", "Insert Plain (KJV)"]
        );

        // `hover-bibles` adds Bibles after the named one; `default-bible` changes the default
        settings.insert("hover-bibles".into(), json!(["kjv"]));
        settings.insert("default-bible".into(), json!("AV"));
        server.configure(Value::Object(settings.clone())).unwrap();
        let both = hover(&server, 0, 24);
        assert!(both.starts_with("**John 3:16**\n\n*KJV*\n"), "{both}");
        settings.insert("hover-bibles".into(), json!(["BSB"]));
        server.configure(Value::Object(settings.clone())).unwrap();
        let both = hover(&server, 0, 24);
        let (kjv, bsb) = (both.find("*KJV*").unwrap(), both.find("*BSB*").unwrap());
        assert!(kjv < bsb, "{both}");
        settings.insert("hover-bibles".into(), json!(["NIV"]));
        assert!(server.configure(Value::Object(settings)).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
