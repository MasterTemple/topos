# Caching, sharing, and managing translations

A plan for a Bible library that sits above topos: it reads translations from many package formats
(USX, SWORD through sword-modules, and others), caches them, shares them between apps and devices,
and installs, updates, and removes them. Its working name here is **bibliotheca**; the name is
open.

[translations.md](./translations.md) covers what is done: references that name a translation
(`John 3:16 KJV`), each translation's own verse counts, and several Bibles in the LSP, Neovim, and
Obsidian. Each front end finds and reads Bibles on its own today, and that's what this plan
replaces.

## Where things stand

| | Finds Bibles | Reads text | Verse counts | Caches |
|---|---|---|---|---|
| topos-lsp | `bible`/`bibles` folders of USX | `bible.rs` (markup) | scanned on load | parsed and rendered books, in memory |
| Neovim | through the LSP | through the LSP | through the LSP | — |
| Obsidian | USX folders in the vault | `ToposBook` (FFI) | `VerseCounts` → `verse-counts.json` | file-plus-plus's per-tab state |
| sword-modules | a SWORD library folder | its drivers, then markup | its 18 versifications | none |

Four copies of "find Bibles, read them, count verses" exist, and only sword-modules can install
anything.

## Layers

```
topos-bible          references: parsing, search, Translations (names, aliases, counts)
topos-bible-markup   Bible text as one tree (USX, OSIS, ThML, GBF, TEI), Flat, HTML
sword-modules        SWORD modules: read, install, update (one source among several)
bibliotheca          translations from any source: one Translation API, cache, registry,
                     installs, versification data; builds topos's Translations
front ends           CLI, LSP, FFI/WASM, Neovim, Obsidian
```

- topos stays as it is: it never depends on bibliotheca, and it only learns about translations
  through `Translations` (names, aliases, counts).
- sword-modules stays a reader and installer for one format. bibliotheca depends on it behind a
  feature, the same way it depends on a USX reader or an OSIS reader.
- topos-bible-markup is where every source's text ends up, so the front ends render text the same
  way whatever the source.

## The API

```rust
/// A translation's stable key: its source and its code there (`sword:KJV`, `usx:BSB`,
/// `ebible:engwebp`). Names users type (`KJV`) are aliases that resolve to one.
pub struct TranslationId(String);

pub struct TranslationInfo {
    pub id: TranslationId,
    pub name: String,              // "King James Version"
    pub abbreviation: String,      // "KJV", what references name by default
    pub aliases: Vec<String>,
    pub language: String,          // BCP 47: "en", "grc"
    pub direction: Direction,
    pub license: License,          // public domain, a named license, or "all rights reserved"
    pub versification: Versification, // own counts, plus a named system when known (KJV, NRSV, MT, ...)
    pub books: Vec<BookId>,
    pub source: SourceInfo,        // format, location, version, content hash
}

pub trait Source {
    fn info(&self) -> &TranslationInfo;
    fn book(&self, book: BookId) -> Result<Book>;    // markup Nodes + VerseSpans
}

pub struct Library { /* registry + sources + cache */ }
impl Library {
    pub fn open(root: &Path) -> Result<Library>;
    pub fn translations(&self) -> &[TranslationInfo];
    pub fn get(&self, name_or_alias: &str) -> Option<&TranslationInfo>;
    pub fn passage(&self, id: &TranslationId, passage: &Passage) -> Result<Vec<Node>>;
    pub fn topos_translations(&self) -> topos_bible::data::translations::Translations;
    // management: see below
}
```

`Source` adapters, each behind a feature:

- **USX**: a folder, a zip, a Digital Bible Library bundle, or eBible.org's USX zip (moved from
  the LSP's `bible.rs` and the plugin's `core/bibles.ts`, which both name books from file names
  and `<book code>`)
