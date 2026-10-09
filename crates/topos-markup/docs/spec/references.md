# References the markup tags

`refs::tagged_references(nodes, data)` lists every element of kind `Reference` (OSIS
`<reference osisRef>`, ThML `<scripRef passage>` and `<a href>`, TEI `<ref target>` and `<xr>`,
USX `<ref loc>` and `<char style="xt">`, GBF `<RX>`), in document order, including references
inside notes and inside other references, with:

- `path`: the element's path;
- `target`: `Element::reference_target()` as an owned string (the first present of `osisRef`,
  `passage`, `target`, `loc`, `link-href`, `href`), or `None`;
- `passages`: `refs::passages(element, data)`.

`refs::passages(element, data)` reads the passages an element points to with topos-bible's
`BibleData` (`data`), trying in order, and returning the first success:

1. **OSIS**, when there is a target. The candidates are the target itself and then, if the target
   contains a `:` and the part before the first `:` is non-empty and has no whitespace, the part
   after that `:` (a work prefix such as `Bible:` or `KJV:` removed; OSIS 2.1.1 manual, osisRef
   syntax). For each candidate: it is split on whitespace; every word must parse with
   `data.books().parse_osis(word)`; if any fails, or there are no words, the candidate fails.
   Consecutive passages of the same book are merged into one passage (the later one's segments
   appended to the earlier one's); a different book starts a new passage. So
   `Bible:John.3.16-John.3.18 Rom.8.28` gives two passages, `John.3.16-John.3.18` and `Rom.8.28`,
   and `Gen.1.1 Gen.1.3 Exod.2.1` gives one passage for Genesis with two segments (its OSIS form is
   `Gen.1.1 Gen.1.3`) and one for Exodus.
2. **USFM**, when there is a target: `data.books().parse_usfm(target)` on the whole target (USX
   `JHN 3:16-18`, `GEN 1:1`): one passage.
3. **Written**, when there is a target: `data.parse(target)` (`John 3:16`): one passage.
4. **The element's text** (`Element::text()`, all descendants including notes):
   `data.parse(text)`: one passage, or none.

So a reference with no target, or a target that none of 1 to 3 can read, falls back to its text:
USX `<char style="xt">Rom 5:8</char>` gives `Rom.5.8`, and `<reference>John 3:16</reference>`
gives `John.3.16`. When nothing reads, `passages` is empty but the reference is still listed.

The passages are topos-bible `Passage` values; the golden files record them with
`Passage::to_osis(data.books())`.

## Examples

| Markup | Element | Passages (OSIS) |
|---|---|---|
| OSIS | `<reference osisRef="Bible:John.3.16-John.3.18 Rom.8.28">these</reference>` | `John.3.16-John.3.18`, `Rom.8.28` |
| OSIS | `<reference osisRef="John.1.5">John 1:5</reference>` (inside a note) | `John.1.5` |
| USX | `<char style="xt">Rom 5:8</char>` inside `<note style="x">` | `Rom.5.8` |
| USX | `<ref loc="JHN 3:16-18">…</ref>` | `John.3.16-John.3.18` |
| TEI | `<ref target="StrongsHebrew:011">H11</ref>` | none (target, then `011`, `H11` are not references) |
| OSIS | `<reference osisRef="">empty</reference>` | none (an empty target is still a target; then the text `empty` is not a reference) |
| OSIS | `<reference osisRef="Jn 3:16">x</reference>` | `John.3.16` (step 3) |
| ThML | `<scripRef passage="Gen 1:1-3; Exod 2:1">a</scripRef>` | `Gen.1.1-Gen.1.3` (step 3 gives one passage: what `data.parse` reads first) |
| GBF | `<RXJohn.1.5>John 1:5<Rx>` | `John.1.5`, from the text: GBF's `value` attribute is not a target |
| ThML | `<a href="sword://StrongsGreek/00025">G25</a>` | none |
