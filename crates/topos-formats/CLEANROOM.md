# Clean-room record: the EPUB container reader

`topos-bible-formats` (CC0-1.0) read EPUBs through the `epub` crate (version 2.1.5, GPL-3.0). A
GPL-3.0 dependency makes every compiled program that includes it (topos's and logoxide's
binaries, logoxide's npm package and Python wheels) GPL-3.0 as a whole. The owner wants their
work as close to the public domain as possible, so in October 2026 the `epub` crate is replaced by
a reader written for this crate, in a clean room, so that nothing of the `epub` crate's expression
is in it.

What topos needed from the `epub` crate is small: open the ZIP container, find the package
document that `META-INF/container.xml` names, list its manifest's items (id, path, media type,
properties), and read a file by its path. The CFIs, the spine, the table of contents, path
handling for links, and the content-document parsing were already topos's own code and are not
part of this rewrite.

## Rules (written before any specification or code)

**Two parties.** A *specification writer* writes what the reader must do; an *implementer*, in a
separate repository (`../topos-epub-cleanroom`), writes the reader from that specification alone.

**No one reads the `epub` crate.** Unlike topos-bible-markup's clean room (now logoxide-markup's,
in logoxide), nobody on either side studies the original: the reader's behaviour comes from the
public EPUB standards, not from the `epub` crate. Neither party may read, for any purpose:

- the `epub` crate's source, in any version (crates.io, `~/.cargo/registry/src/*/epub-*`, its git
  repository), its README, examples, tests, or documentation (docs.rs);
- compiled artifacts of it under any `target/` folder;
- compiler or tool output that quotes its source (so the dependency is removed before the new
  reader is built anywhere it could be compiled alongside it);
- any other GPL or proprietary EPUB reader's code.

Black-box use is allowed: the specification writer may run topos as it was (with the `epub` crate
compiled in, called only through topos's own public API) to record outputs, and compare them with
the new reader's.

**Inputs the specification writer may use:** the public standards (W3C EPUB 3.3, IDPF OPF 2.0.1
and OCF 2.0.1, the ZIP application note, RFC 3986 and the WHATWG URL standard), topos's own code
(CC0, including its calls into the `epub` crate, which show which operations topos needs but not
how the crate does them), and black-box outputs as above.

**Inputs the implementer may use:** what is in `../topos-epub-cleanroom` (the specification
`docs/spec/epub-container.md`, the interface `docs/spec/api.md`, this record, and a crate skeleton
that does not depend on `epub`), the public standards above, and the public source and
documentation of the dependencies it is given (`zip`, `roxmltree`). It does not read topos.

**Specification first.** A difference found by testing is resolved by changing the specification
(from the standards or black-box observation), then the code.

## What the specification writer had seen

The specification writer is Claude (Opus 5.5, Anthropic), in a Claude Code session working for
the owner. Before writing the specification it had read topos's `crates/topos-formats/src/epub.rs`
(which calls the `epub` crate's `EpubDoc::new`, `EpubDoc::from_reader`, `root_file`, `resources`,
`get_resource_by_path`, and `get_resource_str_by_path`), the `epub` crate's name, version, and
license as `cargo metadata` lists them, and sections 4 and 5 of W3C EPUB 3.3. It has not read the
`epub` crate's source or documentation.

## Limits

- The implementer is an AI model whose training data may have included public code, the `epub`
  crate's among it. It does not consult such code during the work, but a model cannot show what it
  has not been trained on.
- This is not legal advice.

## Log

1. This record and its rules.
2. The specification (`docs/spec/epub-container.md`) and interface (`docs/spec/api.md`), from
   EPUB 3.3 §4–5 and OPF/OCF 2.0.1; choices the standards leave open are marked in it.
3. The reader (`src/lib.rs`), its tests (`tests/`), and `docs/spec/gaps.md`, by the implementer:
   Claude (Opus 5.5, Anthropic), in a fresh Claude Code session that had not seen the `epub`
   crate, topos, or any other EPUB reader's code. It read only this repository (`CLEANROOM.md`,
   `README.md`, `docs/spec/api.md`, `docs/spec/epub-container.md`, the skeleton, `Cargo.toml`,
   `Cargo.lock`) and, on the web, two docs.rs pages for the `zip` crate 3.0.0 (`ZipArchive` and
   `ZipWriter`); no standards pages were fetched (the standards cited are known to it from
   training). It didn't open `~/.cargo`, any other folder outside this repository, or any EPUB
   reader's source or documentation. Thirteen questions the specification left open are decided
   in `gaps.md` from EPUB 3.3, XML 1.0, RFC 3986, and the URL Standard: which `rootfile`s are
   usable, how `container.xml` is found and read, whitespace and backslash network paths in URLs,
   a final `.` or `..`, bases without a final `/`, duplicate and empty entry names, which errors
   are `Zip` or `Io`, the wording of the messages the specification didn't give, and a declared
   size over the limit.
4. Black-box record, before the switch (by the specification writer): every distinct EPUB on the
   owner's machine (1,760 files, by SHA-256; books the owner keeps, not redistributable, so
   neither they nor their outputs are in any repository) was run through topos as it was, with
   the `epub` crate compiled in, called only through topos's public API (`search_epub_reader`,
   with and without `[id]` assertions, and `epub_text_reader`, with the default `BibleMatcher`).
   1,755 read (1,054,460 references); 5 are damaged ZIP archives that couldn't be read.
5. The switch: the `epub` dependency was removed from `Cargo.toml` first, then the reader was
   copied unchanged from `../topos-epub-cleanroom` (commit `68fb884`) into
   `src/epub/container.rs` (only its doc test's crate path changed) with `docs/spec/gaps.md`, and
   topos's calls into the `epub` crate were rewritten to it. Compiled copies of the `epub` crate in
   `target/` folders were deleted without being opened.
6. Black-box comparison, after the switch: the same 1,760 files give identical results for all
   1,755 that read: every reference, its CFIs (both forms), chapter, line, column, and the whole
   book text. The 5 damaged files still fail, with differently worded errors. topos's tests (325),
   logoxide's (127), and logoxide's bindings' (16) pass, and logoxide's bindings build for
   WebAssembly. Neither workspace depends on any GPL crate with its default features (`cargo
   metadata`); the opt-in `pdf` feature still brings in MuPDF (AGPL-3.0).
7. Limits on what one container reads (October 2026), against ZIP bombs: many entries, each under
   `MAX_FILE_BYTES`, could still exhaust memory. The specification first
   (`docs/spec/epub-container.md` section 1, `docs/spec/api.md`): `MAX_TOTAL_BYTES` (1 GiB, all
   that one container decompresses, `TooMuchData`) and `MAX_ENTRIES` (200,000, `TooManyEntries`),
   then the code and its tests. Both were written by Claude (Opus 5.5, Anthropic) in a Claude Code
   session for the owner that had read topos (this reader's code among it) and logoxide's own
   archive limits (`MAX_ENTRIES` there is the same number), from the ZIP application note; it
   didn't read the `epub` crate's source or documentation, nor any other EPUB reader's code.