- **SWORD**: through sword-modules (`VerseModule::passage` gives each verse's markup)
- **OSIS XML**: a whole Bible in one file
- **USFM**: needs a USFM-to-tree parser in topos-bible-markup (USX is USFM as XML, so the tree
  is the same)
- Later: Zefania XML, and JSON from gracious-tech's collection

## Cache

Reading a source is slow in different ways: parsing a USX book takes milliseconds (the whole BSB
about 100 ms), a SWORD module decompresses blocks, and the LSP renders whole books. Each
translation gets a cache, derived from its source and never edited by hand:

```
<cache>/<translation id>/
  manifest.postcard   TranslationInfo, the versification, each book's offset, the source's hash,
                      and the cache format's version
  books.postcard      each book's tree (markup Nodes) and VerseSpans, read one book at a time
  text.postcard       later: each verse's plain text, for full-text search across translations
```

- **Keys**: the source's content hash, or its size and mtime first, like topos-bible-index
  checks files, plus the cache format's version. A change to either rebuilds that translation.
- **A tree, not rendered text**: each front end renders differently (the LSP's markup templates,
  the plugin's HTML), so the cache keeps the parsed tree. A book loads without parsing, and a
  passage loads without the rest of its book.
- **Per device**: the cache can always be rebuilt, so it never syncs.
  - CLI and LSP: `$XDG_CACHE_HOME/bibliotheca/`
  - Obsidian desktop: the plugin's folder, excluded from sync, or IndexedDB
  - Obsidian mobile: IndexedDB (the plugin's folder syncs)
  - The phone apps: their cache folder
- **Verse counts** are part of the manifest, so the plugin's `verse-counts.json` and the LSP's
  scan on load both go away. `Library::topos_translations()` builds topos's `Translations` from
  the manifests, without opening a book.
- **In memory**: the LSP keeps its `Arc<Book>` and `Rendered` caches, now filled from the disk
  cache instead of parsing.

## Sharing

Three things get shared, each a different way:

1. **Between apps on one machine** (CLI, LSP/Neovim, Obsidian desktop): one library folder,
   `$XDG_DATA_HOME/bibliotheca/`, with a registry (`library.toml`: installed translations,
   enabled ones, the default, aliases, order, compare list) and the installed packages
   (`sword/`, `usx/`). topos's `config.toml` points at it (`library = "~/.local/share/bibliotheca"`)
   or names translations from it (`bibles = ["KJV", "BSB"]`), so the LSP stops needing paths.
   Obsidian on desktop can use it too, as well as its own vault folders.
2. **Between devices**: sources sync, caches don't.
   - Obsidian: Bibles in the vault sync with the vault (as now). The plugin's settings hold the
     registry part: aliases, the default, the compare list.
   - Large or licensed modules: sync an install recipe instead (repository, module, version).
     Each device downloads its own copy, so a phone isn't sent a SWORD library it never opens.
3. **Between people**: share recipes and names, never files. Many translations (ESV, NIV, NASB)
   are copyrighted, so bibliotheca shares a translation as its id, source, and version, and each
   person installs it from where it's published. A note's `John 3:16 ESV` is already portable;
   resolving `ESV` to an installed translation is the library's alias table. A list of common
   abbreviations (ESV, NIV, KJV, NASB, CSB, NLT, NKJV, LSB, BSB, WEB, ...) ships with bibliotheca,
   not topos, so a note's names mean the same thing everywhere.

## Managing

| Operation | What it does |
|---|---|
| `list` | Installed translations: id, abbreviation, language, license, size, books |
| `available` | What repositories offer: CrossWire and other SWORD repositories (sword-modules' catalogs), eBible.org, gracious-tech's collection, the Digital Bible Library (needs a key) |
| `install` | Download (with an explicit user choice to go online, which CrossWire asks for), verify, unpack to staging, build the cache, then register |
| `import` | A local zip or folder (USX, SWORD, OSIS) |
| `update` | Compare versions (sword-modules' `module_status`, eBible.org dates) and reinstall |
| `remove` | Unregister, delete the package and its cache |
| `default`, `alias`, `rename`, `order`, `enable` | Edit the registry |
| `verify` | Rebuild caches whose sources changed, and report broken packages |

Front ends:

- **CLI**: `topos bible list|install|remove|default ...`, or bibliotheca's own binary. The CLI's
  search doesn't need translations, so a separate binary keeps topos small.
- **LSP**: the `topos/bibles` request lists them. Management goes through
  `workspace/executeCommand` (`topos.bible.install`), and the editor confirms downloads.
- **Neovim**: `:ToposBible install KJV` with a picker of what's available, and
  `:ToposBible remove`.
- **Obsidian**: a Bibles section in the settings: installed translations (size, license,
  remove, set default), and *Add a Bible* from a repository or a file. Downloads go through
  Obsidian's `requestUrl` (a `Transport` for sword-modules), so they work on mobile.

## Versification mapping

Verse counts per translation already work (step 4 of translations.md). Mapping between them
(Malachi 4:1 KJV = 3:19 MT, Psalm titles, Romans 16:25-27) belongs in bibliotheca, since it is
data about translations:

- `Versification` names a system (sword-modules' 18) or the translation's own counts
- A mapping table per pair of systems, through a canonical one, so one mapping per system is
  enough. Sources: CrossWire's versification mappings (libsword has some; sword-modules doesn't
  read them yet) or the Copenhagen Alliance versification files
- `Library::map(passage, from, to)` gives the same text's place in another translation, for
  side-by-side views (the LSP's `topos/sameVerse`, the plugin's parallel tabs) and for showing a
  `John 3:16 KJV` reference in a Bible that numbers it differently

## Bindings

The plugin uses one WebAssembly package (topos-ffi). Two packages would ship topos twice. Options:

1. **bibliotheca-ffi includes topos's API**, and becomes the package apps use. topos-ffi stays
   the small, references-only package for those who want just that. This is the recommendation:
   it keeps the dependencies one way (bibliotheca → topos).
2. topos-ffi gets a `library` feature that depends on bibliotheca. That makes topos depend on
   something that depends on topos (across repositories), so it isn't recommended.

`ToposBook` moves to bibliotheca (as its `Book`), and `ToposTranslations` comes from
`Library.toposTranslations()`. In WebAssembly, a source can't read the file system, so sources
take bytes:
- USX already parses from text
- sword-modules needs a storage trait (read a file by path) before SWORD works in the plugin;
  today it opens modules from a folder on disk

## Steps

1. **Create bibliotheca** (its own repository, like sword-modules) with `TranslationInfo`, the
   `Source` trait, and the USX source: the LSP's `bible.rs` reading code, the plugin's file
   naming rules, and `usx_chapter_verses`.
2. **The cache**: manifests and book trees, with keys and the format version. Benchmark the BSB
   cold (parse) against warm (cache).
3. **The LSP uses it**: `bibles` entries become sources, `Bibles::new` builds from the library,
   and the verse-count scan goes away.
4. **The registry and the shared library folder**: `library.toml`, and `library =` in topos's
   config. Neovim's `bibles` can then be names instead of paths.
5. **The SWORD source** through sword-modules, then a storage trait in sword-modules so modules
   can be read from bytes (WASM).
6. **Management**: list, available, install, update, remove, starting with SWORD (sword-modules
   already installs) and eBible.org.
7. **bibliotheca-ffi** with topos's API, and the plugin moves to it: the Bibles section in the
   settings, an IndexedDB cache on mobile, and `VerseCounts` removed.
8. **Versification mapping**.

## Open decisions

- The name, and whether bibliotheca's CLI is part of `topos` (`topos bible ...`) or its own
  binary
- One package for apps (bibliotheca-ffi including topos) or two
- Whether Obsidian desktop also uses the machine's shared library, or only the vault's Bibles
- Which repositories come first after SWORD (eBible.org has the most open translations with USX)
- How strict licenses are: only show them, or block syncing a translation that isn't openly
  licensed
