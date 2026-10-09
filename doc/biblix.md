# biblix: caching, sharing, and managing translations

biblix is a library above topos (and its CLI, `bx`) that reads translations from many formats,
packages them so they can be shared and versioned, caches them, and installs, updates, and
removes them. topos stays references only; sword-modules is one source among several.

[translations.md](./translations.md) covers what is done: references that name a translation
(`John 3:16 KJV`), each translation's own verse counts, and several Bibles in the LSP, Neovim, and
Obsidian.

## Decisions (2026-10-08)

- **Name**: `biblix`, with the CLI `bx`, in its own repository (like sword-modules).
- **One package for apps**: biblix-ffi includes topos's API. topos-ffi stays the small,
  references-only package.
- **Obsidian on desktop uses the shared library**, at a location the user can choose, including
  inside `.obsidian/` so it syncs to mobile.
- **Licenses are shown, never enforced**: assume every package can be shared.
- **Packages are git repositories** with a `biblix.toml` and semantic versions, specified the way
  Cargo specifies dependencies (versions, features, `git =` with a tag or revision).
- **One manifest name, `biblix.toml`**, for packages and for vaults, like `Cargo.toml`: a vault's
  is a manifest without `[package]`. "Index" means only a registry.
- **No default registry for now**: dependencies name a `git =` URL or a `path =`. The format leaves
  room for registries later (anyone could host one), but nobody has to mirror upstream texts.
  Catalogs (metadata and URLs, no texts) are searched instead; see Status.
- **A package can hold several formats** (USX and SWORD of the same text), each behind a feature
  the package's owner names.
- **Summaries with every verse's hash are published for every package**, not only Bibles.
- **Audio and video are referenced, never stored**: a package lists each media file's URL, size,
  and SHA-256, with its transcript and timings (see Media). Git keeps only small, diffable files,
  and the media stays wherever its owner serves it.
- **biblix is its own repository**: `~/Development/rust/biblix`, next to sword-modules.
- **Imports are named by their catalog's kind** (`ebible:<id>`, `fetch:<id>`): each kind declares
  its prefix, and `bx import` looks the name up in the project's catalogs, the same ones
  `bx search` reads (decided October 2026).
- **Short names for git hosts** (decided October 2026): `github:owner/repo` (also `gitlab:`,
  `codeberg:`; the path is the host's own, so GitLab's nested groups work) wherever a URL is
  taken. They're shorthand for typing: manifests and lock files keep the full URL.
- **Forks side by side** (decided October 2026): a dependency can name another package
  (`kjv-bob = { package = "kjv", … }`, like Cargo) and give its own `abbreviation` and `aliases`,
  so references name each fork apart. Library folders are `<version>+g<commit>`, so forks at one
  version never share a folder, and lock files name packages the same way.
- **Each work is its own package; libraries are packages of packages** (decided October 2026):
  a collection holds no text, only optional dependencies a feature turns on each
  (`gill = ["dep:gill"]`), so readers take only the works they want, and each work is versioned
  on its own. Rejected: parts of one package chosen by features, which would need partial
  checksums and summaries per part.
