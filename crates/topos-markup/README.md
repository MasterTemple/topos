# topos-bible-markup

Bible markup as one tree: OSIS, ThML, GBF, TEI, USX, and plain text parse into the same
[`Node`]s, each element with a [`Kind`] that means the same in every format (a word, a note,
words of Jesus, a title). Parsing never fails: SWORD's verse fragments, unbalanced tags, and
stray `<` are all fine.

From the tree:

- **Plain text** for terminals and editors ([`plain_text`]), and a [`Flat`] text with a map
  back to the tree and the span of every element, for highlighting
- **HTML** with prefixed classes (`topos-woj`, `topos-note topos-crossref`) and data attributes
  (`data-strong`, `data-osis-ref`), never inline styles ([`HtmlRenderer`]), and `<mark>`s for
  search matches ([`HtmlRenderer::mark`])
- **Filters** that keep, unwrap, or drop elements by kind, note type, or a custom rule
  ([`Filter`])
- **Verses in a whole book** (USX, or OSIS with milestones), and the tree between two places, so
  a passage keeps its paragraphs and headings ([`verses`])
- **References**: the ones the markup tags, as topos passages ([`refs::tagged_references`]), and,
  with `topos-bible-formats`' `markup` feature, the ones written in the text, with their place
  in the tree

```rust
use topos_bible_markup::{Filter, HtmlRenderer, Kind, Markup, TextOptions, parse, plain_text};

let nodes = parse(Markup::Osis, r#"<w lemma="strong:G2316">God</w> so loved the world<note type="explanation">Or, cosmos</note>"#);
assert_eq!(plain_text(&nodes, &TextOptions::default()), "God so loved the world");

let with_notes = TextOptions { filter: Filter::text().keep(Kind::Note), ..TextOptions::default() };
assert_eq!(plain_text(&nodes, &with_notes), "God so loved the world [Or, cosmos]");

let html = HtmlRenderer::new().render(&nodes);
assert!(html.starts_with(r#"<span class="topos-w" data-strong="G2316""#));
```

The class names are listed on [`HtmlRenderer`] and are part of the API.

## Provenance

The parsers and classification came from `sword-modules`, where they were written from the
markup formats' public documentation, not from libsword (see that crate's `CLEANROOM.md`).

## License

CC0-1.0
