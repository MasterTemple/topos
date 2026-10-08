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
- **Packages are git repositories** with an `index.toml` and semantic versions, specified the way
  Cargo specifies dependencies (versions, features, `git =` with a tag or revision, other
  registries).
- **Versions keep links working**: a link resolves against the version it was made with, and the
  version number says when a change can break links.

## Layers

```
topos-bible          references: parsing, search, Translations (names, aliases, counts)
topos-bible-markup   Bible text as one tree (USX, OSIS, ThML, GBF, TEI), Flat, HTML
sword-modules        SWORD modules: read, install from SWORD repositories
biblix               packages (index.toml), sources (formats), the library (installed
                     versions), the cache, the lock file; builds topos's Translations
bx                   the CLI
biblix-ffi           biblix's and topos's API for apps (Python, WASM, Swift, Kotlin)
front ends           topos CLI, LSP, Neovim, Obsidian
```

## Packages

A package is a git repository: a manifest, `index.toml`, and the text in one of the formats biblix
reads. A tag `v1.2.0` is a release.

```toml
[package]
id = "bsb"                       # unique in its registry
name = "Berean Standard Bible"
abbreviation = "BSB"             # what references name it by (John 3:16 BSB)
aliases = ["Berean"]
version = "1.2.0"                # see Versioning
kind = "bible"                   # bible | commentary | lexicon | cross-references | versification-map
language = "en"                  # BCP 47
direction = "ltr"
license = "CC0-1.0"              # SPDX; or license-file = "LICENSE.md"
publisher = "Berean Bible"
homepage = "https://berean.bible"
upstream = "https://ebible.org/details.php?id=engbsb"  # where it was imported from, if anywhere

[content]
format = "usx"                   # usx | usfm | usj | osis | sword | zefania | ...
path = "usx/"                    # a folder of books, or one file
versification = "own"            # or a named system: "KJV", "NRSV", "MT", ...

[features]
default = ["headings"]
headings = []                    # elements that can be left out at install or render time
notes = []
strongs = ["dep:strongs-greek", "dep:strongs-hebrew"]  # optional dependencies, like Cargo

[dependencies]
strongs-greek = { version = "^2", optional = true }
tsk = { git = "https://github.com/someone/tsk-cross-references", tag = "v1.0.0" }
kjv-map = { version = "1", registry = "crosswire-mirror" }
```