- **USFM is read with usfm3.** On the whole KJV, both usfm3 and
  [usfm-onion](https://github.com/WycliffeAssociates/usfm-onion) gave its 31,102 verses, and
  usfm-onion was faster (0.58 s against 0.86 s). But usfm-onion's USX drops the space between
  adjacent `\w` words (`For</char><char …>God`), so it reads "ForGodsoloved". usfm3 keeps the
  spaces, and it's on crates.io.

## Status

Steps 1 to 4 are done in the biblix repository (`bx init`, `add`, `remove`, `install`,
`update`, `list`, `show`, `check`, `release`, `diff`, `verify`). Step 1:
- `biblix.toml`, `Storage`, the USX and USFM sources, packages, the cache, and `bx show`/`bx check`
- Hand-made packages in `~/Development/biblix-packages/`, as git repositories tagged `v1.0.0`:
  `bsb` (USX, from a DBL bundle) and `kjv` (USFM, from fetch.bible)

Reading every book of each:

| | From source | Building the cache | From the cache |
|---|---|---|---|
| BSB (USX, 31,086 verse spans) | 166 ms | 227 ms | 120 ms |
| KJV (USFM, 31,102 verses) | 1 s | 2 s | 323 ms |

The cache helps most with USFM. A warm open also hashes the whole package (about 30 ms); a git
package could be keyed by its commit instead.

Steps 2 and 3:
- **Fetching**: the `git` command for now, behind a `Fetcher` trait.
- **Lock files**: `biblix.lock` pins each package's commit and checksum. Another library gets the
  same text from it, and a changed installed file is caught.
- **Releases**: `bx release` checks the version on real changes:
  - a note changed: patch
  - a word changed: minor
  - two verses' texts swapped: renumbered, so major
- **Summaries**: `bx show --git` reads a release's summary through a partial clone, without its
  text.
- **The hand-made packages** are released again as 1.0.1, with summaries.
- **Verse hashes**: `biblix.verses.bin` is 373 KB for the KJV (12 bytes a verse).
- **Versions keep links working**: a link resolves against the version it was made with, and the
  version number says when a change can break links.

- **The language server moved into biblix** as `biblix-lsp` (`crates/biblix-lsp`, history kept),
  so it can read Bible text from packages without topos depending on biblix. topos keeps the
  library, markup, formats, index, CLI, and topos-ffi.

Step 5, started:
- **biblix-ffi** (`crates/biblix-ffi`, npm and PyPI `biblix`) re-exports topos-ffi, so one package
  has both APIs: BoltFFI exports a local dependency's items with the crate's own.
- **The host does the I/O**: `Biblix.openPackage` takes the package's files as bytes, and
  `cacheWrites()` hands back the cache files to save. `Biblix.parseProject` reads a vault's
  `biblix.toml` and `biblix.lock` and says where each package's files are.
- **Real packages in WebAssembly** (Node, release build): the BSB opens in 150 ms, and the KJV
  (USFM, every book parsed to count verses) in 0.8 s, or 0.09 s from the cache (Python).
- **Installing without git**: `BiblixInstaller` runs the same resolver as `bx install` against
  what the host has downloaded, and asks for what's missing: a repository's refs (git's smart
  HTTP, `info/refs`, which GitHub, GitLab, Gitea, Forgejo, and Bitbucket serve), a commit's
  `.tar.gz`, or an installed package's files. An archive's files hash the same as a checkout's,
  so lock files work both ways: `bx install` accepts a library and lock written in WebAssembly.
  Installing the KJV and BSB in Node takes 0.4 s, checking them again 0.15 s.
- **The core reads and writes through `Storage`** for this: `Fetcher` fetches into any storage,
  and the resolver finds installed packages through a `Packages` trait (the library's folders
  natively).
- **The Obsidian plugin is on biblix's package** (one WebAssembly module for the plugin and its
  worker) and reads the vault's `biblix.toml` and `biblix.lock`: packages add translations with
  their verse counts and give passage text.
- **The plugin's library is a vault folder, `Biblix/` by default** (a setting; decided
  2026-10-08), with the vault's `biblix.toml` and `biblix.lock` in it unless they're at the
  vault's root. Its books are vault files, so the reader, hover previews, and embeds open them
  (files in `.obsidian/` aren't vault files). USFM books open too, read as USX.
- **The plugin's settings list the vault's packages** (each with a remove button) and add more:
  *Search* looks through the catalogs and installs or imports the one chosen, *Git URL* adds a
  package's newest release.

Catalogs and imports (from step 7):
- **Catalogs** (`catalog.rs`): lists of packages to search by name, abbreviation, or language: a
  `catalog.toml` of git packages (metadata and URLs only, so anyone can host one in a git
  repository; `bx catalog entry <url>` writes an entry), or eBible.org's `translations.csv`
  (1,500 translations; the downloadable, redistributable ones are listed). **eBible.org's and
  fetch.bible's own lists are searched by default** (their listings of their downloads, not
  mirrors; the owner confirmed this, October 2026), **and users add their own catalogs**:
  `[settings] catalogs` in a vault's `biblix.toml` are searched first (their entries win when ids
  clash), `default-catalogs = false` turns the defaults off. `bx catalog list|add|remove` and
  the plugin's *Catalogs* settings edit them; adding reads the catalog first to check it. A
  hosted `catalog.toml` is how anyone runs a registry of their own for now.
  **Registries** are named catalogs (`[settings.registries]`, `bx catalog add <location> --name
  dorean-local`): a folder of package repositories (each git repository with a `biblix.toml`,
  at any depth) or a catalog. `dorean-local:dorean-collection` names a package by its id,
  `dorean-local:books/romans` by its path in the folder (the owner asked for both, October
  2026); manifests keep the full URL.
  fetch.bible's `manifest.json` lists 1,400 translations as USX, with SPDX-like licenses.
