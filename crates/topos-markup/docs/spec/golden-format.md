# Golden files

`tests/golden/<markup>/<case>.golden` hold inputs and the outputs version 0.5.0 gave for them.
`tests/golden.rs` recomputes every output with the public API (`tests/common/golden.rs`) and
compares. The expected outputs were produced by running the original crate as a black box; they
are the reference wherever the prose specification is silent or unclear.

Changed on purpose since then (the prose was changed first):

- October 2026, verses.md: the first verse inside an OSIS chapter element holding its verses
  starts at the chapter's first child. 0.5.0 started it after the whole chapter, so its text was
  empty. The `verses` sections of `osis/chapter-element-with-verses`,
  `osis/fixture-osis-document-genesis-1`, and `osis/test-verse-elements` changed.

- October 2026, verses.md: a verse number's letter is its part (`VerseSpan::part`), and the
  verses line writes it after the verse (`1:2a-2`). The `verses` sections of
  `usx/verse-variants` and `osis/verse-n-attribute` changed.

## File format

A golden file is a sequence of sections. Each section starts with a header line

```text
#### <name> ####
```

where `<name>` is lowercase letters and `-`. The section's content is every line after the header
up to the next header, joined with `\n`. The file ends with `\n`, which is not part of the last
section. Content is exact: it may contain `\r`, tabs, trailing spaces, and empty lines, and no
content line has the form of a header. Files are UTF-8 and must be kept byte for byte
(`.gitattributes` turns off line-ending conversion for them).

Sections, in order:

| Section | Count | Content |
|---|---|---|
| `markup` | 1 | The markup name, for `Markup::from_name` (`osis`, `thml`, `gbf`, `tei`, `usx`, `plain`) |
| `input` | 1 or more | One entry each, in order |
| `tree` | one per input | That entry's tree, in the textual form below |
| `text` | 1 | `Flat::text` of a `Flattener` with `TextOptions::default()`, all entries pushed in order |
| `text-all` | 1 | The same with the "all" options below |
| `text-nobreaks` | 1 | The same with `line_breaks: false` and otherwise default options |
| `spans` | 1 | The default flattening's entries and spans (below) |
| `sources` | 1 | The default flattening's sources (below) |
| `html` | one per input | `render` of each entry with one `HtmlRenderer::new()`, in order |
| `html-bare` | one per input | The same with one renderer whose options are `class_prefix: ""` and `filter: Filter::all()` |
| `verses` | one per input | `verses` of that entry's tree (below) |
| `usx-chapter-verses` | one per input, USX only | `usx_chapter_verses` of that input, as Rust's `Debug` of the `Vec<u8>` (`[4, 1]`), then `\n` |
| `refs` | one per input | `refs::tagged_references` with `BibleData::default()` (below) |

The "all" options are: `Filter::all()`, `strongs: true`, `line_breaks: true`, verse label
`[{book} {chapter}:{verse}]`, chapter label `{book} {chapter}`, and templates `“{text}”` for
`WordsOfJesus`, `## {text}` for `Title`, `({text})` for `Note`, `{text}*` for `Added`, and `^` for
`DivineName`.

## Escaped strings

Strings inside dump lines are written in double quotes with these escapes: `\\` for a backslash,
`\"` for a double quote, `\n`, `\r`, `\t`, and `\u{h}` (lowercase hexadecimal, no padding) for any
other control character (Unicode general category Cc). Every other character is written as is.

## The textual tree

One line per node, in document order. Each line is indented two spaces per depth (top-level nodes
are not indented); a node's children follow it, one level deeper.

- A text node: `text ` followed by its escaped text: `text "In the beginning"`.
- An element: its kind's variant name (`Debug` of `Kind`: `Word`, `WordsOfJesus`, …), a space, its
  name as is (possibly empty, giving two spaces before what follows), then:
  - ` @start` or ` @end` for its milestone, if any;
  - ` @unclosed` if `unclosed`;
  - for each attribute in order, a space, the key as is, `=`, and the escaped value.

```text
Paragraph div @start sID="gen4" type="x-p"
text "And "
Word w lemma="strong:H0430"
  text "God"
WordsOfJesus FR @unclosed
  text "never closed"
```

The section's content is the lines, each ending with `\n`.

## Spans

First a line `entries:` followed by ` start..end` for each entry range. Then one line per span in
`Flat::spans` order:

```text
<Kind> <start>..<end> entry=<entry> path=<path> <escaped text of the range>
```

where `<path>` is the `NodePath` displayed (`/1/0`, or nothing for the empty path, giving
`path= "…"`). Each line ends with `\n`.

## Sources

The bytes `0..text.len()` of the default flattening that have a `source`, grouped into maximal runs
in which consecutive bytes come from the same entry and path with offsets going up by one. One
line per run: `<start>..<end> entry=<entry> path=<path> offset=<offset of the first byte>`. (A
run can span two adjacent pieces of one text node when nothing separates them in the text and the
offsets continue; the grouping is of the observable `source` results, not of the private pieces.)

## Verses

One line per `VerseSpan`, `<chapter>:<verse><part>-<last> marker=<path> start=<path> end=<path> "<text>"`,
where `<part>` is empty for part 0 and the lowercase letter for parts 1 to 26 (`a` to `z`), and
the text is the escaped `plain_text` of `slice(nodes, &start, Some(&end))` with
`TextOptions { filter: Filter::all(), ..default }`. Then `chapter_verses: ` and `Debug` of
`chapter_verses` (`[2]`), and `\n`.

## References

One line per tagged reference: `<path> target=<escaped target, or -> passages=[<OSIS>, …]`, each
passage as `Passage::to_osis(data.books())` (or `?` when that gives `None`), joined by `, `.

## The corpus

Inputs come from:

- the original crate's unit tests and doc examples (`test-*`, `doc-*`);
- the sword-modules-cleanroom test fixtures (`fixture-*`): entries of a generated OSIS Bible
  module (`TRaw`), TEI dictionary entries, ThML general-book entries, and the start of the OSIS
  source document;
- USX from real packages: whole small books and excerpts of the Berean Standard Bible (`bsb-*`)
  and the King James Version (`kjv-*`, converted from USFM with the `usfm3` crate);
- hand-made cases for every element mapping, malformed markup, unclosed and orphan tags, unknown
  elements, odd entities and attributes, labels, templates, and milestones.

Cases on which the original crate panics are not goldens; they are in `tests/regressions.rs`, with
the required behaviour.
