# Roadmap

The phases are in order, and each one leaves the repo working.
- Findings and file references: [doc/review.md](./doc/review.md)
- Target design: [doc/architecture.md](./doc/architecture.md)

## Phase 0: Get back to green

- [x] Make the `pdf` feature opt-in, and gate the `PDFMatchError` import and variant
- [x] Replace `.from_tuple()` calls with `.map(...)`, or fix `from_nested_tuple`
- [ ] Pin the `htmloc` git rev (the `Selection { bytes }` change is fixed)
- [x] Fix the CLI and binding call sites for the `MatchResult` return type
- [ ] Delete dead code: `crates/topos-parser` and the placeholder exports (the old segment parsers and unlinked files are already gone)
- [x] Replace the absolute-path HTML test fixture with a file in the repo (the test is `#[ignore]`d because `htmloc` takes minutes on it)
- [ ] Add CI: `test` + `clippy` for both default features and `--no-default-features`

## Phase 1: Correctness, with a test corpus first

- [x] Add `tests/cases/search.txt`, a table of `input => expected references`, run by one test
- [x] Fix the `5:12-6:6` resolution bug
- [x] Restore the "chapters until a verse is seen" rule (and `;` starts chapters again)
- [x] Handle single-chapter books (`Jude 5`, `3 John 5`)
- [x] Always match all books and filter by `BookId` afterward: Searching `John` only matches `1 John` [#8](https://github.com/MasterTemple/topos-bible/issues/8)
- [x] Escape regex keys, sort them longest-first, and drop the hard-coded `1..=66` (plus ASCII word boundaries: search is 31x faster)
- [x] Implement `Ord` for `Segment` by hand, and fix `Passage::contains` / `Segments::fully_contains`
- [x] Restrict Roman numerals to chapter position, whole words, and canonical forms; use `is_ascii_digit`
- [x] Validate against versification (Validate match segments based on actual verse in the text)
- [x] Add a fuzz target (`cargo fuzz run search` in `fuzz/`), plus a seeded randomized test of the same invariants in CI

## Phase 2: One parser and a resolve stage

- [x] One lexer/parser that produces a lossless syntax tree with spans, replacing minimal, verbose and the autocomplete regexes (`segments/grammar`)
- [x] A resolver stage (`segments::resolve`) and document context (`matcher::context`)
  - [x] Parse older formats / Roman numerals / sub-verses: `Matth. x, 8` [#5](https://github.com/MasterTemple/topos-bible/issues/5)
  - [x] `cf` and `ff` [#10](https://github.com/MasterTemple/topos-bible/issues/10)
  - [x] Support dashes in book names for justified text [#11](https://github.com/MasterTemple/topos-bible/issues/11)
  - [x] Contextual parsing: verse headings in a document about one book [#6](https://github.com/MasterTemple/topos-bible/issues/6)
- [x] Rules plus `ambiguous` abbreviations in the book data (instead of a numeric confidence score): Reduce false positives (`"is"` for Isaiah) [#3](https://github.com/MasterTemple/topos-bible/issues/3)
- [x] One `ToposError` enum, and no `unwrap` in library code (except the PDF module, rewritten in Phase 3)
- [x] Byte spans as the source of truth, with a `LineIndex` that gives byte, char and UTF-16 columns

## Phase 3: Crate split and formats

- [x] Split the core (`topos-lib`) from `topos-formats` (feature-gated adapters behind a `Format` trait)
- [x] Generalize match location [#9](https://github.com/MasterTemple/topos-bible/issues/9): plain text, HTML, SRT (multi-line cues), WebVTT, SBV, PDF (MuPDF 0.8; `topos-pdf` removed), EPUB, JSON, XML
- [x] EPUB CFIs identical to EPUB++'s (real spine positions, UTF-16 offsets across text chunks, one range CFI from `<body>`, optional `[id]` assertions), checked against its `BookText.cfi` on 97 books

## Phase 4: Formatting, interop, autocomplete

- [x] Formatter (`FormatOptions`, on resolved passages): Provide user-specified formatting options [#4](https://github.com/MasterTemple/topos-bible/issues/4)
- [x] OSIS parse and format, plus a BCV integer key: Create a standard format for specifying segment ranges
- [x] Autocomplete `BibleMatcher::complete(text, cursor, opts)` that returns `TextEdit`s: books, chapters, and verses, written with `FormatOptions`
- [x] Book filters as data: `filter::BookScope` (testaments, genres, and books, included and excluded; serde, snake or camel case) and the books they keep (the FFI's `Topos.booksIn(query)`)
- [x] Books alone as the whole book: `BibleData::parse_book` (the FFI's `parse` reads `Gen` as `Genesis 1-50`) and, opt-in for running text, `BibleMatcher::with_books_alone` (the FFI's `ToposOptions.booksAlone`); a single-chapter book's whole chapter is written as its verses (`Jude 1:1-25`)

## Phase 5: CLI

- [x] Stream results while the directory walk runs (`--sort` waits and sorts by path)
- [x] Output modes: rg-style `path:line:col: ref` (default when piped), grouped and colored (TTY), real NDJSON (`-m json`), counts, aligned tables
- [x] Context: `-C/-A/-B` lines with highlighting (`-C 0` for the matched line alone; EPUBs and PDFs too, a paragraph or a line of a page), `--format name|abbreviation|osis`, `--color`
- [x] Choose a format by file extension through `topos-formats`, skip binary files, and report unreadable files on stderr (exit codes like ripgrep)
- [x] Clarify inputs: `topos [PATH]...` (paths only, like ripgrep), with `--text` or piped stdin for literal text (`-i` already filters by passage)
- [x] `--config` for custom book, genre and versification data (JSON, via `BibleDataInput`)
- [x] Cache search results [#2](https://github.com/MasterTemple/topos-bible/issues/2) (`--cache`)
- [x] Verses only by default, `--chapters` for whole chapters (also when every passage filter names no verse; `--skip-single-chapter` also drops `Jude 25`) and `--translation NAME` (references followed by a translation), in the library (`BibleFilter::filter_verses`, `filter_translation`) and the FFI (`ToposQuery.verses`, `translation`)
- [x] `--epub-links wiki|markdown`: an EPUB++ link to each reference in EPUBs (`--cfi-assertions` for `[id]` assertions)
- [x] EPUB positions in `-m json` (spine index, chapter, offsets and lines in the book's text)
- [x] `topos-bible-index`: a persistent, compact index (about 15 bytes a reference) with checks for changed files, per-device packs that sync without conflicts, and filtered, sorted, paged queries; in the bindings (`ToposIndex`) and the CLI (`-m index`); the Obsidian plugin keeps a million references in it and searches only changed files
- [x] File walking, per-format search, and the cache moved from the CLI into `topos-bible-index` (`files`), shared by the CLI, LSP, and FFI (`Topos.searchFiles` with `ToposFiles`, including `cacheDir`)
- [x] `ToposBook` in the bindings (feature `markup`): a USX or OSIS book you bring, with passages as HTML or text (verses found by `topos-bible-markup::verses`), for the Obsidian plugin's previews (moved to logoxide with the markup crate, October 2026: `logoxide-markup`, and the book in `logoxide-ffi`)
- [x] EPUBs searched in WebAssembly (`Topos.indexEpub`, the CLI's code), so the Obsidian plugin searches books in its Web Worker instead of through EPUB++ (kept for books the Rust ZIP reader rejects)

## Phase 6: Bindings and tooling

- [x] A `topos-ffi` façade (BoltFFI 0.31) with owned data types and offsets in the caller's unit
- [x] Remove the old `topos-py`, `topos-ts`, and `topos-ts-boltffi` test crates
- [x] Python through BoltFFI (`boltffi pack python` builds a working wheel)
- [x] An LSP server (`topos-lsp`): completion, hover with the normalized reference, and document symbols (moved to logoxide as `logoxide-lsp`, October 2026)
- [x] LSP diagnostics for references that do not exist (`BibleMatcher::problems`)
- [x] LSP Bible text from a folder of USX files (`bible`, `markup`): passage text in hover, go to definition into a whole book (`topos-bible://BSB/JHN`, colored with semantic tokens), and references from a verse
- [x] The Neovim plugin moved to its own repository, `logoxide.nvim`
- [ ] Incremental parsing (re-scan only the edited lines). Deferred: search runs at about 30 MiB/s, so re-searching a document on each LSP request is fast enough for now
- [x] Criterion benchmarks (`cargo bench -p topos-lib`)

## Phase 7: Multiple translations

Plan: [doc/translations.md](./doc/translations.md)
- [x] `BibleData::parse_with_translation` and `Translations`: a reference and the translation named after it (`John 3:16 (KJV)`), as a new `TranslatedPassage`
- [x] Search, problems, and completion with translations (`tests/cases/translations.txt`), without slowing plain search
- [x] A translation's own chapter and verse counts (`Translations::set_chapter_verses`, from a book's USX with `verses::chapter_verses`)
- [x] The FFI: `ToposTranslations`, `parseWithTranslation`, `searchWithTranslations`, `completeWithTranslations`, `ToposBook.chapterVerses` (moved to logoxide, October 2026)
- [ ] Mapping verses between versifications (Malachi 4:1 KJV = 3:19 MT), in logoxide
- [x] Several Bibles in the LSP (`bibles`, `default-bible`, `hover-bibles`, `topos/bibles`)
- [x] Several Bibles in the Neovim plugin (`bibles`, `:ToposBible`, `:ToposParallel`) and the Obsidian plugin (named references, verse counts, compare in embeds, parallel tabs)
- [ ] logoxide, a library and CLI (`lx`) above topos: translations as versioned git packages, cached, shared, and installed from USX, SWORD, and more: [doc/logoxide.md](./doc/logoxide.md)