- **Importing from fetch.bible**: each book's USX (66 downloads; 12 s one at a time in `bx`,
  1 s eight at a time in Obsidian) into `usx/`, with its attribution in `COPYRIGHT.txt`. Its BSB
  has the hand-made package's 31,086 verses.
- **Importing from eBible.org** (`import.rs`): its `<id>_usfm.zip` becomes a package
  (`biblix.toml`, `usfm/`, `copr.htm` as the license file, versioned by eBible.org's update date,
  `2026.10.8`), used as a path package in `imports/<id>/`. The ASV imports in 0.3 s (`bx`) or 2 s
  (Obsidian), and reads all 31,102 verses.

## Layers

```
topos-bible          references: parsing, search, Translations (names, aliases, counts)
topos-bible-markup   Bible text as one tree (USX, OSIS, ThML, GBF, TEI), Flat, HTML
sword-modules        SWORD modules: read, install from SWORD repositories
biblix               packages (biblix.toml), sources (formats), the library (installed
                     versions), the cache, the lock file; builds topos's Translations
bx                   the CLI
biblix-ffi           biblix's and topos's API for apps (Python, WASM, Swift, Kotlin)
front ends           topos CLI, LSP, Neovim, Obsidian
```

## Packages

A package is a git repository: a `biblix.toml` and the text in one or more of the formats biblix
reads. A tag `v1.2.0` is a release.

```toml
[package]
id = "bsb"                       # what dependencies call it
name = "Berean Standard Bible"
abbreviation = "BSB"             # what references name it by (John 3:16 BSB)
aliases = ["Berean"]
version = "1.2.0"                # see Versioning
kind = "bible"                   # bible | commentary | lexicon | cross-references | versification-map | book
language = "en"                  # BCP 47
direction = "ltr"
license = "CC0-1.0"              # SPDX; or license-file = "LICENSE.md" (shown, never enforced)
publisher = "Berean Bible"
homepage = "https://berean.bible"
upstream = "https://ebible.org/details.php?id=engbsb"  # where it was imported from, if anywhere
versification = "own"            # or a named system: "KJV", "NRSV", "MT", ...

# Each format of the text, behind a feature
[formats]
usx = { path = "usx/" }
sword = { path = "sword/", feature = "sword" }

[features]
default = ["usx", "headings"]
usx = []
sword = []                       # the same text as a SWORD module, for SWORD apps
headings = []                    # elements that can be left out at render time
notes = []
strongs = ["dep:strongs-greek"]  # optional dependencies, like Cargo

[dependencies]
strongs-greek = { git = "https://github.com/someone/strongs-greek", version = "2", optional = true }
tsk = { git = "https://github.com/someone/tsk-cross-references", tag = "v1.0.0" }
```

- **Formats** are the same text in different forms; each has a path and the feature that turns it
  on. biblix reads whichever enabled format it supports best (USX before SWORD, say), and a
  package can carry a format only other apps use.
- **Features** turn on formats, parts of the text (footnotes, Strong's numbers), and optional
  dependencies, like Cargo's `dep:`. Installing with fewer features can fetch less: a native `bx`
  fetches only the enabled formats' paths (a partial clone with sparse checkout); an archive
  download (WebAssembly) gets everything and keeps what's enabled.
- **Dependencies** are other packages: lexicons for Strong's numbers, a cross-reference set, a
  versification map, or the base text of an interlinear. They're `git =` (with `version`, `tag`,
  `rev`, or `branch`) or `path =`. A `version` requirement picks the newest matching tag.
