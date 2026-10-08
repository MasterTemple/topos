# τόπος Bible language server

`topos-lsp` speaks the Language Server Protocol over stdin and stdout:

- **Completion**: book names (`1 Co` → `1 Corinthians`), then chapters and verses from the
  book's versification; each completion rewrites the whole reference (`jn 3:1` → `John 3:16`).
  With a `bible`, chapters and verses resolve (`completionItem/resolve`) to their text
- **Hover**: the reference under the cursor, normalized, with what the `hover` setting lists:
  the passage's text (with a `bible`), its full name and abbreviation, OSIS id, book (and its
  number and chapters), testament, genres, verses, and where it is. On a reference that doesn't
  exist, what does (`John 3 has 36 verses`, `John has 21 chapters`)
- **Go to definition** and **declaration** (with a `bible`): the passage in its whole book, a read-only document
  like `topos-bible://BSB/JHN` (see [Whole books](#whole-books))
- **Inlay hints**: after each reference, how it's written in your format (where that differs,
  like `jn 3:16` → `John 3:16`), or its OSIS id
- **Document symbols**: every reference in the file, for outlines and pickers
- **Go to references**: every reference in the workspace that is exactly the one under the
  cursor (`--exact-overlap`), however it's written. In a whole book, every reference that names
  the verse under the cursor (`--explicit-overlap`), or on a chapter's label, every reference
  that shares a verse with the chapter
- **Code actions**: on a reference, reformat it the way completion writes it (`Reformat as
  "John 3:16"`, and `Reformat all N references in this file` when several differ), or search the
  workspace for it: `Search "John 3:16" for explicit overlap`, `... for any overlap`,
  `... for exact overlap`, and `Search inside "John 3"`. With a `bible`, one action per
  `insertions` entry (`Insert Callout`) writes the passage's text below the reference
- **Diagnostics**, pushed, or pulled when the editor asks for them (`textDocument/diagnostic`, for
  editors that support it): an information diagnostic on each reference with how it's written in your
  format (`John 3:16`, code `reference`), and warnings for references that do not exist, like
  `John 3:99` (code `missing`)

Workspace searches cover every text file in the workspace folders, skipping hidden, binary, and
ignored files (`.gitignore`, `.ignore`, `.toposignore`), like the CLI. Open files are searched
as they are in the editor, including unsaved changes.

Install with:

```sh
cargo install --git https://github.com/MasterTemple/topos-bible topos-lsp
```

## Settings

The server reads the CLI's `~/.config/topos/config.toml` (or `$XDG_CONFIG_HOME/topos/config.toml`),
so one file configures both. It uses these keys and ignores the others:

| Key | Effect |
|---|---|
| `format`, `psg-fmt`, `fmt-*` | How references are written in completions, hover, hints, symbols, and action titles (see the CLI's "Writing references") |
| `data`, `merge-data`, `remove-data` | Custom book data |
| `context-book`, `context-heading` | References without a book (`3:16`) |
| `ext` | Extensions searched in the workspace, like `"md,txt"` (all text files by default) |
| `inlay-hints` | `"changed"` (default), `"always"`, `"osis"`, or `"never"` |
| `reference-diagnostics` | The diagnostic on each reference: `"info"` (default), `"hint"`, or `"never"` |
| `hover` | What a hover shows below the reference, in order (a list or `"osis,book"`; `[]` for none): `text` (the passage, from `bible`), `written`, `name`, `abbreviation`, `osis`, `book`, `testament`, `genres`, `verses`, `bcv`, `location`. The default is every one but `written` and `bcv`. `name` and `abbreviation` are left out when the reference is already written that way |
| `bible` | A folder of USX files, one book each (`JHN.usx`, `44JHNBSB.usx`, or any name with a `<book code>` inside), as Paratext, the Digital Bible Library, and eBible.org publish them. Its USX files are left out of workspace searches |
| `markup` | How Bible text is written (see below) |
| `insertions` | Code actions that insert a passage's text (see below) |

The CLI skips `inlay-hints`, `reference-diagnostics`, `hover`, `bible`, `markup`, and
`insertions`, so they can go in the shared `config.toml`.

`markup` has a table for each kind of element, with `show` and `display` (the defaults are
shown; other keys, like an editor's `color`, are ignored):

```toml
[markup]
verse_numbers = { show = true, display = "[{verse}]" }
chapter_numbers = { show = true, display = "{name} {chapter}" } # in whole books, on their own line
headings = { show = true, display = "{text}" }
notes = { show = false, display = "[{text}]" } # footnotes
words_of_jesus = { show = true, display = "{text}" }
added = { show = true, display = "{text}" }        # translators' additions (KJV italics)
divine_name = { show = true, display = "{text}" }  # LORD in small capitals
quotes = { show = true, display = "{text}" }       # quotations other than words of Jesus
```

Labels (verse and chapter numbers) can use `{verse}` (`16`, or `4-5` for a bridge),
`{chapter}`, `{name}` (`John`), and `{book}` (the USX code, `JHN`). The other elements' `display`
is a template with `{text}` (and `{name}`), like `## {text}` or `“{text}”`, glued to the element's
words. Headings and notes that aren't shown are left out; other elements that aren't shown are
plain text, without their template or color.

Each insertion has a `name` (the action is `Insert {name}`), a `display` (default `{text}`; also
`{reference}` as formatted and `{osis}`), a `line_prefix` written before each line of `{text}`,
and its own `markup` over the one above:

```toml
[[insertions]]
name = "Callout"
display = "> [!bible] {reference}\n{text}"
line_prefix = "> "
markup = { headings = { show = false } }
```

The editor's settings override the file: as initialization options, or as `settings` (changed
while the server runs, without a restart). They take the same keys, at the top level or under
`topos`. Two more choose the file: `config` (another path) and `no-config` (none). A `format`
can also be a whole object, like `psg-fmt`. Settings that can't be used are reported as a
warning, and the previous ones stay.

## Whole books

With a `bible`, go to definition returns a document named like `topos-bible://BSB/JHN` (the
folder's name, then the book's USX code), which no file backs. The editor asks for its text with
the `topos/book` request (`{ "uri": "topos-bible://BSB/JHN" }` gives `{ "title": "John", "text":
"..." }`), shows it read-only, and opens it with the server like any document. The server then
provides:

- **Semantic tokens**: `verseNumber`, `chapterNumber`, `wordsOfJesus`, `heading`, `note`,
  `added`, `divineName`, and `quote` (other documents have none), for the editor to color
- **Go to references** from a verse, as above; **hover** on a verse says how many references
  name it and in how many files (`???` if searching takes more than half a second; files read
  until then are cached)
- Hover and go to definition on the references in its headings; no diagnostics, inlay hints, or
  reformat actions

## Passage text in hover

A passage's text in hover and completion documentation is a Markdown code block whose info
string, after the language, says what to color: `line:column:length:type` for each token, with
byte columns in the block's text and the types numbered as in the semantic tokens legend:

````markdown
```topos-bible 0:0:19:3 1:0:4:0 1:33:4:0
Jesus and Nicodemus
[16] For God so loved the world. [17] ...
```
````

Editors show only the language; the Neovim plugin colors the block from it.

## Neovim

The [topos-bible.nvim](https://github.com/MasterTemple/topos-bible.nvim) plugin sets up the
server, builds it, opens whole books as read-only buffers colored by `markup`, opens the code
actions' results in Telescope (or the quickfix list), and adds search commands and pickers.

Without the plugin (Neovim 0.11+):

```lua
vim.lsp.config('topos', {
  cmd = { 'topos-lsp' },
  filetypes = { 'markdown', 'text' },
  root_markers = { '.git', '.obsidian' },
  init_options = { ['psg-fmt'] = { join_adjacent = true }, ['inlay-hints'] = 'always' },
})
vim.lsp.enable('topos')
```

Go to references and completion then work. The code actions need a `vim.lsp.commands['topos.search']`
handler to show their results (see below; the plugin's is in its `lua/topos/init.lua`), and
go to definition needs the editor to open `topos-bible://` documents (the plugin's
`lua/topos/book.lua`).

## Code actions in other editors

Reformat actions in an unnamed buffer (whose URI, `file://`, names no file) carry their edits as the
`topos.applyEdits` command (`{ "edits": [TextEdit] }`) for the editor to apply to that buffer;
in named files they are ordinary workspace edits. Completion lists are marked incomplete, so
editors ask again as you type instead of filtering a stale list.

LSP has no standard way for a server to open the editor's references list. The code actions
run the `topos.search` command, which returns the matching locations
(`workspace/executeCommand` with `{ "mode": "explicit-overlap" | "any-overlap" |
"exact-overlap" | "inside", "passage": "John 3:16" }`); the editor shows them.