- **Features** turn on parts of a package (footnotes, Strong's numbers, cross references) and
  their optional dependencies, like Cargo's `dep:`.
- **Dependencies** are other packages: lexicons for Strong's numbers, a cross-reference set, a
  versification map, or the base text of an interlinear.
- **Where a package comes from**, like Cargo:
  - a registry (the default, or one named in the config)
  - `git =` with `tag`, `rev`, or `branch`
  - `path =` (a local folder, for writing a package)
- **A registry** is itself a git repository of entries (`b/bs/bsb.toml`, like crates.io's index):
  each version with its git URL, tag, commit, and checksum. Anyone can host one.
- **Importing**: `bx import` turns a download in another format (an eBible.org zip, a SWORD
  module, a Door43 repository) into a package (an `index.toml` around the files), so every other
  format becomes a biblix package.

## Projects and the lock file

A vault or a notes folder declares the Bibles it uses, like `Cargo.toml`, and pins them, like
`Cargo.lock`:

```toml
# biblix.toml (in a vault: .obsidian/biblix.toml, or at the vault's root)
[bibles]
bsb = "1"
kjv = { version = "2", features = ["strongs"] }
lxx = { git = "https://github.com/someone/lxx", tag = "v0.3.0" }

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
| CrossWire and other SWORD repositories (eBible.org, IBT, STEP, Lockman, ...) | Hundreds of Bibles, commentaries, lexicons | SWORD modules | Mixed; some locked with keys | sword-modules already installs them |
| [eBible.org](https://ebible.org) | About 1,000+ translations, a `translations.csv` index | USFM zips, USFX, a SWORD repository, EPUB, HTML | Mostly public domain or Creative Commons | Large and open: first after SWORD |
| [fetch.bible](https://fetch.bible) (gracious-tech) | About 1,400 Bibles in 1,100+ languages, a manifest on a CDN | USX or USFM (raw), HTML or text (normalized) | Open licenses only; 66 books | Static files with a manifest: easy to import |
| [Door43](https://git.door43.org) (unfoldingWord) | Translations as git repositories | USFM + `manifest.yaml` ([Resource Container](https://resource-container.readthedocs.io/)) | Open (CC BY-SA mostly) | Already git: import a repository directly |
| Digital Bible Library | Publishers' releases | USX + `metadata.xml` ([Text Release Bundle](https://app.thedigitalbiblelibrary.org/static/docs/entryref/text/index.html)) | Licensed; an account is needed | Import bundles the user has |
| [Scripture Burrito](https://docs.page/unfoldingWord/uW-Tools-Collab/4-repository-formats) | The newer packaging standard (DBL, Paratext) | `metadata.json` + ingredients | — | Map its metadata to `index.toml`; import burritos |
| MyBible, MySword, e-Sword, theWord ([formats](https://mysword.info/modules-format), [MyBible in AndBible](https://github.com/AndBible/and-bible/wiki/MyBible-module-support)) | Large community module sites | SQLite (MyBible, MySword, newer e-Sword), RTF/HTML, plain text (theWord) | Mixed, often unclear | Importers later |
| Zefania XML, OSIS XML (open-bibles), CSV/SQL sets | Public domain texts | XML, CSV, SQL | Public domain | Small importers |
| API.Bible, Bible Brain | Many licensed translations | JSON APIs with keys | Licensed per translation | Not packages; maybe an online source later |

Formats to read:
- **USX** (done)
- **USFM**: the Rust crate [`usfm3`](https://docs.rs/crate/usfm3/0.1.8) parses USFM 3 into USX or USJ, so USFM needs no parser of our own
- **USJ** (USX as JSON)
- **OSIS** (done in topos-bible-markup)
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
  - **Fetching without git**: a release is downloaded as an archive (GitHub, GitLab, and Gitea
    serve one per tag) through the host's HTTP (`requestUrl` in Obsidian, which works on mobile).
    Native `bx` clones with gitoxide. Either way the result is the same files, checked against
    the registry's checksum.

## Managing

`bx` (and the same operations through biblix-ffi, the LSP's commands, and the plugin's settings):

| Command | What it does |
|---|---|
| `bx search <words>` | Packages in the registries |
| `bx add bsb`, `bx add --git URL --tag v1.0.0` | Add to `biblix.toml`, install, and lock |
| `bx install` | Install what the lock file says (a new device) |
| `bx update [id]` | Newer versions within the requirements; shows major versions available |
| `bx remove id` | Drop it from `biblix.toml` (versions links still need stay until `bx clean`) |
| `bx import <file or URL>` | Make a package from another format |
| `bx diff id 1.2.0 1.3.0` | What changed, and the version it needs |
| `bx release` | Tag a package's new version, after checking it against the last one |
| `bx list`, `bx info id` | Installed packages: version, license, size, books |
| `bx clean`, `bx verify` | Remove versions nothing uses; rebuild broken caches |

- Neovim: `:ToposBible add bsb`, with a picker over `bx search`.
- Obsidian: a Bibles section in the settings: the library's location, installed Bibles (version,
  license, remove, default), and *Add a Bible* from a registry, a git URL, or a file.

## Steps

1. **Create biblix** with `index.toml`, the USX source (moved from the LSP and the plugin),
   `Storage`, and the cache. Make packages of the BSB and a KJV by hand.
2. **`bx` with git sources**: `add --git`, `install`, the lock file, side-by-side versions, and
   archive downloads.
3. **The LSP and topos's config use the library** (`library =`, `bibles = ["bsb"]`).
4. **biblix-ffi with topos's API**: the plugin moves to it, and it gets the library location
   setting (inside `.obsidian/` to sync) and the Bibles section.
5. **Versioning tools**: `bx diff` and `bx release`, and `ver=` in links.
6. **Importers and a registry**: eBible.org and fetch.bible (USX/USFM), Door43 (git), SWORD
   through sword-modules with `Storage`. Then a default registry with the open translations.
7. **Links that follow changes** (text fragments re-resolved across versions).
8. **Versification mapping** packages (`kind = "versification-map"`).

## Open questions

- **Where the default registry lives**, and who publishes the packages imported from eBible.org
  and fetch.bible.
- **Whether a package can hold several formats** (USX and SWORD of the same text) or one per
  package.
- **The manifest's name**: `index.toml` (as proposed) or `biblix.toml`. A project's file is
  `biblix.toml` above, so the two names should differ.