- **Importing**: `bx import` turns a download in another format (an eBible.org zip, a SWORD
  module, a Door43 repository, a CCEL ThML file) into a package (a `biblix.toml` around the
  files), which can then be pushed anywhere as a git repository.
- **Media** (audio, video) is listed, not stored: each file's URL, size, and SHA-256, plus its
  transcript (what topos searches) and its timings (each verse's start and end). biblix fetches a
  file when it's first played, checks its hash, and keeps it by hash, so versions share it.
  Releases are judged by the transcript and timings; a new hash for the same words is a patch.
  A recording follows one text, so it names that text's package as a dependency.

  ```toml
  [media]
  timings = "timings/"           # each verse's start and end, per file: timings/JHN.vtt

  [[media.files]]
  path = "JHN.mp3"               # what timings name it by
  url = "https://example.org/audio/JHN.mp3"
  size = 31457280
  sha256 = "…"
  ```
- **Registries (later)**: a registry would be a git repository of entries (each version's git URL,
  tag, commit, and checksum), so `bx add bsb` works without a URL. biblix can add `registry =`
  without changing packages, so it waits until someone wants to host one.

## Summaries

A package version ships a small summary of what it holds, so its contents can be known without
downloading or reading it. `bx release` writes it, it's committed with the tag, and anyone can
check it (`bx verify` builds it again from the text and compares):

```toml
# biblix.summary.toml, beside biblix.toml at each tag
version = "1.2.0"
checksum = "sha256:…"            # of the text, for verifying downloads
size = { usx = 4_812_331, sword = 2_106_880 }   # bytes per format, so a phone can choose

[books]
GEN = [31, 25, 24, …]            # each book's verse counts: its versification
EXO = [22, 25, 22, …]
# …

[contents]
notes = 9_412                    # how many of each element: footnotes, headings, words of Jesus,
headings = 1_870                 # Strong's numbers, ...
strongs = 0
```

Plus, beside it, `biblix.verses.bin`: a short hash of each verse's text (373 KB for the KJV), published for every package of any kind (commentaries are keyed by verse too).

What this gives without installing:
- **Browsing**: which books (the deuterocanon or not), which features and elements, and how big
  each format is
- **Checking references** against a translation that isn't installed: its verse counts become
  topos `Translations` (John 3:18 XYZ is checked before XYZ is downloaded)
- **Versions**: two tags' verse hashes give `bx diff` (renumbered verses from the counts,
  changed wording from the hashes) without fetching either text, and tell a link whether its
  verse changed since its version
- **Fetching it alone**: git hosts serve one file at a tag
  (`raw.githubusercontent.com/<owner>/<repo>/<tag>/biblix.summary.toml`, and the same on GitLab
  and Gitea), and a native `bx` can fetch just those files with a sparse checkout

The parsed cache itself isn't distributed:
- it's larger than the text (trees and spans)
- it depends on the cache format's version, so every new biblix would need it published again
- it rebuilds from the text in well under a second per Bible

A plain-text search index could be published later, the same way as the verse hashes, if
searching uninstalled translations turns out to matter.

## Projects and the lock file

A vault or a notes folder declares the Bibles it uses, like `Cargo.toml`, and pins them, like
`Cargo.lock`:

```toml
# biblix.toml in a vault (.obsidian/biblix.toml, or the vault's root): a manifest without [package]
[dependencies]
bsb = { git = "https://github.com/someone/bsb", version = "1" }
kjv = { git = "https://github.com/someone/kjv", version = "2", features = ["strongs"] }
mine = { path = "Bibles/My Translation" }

[settings]
default = "bsb"
compare = ["kjv"]
```

- `biblix.lock` records each package's exact version, commit, and checksum. It syncs with the
  vault, so every device installs the same text, and links mean the same thing everywhere.
- `bx update` moves to newer versions within the requirement (`1` allows `1.x`). A new major
  version is only taken when the requirement changes (see Versioning).

## Versioning

A package's version says what an update can do to links. `bx` checks this at release time
instead of trusting it:

