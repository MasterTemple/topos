# topos-bible-markup: behavioural specification

These files say what the crate does, observably, through its public interface. They are written
so that it can be implemented again without the original source. Where a rule comes from a public
standard it is cited; everything else is the behaviour of version 0.5.0, recorded from the outside.

| File | Area |
|---|---|
| [api.md](api.md) | Every public item: signatures, derives, doc comments, the `serde` feature |
| [parsing.md](parsing.md) | `parse`: reading OSIS, ThML, TEI, USX (XML-like), GBF, and plain text into the tree |
| [tree.md](tree.md) | `Node`, `Element`, `Kind`, `Milestone`, the element accessors, and each format's element-to-kind table |
| [entities.md](entities.md) | `decode_entities` and the named-entity table |
| [strongs.md](strongs.md) | `Strongs`, `StrongsLang`, Strong's numbers and morphology on words |
| [plain-text.md](plain-text.md) | `TextOptions` and `plain_text`: how the tree becomes text |
| [flatten.md](flatten.md) | `Flat`, `Flattener`, `Span`, `Point`: the text with a map back to the tree, across entries |
| [html.md](html.md) | `HtmlRenderer`, `HtmlOptions`: tags, class names (API), data attributes, marks |
| [filters.md](filters.md) | `Filter` and `Action` |
| [verses.md](verses.md) | `verses`, `VerseSpan`, `slice`, `passage_ranges`, `chapter_verses`, `usx_chapter_verses` |
| [node-paths.md](node-paths.md) | `NodePath` |
| [references.md](references.md) | `refs::tagged_references`, `refs::passages` |
| [golden-format.md](golden-format.md) | The golden files in `tests/golden/` and the textual tree form they use |
| [gaps.md](gaps.md) | Questions the specification leaves open, and how they were settled |

## Terms

- **Entry**: one piece of markup parsed on its own, typically one verse or one dictionary entry
  as a SWORD module stores it, or one whole USX book. Entries can be fragments: an element may be
  opened in one entry and closed in a later one.
- **Tree**: the `Vec<Node>` that `parse` returns for one entry. The top level is a list, not a
  single root.
- **Document order**: pre-order: an element comes before its children, and children in order.
- **Path**: a node's position as the list of child indexes from the top-level list down (see
  node-paths.md). Paths compare element by element, and a path that is a prefix of another sorts
  first, which is document order.
- **Place**: a path used as a position between nodes; it may be one past the last child of its
  parent (verses.md).
- **Whitespace**: Unicode whitespace (the `White_Space` property, as Rust's `char::is_whitespace`)
  unless a rule says ASCII.
- **Case-insensitive**: ASCII case only, unless stated.

## Principles that hold everywhere

1. No function panics on any input: any string to `parse` or `decode_entities` or
   `Strongs::parse`, any tree (including hand-built ones), any paths, any points, any options.
2. Parsing never fails. Whatever cannot be read as markup is kept as text.
3. Output is deterministic: the same input and options give byte-identical output.
4. Byte offsets and ranges given out by the crate (in `Flat`, `Span`, `Point`) are always on
   UTF-8 character boundaries of the string they index (for `Point`, the text node's text).

## Standards cited

- OSIS 2.1.1 User's Manual (Bible Technologies Group, 2006), "OSIS 2.1.1 User Manual 06March2006",
  and the OSIS 2.1.1 schema (`osisCore.2.1.1.xsd`).
- ThML 1.04 DTD and documentation (Christian Classics Ethereal Library, Harry Plantinga).
- GBF: General Bible Format specification (Online Bible / CrossWire's public GBF reference
  table of `<XX>` tokens).
- TEI P5 Guidelines, chapter 9 "Dictionaries" (`entryFree`, `entry`, `superEntry`, `orth`,
  `pron`, `def`, `ref`, `xr`, `hi`, `emph`, `lb`).
- USX 3.0 documentation (ubsicap.github.io/usx), and the USFM 3.0 marker reference for the
  `style` values.
- HTML Living Standard, section 13.5 "Named character references" (for entity names), and
  XML 1.0 (Fifth Edition) section 4.1 (character and entity references) and 2.4 (predefined
  entities).
- Strong's Exhaustive Concordance numbering (H1 to H8674, G1 to G5624).
