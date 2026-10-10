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

**No one reads the `epub` crate.** Unlike topos-bible-markup's clean room, nobody on either side
studies the original: the reader's behaviour comes from the public EPUB standards, not from the
`epub` crate. Neither party may read, for any purpose:

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