| Change | Version | What it can break |
|---|---|---|
| Verses renumbered, merged, split, or removed; books removed; versification changed | **major** | Verse links (`v=3:16`) may point at different text |
| A verse's wording changed | **minor** | Exact-word links (text fragments) in that verse may not match |
| Markup, headings, notes, metadata | **patch** | Nothing |
| Books or verses added | **minor** | Nothing (new verses only) |

- `bx diff 1.2.0 1.3.0` classifies each change by comparing verse counts (major) and each verse's
  text (minor). `bx release` refuses a version number smaller than the change, and the
  changelog lists the verses whose wording changed.
- **Links pin what they need.** A verse link needs the major version, and an exact-word link the
  exact version. The lock file says which version a vault uses. A link made against another
  version can carry it (`JHN.usx#v=3:16&ver=1.2.0`). The library keeps every version a lock file
  or a link asks for, side by side (`packages/bsb/1.2.0/`).
- **Later: links that follow changes.** For an exact-word link made against an older version:
  1. Read that version's verse (a git tag, so old text is always available).
  2. Find the fragment's words in it.
  3. Map that place into the new version's verse.
  4. Build the shortest text fragment that matches once there.
  5. Rewrite the link in its original format (wiki link, Markdown link, `v=` locator).

  Most changes resolve on their own. When a fragment can't be made unique, the link is reported
  instead of guessed.

## The library

```
<library>/                       BIBLIX_HOME, else $XDG_DATA_HOME/biblix; any folder the user picks
  biblix.toml                    the machine's own requirements and settings (default, aliases)
  biblix.lock
  packages/<id>/<version>/       each installed version as plain files (no .git), so it syncs well
<cache>/                         per device, never synced: $XDG_CACHE_HOME/biblix, IndexedDB on mobile
  git/                           bare clones and fetched archives
  books/<id>/<version>/          parsed book trees and the manifest (verse counts, offsets)
```

- **The library's location can be chosen.** For Obsidian, put it inside the vault's `.obsidian/`
  so it syncs to mobile. Packages are plain files there; git objects and caches stay outside, on
  each device.
- The topos CLI and LSP find it through `library =` in `~/.config/topos/config.toml` (or
  `BIBLIX_HOME`), and name Bibles instead of paths: `bibles = ["bsb", "kjv"]`.
- The LSP's `bible.rs`, the plugin's `core/bibles.ts` and `VerseCounts`, and `ToposBook` move into
  biblix as the USX source and the cache.

## The cache

Parsing a USX book takes milliseconds (the whole BSB about 100 ms), SWORD decompresses blocks, and
the LSP renders whole books. The cache keeps, per package version:
- a manifest: verse counts, each book's offset, the package's checksum, and the cache format's
  version
- each book's parsed tree and verse spans (postcard)

It is keyed by the package's checksum and the cache format, so a release (or a new biblix)
rebuilds it. `Library::topos_translations()` builds topos's `Translations` from the manifests
without opening a book.

## Formats and repositories

What exists, from a survey on 2026-10-08:

