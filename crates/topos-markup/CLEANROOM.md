# Clean-room record

> **Where this happened.** The rewrite was done in a separate repository,
> `topos-bible-markup-cleanroom`, whose history holds each step below (the hand-over, the
> implementation, the depth limit). Its result replaced this crate's code in October 2026.
> topos's own git history before that commit still contains the earlier implementation, which
> shared the old sword-modules' provenance; judge published versions from this commit on.

`topos-bible-markup` (topos's `crates/topos-markup`, CC0-1.0) was written by an author who had
read libsword, CrossWire's GPL-2.0 C++ SWORD library, whose filters do similar work. To be free of
any of its expression, the crate is rewritten in two parties:

- a **specification writer** (the "dirty side"), who read the existing crate and wrote what it
  does, and
- an **implementer**, who never sees the existing crate and writes it again from this folder.

## The specification writer

The specifications, tests, and golden files in this folder were written by Claude (Opus 5.5,
Anthropic), working as the specification writer in a Claude Code session in October 2026, for
the crate's owner.

The writer read:

- the existing crate's source, README, doc comments, unit tests (`src/tests.rs` and the inline
  test in the entities module), and its fuzz example;
- how its dependents use it: topos's `topos-formats` and `topos-ffi` (`topos-index` and
  `topos-cli` do not use it), logoxide's `logoxide` and `logoxide-lsp`, and
  `sword-modules-cleanroom/src`;
- the topos-bible API it calls (`../topos/crates/topos-lib`);
- the public standards cited in `docs/spec/README.md`.

The writer did not read libsword or any other SWORD reader's code.

## What the specifications contain

`docs/spec/` describes observable behaviour: inputs, outputs, rules, tables of facts (element to
kind mappings, the entity table, class names), edge cases, and tie-breaks, with worked examples.
They contain no implementation code. The only Rust in them is `docs/spec/api.md`: the public
items' signatures, derives, public fields, and doc comments (including the doc tests that are part
of the documentation), without bodies or private fields. The internal structure, private helpers,
and algorithms of the existing crate are not described; where behaviour needed stating, it is
stated as a rule about inputs and outputs.

The tests in `tests/` were written anew for the public API: the behaviours the existing unit tests
checked are re-expressed as integration tests, and more were added. They use only public items.

## How the golden files were made

The inputs in `tests/golden/` were gathered from the existing tests and doc examples, the
`sword-modules-cleanroom` test fixtures (entries of generated OSIS, TEI, and ThML modules and
their OSIS source), USX of the Berean Standard Bible and King James Version packages in
`~/Development/logoxide-packages` (the KJV converted from USFM with the `usfm3` crate), and
hand-made edge cases (malformed markup, unclosed and orphan tags, unknown elements, odd entities
and attributes).

The expected outputs were produced by a small program outside this folder that depends on the
existing crate as a library and calls only its public API, treating it as a black box. That
program computes the outputs with the same code that `tests/common/golden.rs` contains (it
includes that file), so the hand-over's harness recomputes exactly what was recorded. Inputs on
which the existing crate panicked were left out of the goldens and put in
`tests/regressions.rs` with the required behaviour instead.

Before hand-over, every test here was run against the existing crate: all pass except the three
regression tests (where the existing crate panics, as documented), and the no-panic test passes
when the inputs that trigger that known panic are excluded.

Every file in the hand-over was reviewed for pasted or paraphrased implementation code; none
remains.

## The implementer

`src/` was written by Claude (Opus 5.5, Anthropic), working as the implementer in a separate
Claude Code session in October 2026, for the crate's owner.

The implementer read only:

- this folder: `CLAUDE.md`, this file, `README.md`, `Cargo.toml`, `docs/spec/`, and `tests/`
  (including the golden files);
- topos's `crates/topos-lib` (the `topos-bible` crate: `BibleData`, `Books::parse_osis` and
  `parse_usfm`, `Passage`, `Passage::ranges`);
- the source of the crates.io crate `quick-xml` 0.42, which the crate uses to read the XML-like
  markups (its reader configuration, its attribute iterator, and how it reports errors and byte
  order marks);
- public standards, where the specifications cite them.

It did not open `../topos/crates/topos-markup`, topos's git history, any other copy of
`topos-bible-markup` (in `~/.cargo/registry`, `target/` folders, or elsewhere), libsword or any
other SWORD reader, `../sword-modules`, or the program that generated the goldens. The private
structure of `src/` (a tree builder fed by quick-xml and a GBF scanner, per-markup
classification, a flattener that writes pieces and separators, an HTML renderer, and the verse,
slice, and reference functions) was designed from the specifications.

### Gaps

`docs/spec/gaps.md` lists every gap, numbered 7 to 20 for those found while implementing, each
with the decision and what settled it. In short:

- settled by the goldens: a second words-of-Jesus start milestone writes no template text (13);
  the lexical layer is quick-xml 0.42's own reader, which every lexical golden agrees with (7);
- settled by the prose or the standards where the goldens are silent: end-tag names stop at the
  first whitespace (8); tag names and attributes split on XML whitespace (9); only one leading
  byte order mark is dropped (10); a nameless `<!DOCTYPE>` is unreadable from its `<` (11); a
  template's closing text ends the glue (12); text in an unwrapped words-of-Jesus element is not
  marked again in HTML (14); label ids split into at most three parts (15); labels are filled in
  one pass (16); `osisID` is the id when present (17); ThML morph classes are lowercased with
  Unicode rules (18); closing text counts as a piece of its entry (19);
- the depth limit (20), once the owner decided it: `parse` keeps at most 1,000 elements open,
  as parsing.md's "Depth limit" says, and the renderer's and flattener's recursion was split so
  that each level of nesting takes little stack (plain text, flattening, and HTML at depth 1,000
  each fit in 512 KB in a debug build); `tests/depth.rs` contradicts itself on how depth is
  counted, so its three depth assertions failed (21; the spec side then corrected the test).

### What is verified

- `cargo test --all-features`: 109 tests pass (102 integration tests in 17 files and 7 doc
  tests; 107 without the `serde` feature), including `tests/depth.rs` once the spec side
  corrected it (gap 21).
- The golden harness compares 2,489 sections in 221 golden files (OSIS 117, USX 28, ThML 25,
  TEI 21, GBF 20, plain text 10); all match byte for byte.
- `tests/regressions.rs` (the `&;` and `&é;` references, which the original crate panicked on)
  passes: they stay literal text.
- `NO_PANIC_ROUNDS=100000 cargo test --release --all-features --test no_panic` passes.
- `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` are clean.
- Library code has no `unwrap`, `expect`, or indexing that can go out of range.

### Limits

- The implementer is an AI model. Its training data may include public SWORD source and earlier
  public versions of topos, including this crate. This process keeps those sources out of the
  implementer's context, but it cannot rule out that the model remembers them. The code was
  written from the specifications, and no remembered code was consulted on purpose.
- Both parties are instances of the same model (Claude Opus 5.5), in separate contexts that
  share nothing but this folder. The separation is between sessions, not between people or
  between different models.
- This record describes a process. It is not legal advice, and it does not decide whether the
  result is free of any copyright claim.
