# τόπος Bible

> [τόπος](https://biblehub.com/greek/5117.htm): spot, location, position

Locate Bible verses in text, files, and entire directories!

## Install

```sh
# The `topos` command-line tool
cargo install topos-bible-cli

# The library, from Rust, JavaScript/TypeScript, or Python
cargo add topos-bible
npm install topos-bible
pip install topos-bible
```

## Neovim

[**logoxide.nvim**](https://github.com/Logoxide/logoxide.nvim) runs the language
server and searches your notes, with Telescope pickers. With a Bible (a folder of USX files),
hovering a reference shows its text, and go to definition opens the whole book, colored the way
you choose. With [lazy.nvim](https://lazy.folke.io):

```lua
{
  'Logoxide/logoxide.nvim',
  main = 'logoxide',
  build = ':LxBuild!', -- needs Rust: https://rustup.rs
  opts = { bible = '~/Bibles/BSB' },
}
```

<details>
<summary>Screenshots</summary>

![Neovim Quick Switcher Integration](https://raw.githubusercontent.com/Logoxide/logoxide.nvim/main/doc/imgs/neovim-quick-switcher.png)

![Neovim Telescope Integration](https://raw.githubusercontent.com/Logoxide/logoxide.nvim/main/doc/imgs/neovim-telescope.png)

</details>

## Crates

| Crate (folder) | What it is |
|---|---|
| [`topos-bible`](./crates/topos-lib/README.md) (`topos-lib`) | The core: book data, the segment grammar, resolving, searching, formatting, OSIS, and autocomplete |
| [`topos-bible-formats`](./crates/topos-formats/README.md) (`topos-formats`) | Locations in HTML, SRT/WebVTT/SBV, EPUB, JSON, XML, and PDF (`pdf` feature) |
| [`topos-bible-cli`](./crates/topos-cli/README.md) (`topos-cli`) | `topos`, a ripgrep-style search tool |
| [`topos-ffi`](./crates/topos-ffi) | Bindings for TypeScript/WASM, Python, Swift, and Kotlin via [BoltFFI](https://boltffi.dev) |

The language server moved to [logoxide](https://github.com/Logoxide/logoxide) as `logoxide-lsp`
(October 2026), with Bible text from logoxide packages; topos stays references only.

The Obsidian plugin (verse search with filters, autocomplete, and links to Bible sites) is in its own
repository, `obsidian-logoxide`, and uses logoxide's npm package, which includes `topos-ffi`'s API.

See [ROADMAP.md](./ROADMAP.md) for what is planned and [doc/architecture.md](./doc/architecture.md) for how it fits together.