| Source | What it offers | Format | Licenses | Fit |
|---|---|---|---|---|
| [CCEL](https://ccel.org) | Classics, commentaries, and some Bibles | [ThML](https://ccel.org/ThML) (its source format; downloads need a login) | Mostly public domain | topos-bible-markup reads ThML: commentary and book packages |
| CrossWire and other SWORD repositories (eBible.org, IBT, STEP, Lockman, ...) | Hundreds of Bibles, commentaries, lexicons | SWORD modules | Mixed; some locked with keys | sword-modules already installs them |
| [eBible.org](https://ebible.org) | About 1,000+ translations, a `translations.csv` index | USFM zips, USFX, a SWORD repository, EPUB, HTML | Mostly public domain or Creative Commons | Large and open: first after SWORD |
| [fetch.bible](https://fetch.bible) (gracious-tech) | About 1,400 Bibles in 1,100+ languages, a manifest on a CDN | USX or USFM (raw), HTML or text (normalized) | Open licenses only; 66 books | Static files with a manifest: easy to import |
| [Door43](https://git.door43.org) (unfoldingWord) | Translations as git repositories | USFM + `manifest.yaml` ([Resource Container](https://resource-container.readthedocs.io/)) | Open (CC BY-SA mostly) | Already git: import a repository directly |
| Digital Bible Library | Publishers' releases | USX + `metadata.xml` ([Text Release Bundle](https://app.thedigitalbiblelibrary.org/static/docs/entryref/text/index.html)) | Licensed; an account is needed | Import bundles the user has |
| [Scripture Burrito](https://docs.page/unfoldingWord/uW-Tools-Collab/4-repository-formats) | The newer packaging standard (DBL, Paratext) | `metadata.json` + ingredients | — | Map its metadata to `biblix.toml`; import burritos |
| MyBible, MySword, e-Sword, theWord ([formats](https://mysword.info/modules-format), [MyBible in AndBible](https://github.com/AndBible/and-bible/wiki/MyBible-module-support)) | Large community module sites | SQLite (MyBible, MySword, newer e-Sword), RTF/HTML, plain text (theWord) | Mixed, often unclear | Importers later |
| Zefania XML, OSIS XML (open-bibles), CSV/SQL sets | Public domain texts | XML, CSV, SQL | Public domain | Small importers |
| API.Bible, Bible Brain | Many licensed translations | JSON APIs with keys | Licensed per translation | Not packages; maybe an online source later |

Formats to read:
- **USX** (done)
- **USFM** (done): [`usfm3`](https://docs.rs/crate/usfm3/0.2.0) converts it to USX (see
  Decisions for the comparison with usfm-onion)
- **USJ** (USX as JSON)
- **OSIS** and **ThML** (done: Bibles, a book a file or the whole Bible in one; on the KJV from
  open-bibles' OSIS and CCEL's ThML, 31,102 verses in the 66 books, the same per chapter as USFM.
  ThML's empty `<scripture parsed=…/>` elements are its verse markers)
- **SWORD** (sword-modules)
- Later: Zefania XML, then the SQLite app formats

## FFI and WebAssembly

biblix-ffi (BoltFFI, like topos-ffi) has biblix's and topos's API in one package:
- **Native** (Python, Swift, Kotlin, desktop): has a file system, so SWORD works through
  sword-modules as it is.
- **WebAssembly** (Obsidian): has no file system and no git, so biblix needs:
  - **Storage**: a trait (read a file, list a folder) that sources and sword-modules read
    through, with a std::fs implementation and a host implementation (the plugin's vault adapter,
    passed in from JavaScript). sword-modules needs this trait, since today it opens a folder on
    disk.
  - **Fetching without git** (done): a repository's refs and a commit's archive are downloaded
    through the host's HTTP (`requestUrl` in Obsidian, which works on mobile), and biblix
    resolves and unpacks them. Native `bx` runs the `git` command (gitoxide could replace it).
    Either way the result is the same files, checked against the lock file's checksum.

## Managing

`bx` (and the same operations through biblix-ffi, the LSP's commands, and the plugin's settings):

| Command | What it does |
|---|---|
| `bx add --git URL [--version 1 \| --tag v1.0.0] [--features …]`, `bx add --path DIR` | Add to `biblix.toml`, install, and lock |
| `bx show --git URL [--tag v1.2.0]` | A package's summary, without installing it |
| `bx install` | Install what the lock file says (a new device) |
| `bx update [id]` | Newer versions within the requirements; shows major versions available |
| `bx remove id` | Drop it from `biblix.toml` (versions links still need stay until `bx clean`) |
| `bx import <file or URL>` | Make a package from another format |
| `bx diff id 1.2.0 1.3.0` | What changed, and the version it needs |
| `bx release` | Tag a package's new version, after checking it against the last one |
| `bx list`, `bx info id` | Installed packages: version, license, size, books (done) |
| `bx clean`, `bx verify` | Remove versions nothing uses (done: lists them, `--remove` deletes); check a summary |
| `bx search`, `bx import` | Find packages in catalogs; make eBible.org's or fetch.bible's download into one (done) |
| `bx media add`, `bx media check` | List a recording by URL and hash; download and check every one (done) |

- Neovim (done): `:ToposBibleSearch <words>` (choose one to install), `:ToposBibleAdd <git URL |
  ebible:id | fetch:id>`, `:ToposBibleRemove <id>`, running `bx` (`bx search --json`) in the current
  folder, then restarting biblix-lsp.
- Obsidian: a Bibles section in the settings: the library's location, installed Bibles (version,
  license, remove, default), and *Add a Bible* from a git URL (showing its summary first) or a
  file.

## Steps

1. **Create biblix** (done) with `biblix.toml`, the USX source (moved from the LSP and the plugin),
   `Storage`, and the cache. Make packages of the BSB and a KJV by hand.
2. **`bx` with git and path dependencies** (done): `add`, `install`, the lock file, side-by-side
   versions, and features that choose formats. Fetching runs the `git` command behind `Fetcher`;
   archive downloads come with biblix-ffi (step 5).
3. **Summaries** (done): `bx release` writes them, `bx verify` checks them, `bx show` reads them from a
   tag alone.
4. **The LSP uses the library** (done, as `biblix-lsp` in the biblix repository):
   - `bibles = ["bsb"]` names packages from the project's lock.
   - With no Bible settings, it uses the workspace's `biblix.toml`, else the library's own.
   - `bible`/`bibles` folders can be packages (USX or USFM) or plain USX folders.
   - The Neovim plugin runs it and builds it from a biblix checkout.
5. **biblix-ffi with topos's API** (done): the plugin moves to it, and it gets the library location
   setting (a vault folder, `Biblix/` by default) and the Bibles section.
6. **Versioning tools**: `bx diff` (from summaries) and `ver=` in links.
7. **Importers**: eBible.org and fetch.bible (done, with their catalogs), Door43 (git), CCEL
   (ThML), SWORD through sword-modules with `Storage`.
8. **Media**: `[media]` in `biblix.toml` (done: each file's path, URL, size, SHA-256, and book;
   `bx media add` and `bx media check`; hosts check downloads with `verify` and keep them by hash),
   the transcript read as a text source (it's the package's `[formats]`, so done), and timings
   (done; **the owner chose WebVTT**, October 2026, for audio Bibles): one file per media file
   (`timings/JHN.vtt`), each cue named by the verse it reads (a USFM reference, `JHN 3:16`, or
   `JHN 3:17-18` for a bridge), its time range the verse's, its text the verse's or empty; cues
   not named by a verse (introductions, music) are skipped. Any subtitle player shows them, and
   Transcript++ can. `bx check`, `bx media check --offline`, and `bx release` check them against
   the text (verses it lacks, another book than the file's, out of order, overlapping, a file
   without timings); apps get them from `BiblixPackage.timings(file)`. A timing change is a
   patch. Still to do: a player in the plugin (play from a verse, follow along).
9. **Links that follow changes** (text fragments re-resolved across versions).
10. **Collections** (done): `kind = "collection"`, a package with no `[formats]`, only
   dependencies (optional ones a feature turns on); a project's Bibles include what its
   collections bring in. Books (`kind = "book"`, an `epub` format) are packages too, kept as
   files: `bx new --from book.epub` fills in `[package]` from the EPUB (title, subtitle, every
   author: `authors` is a list), and a release compares their files (any change is minor). A
   book's release publishes the passages topos finds it citing, with counts
   (`biblix.references.bin`, a few KB), so what it cites is known without downloading it;
   where each reference is comes from the book itself (a tenth of a second).
   **A package is what its manifest declares** (decided October 2026): its formats' files by
   extension, the license file, summary, timings, and `include` globs. Files a reader adds
   beside them (File++'s annotations) don't change its checksum, `bx clean` never removes them,
   and `bx install --repair` puts back edited files and keeps them. The
   bindings make packages (`newPackage`, `newCollection`, `fileMetadata`) and find references
   (`BiblixPackage.references()`) in WebAssembly too.
11. **Versification mapping** packages (`kind = "versification-map"`).
12. **Registries**, if someone wants to host one.

## Open questions

- **What a summary counts**: the list of elements in `[contents]`.
