# Public interface

This is the interface that must be kept exactly: names, module paths, signatures, derives, public
fields (and their order, which matters to `serde` formats such as postcard), and doc comments.
Bodies and private fields are left out; a struct shown with `/* private fields */` has private
state the implementation chooses. Behaviour is in the other spec files.

The crate is `topos-bible-markup` 0.5.0 (library name `topos_bible_markup`), edition 2024.

## Crate layout

```text
topos_bible_markup
├── (root)    Markup, Node, Kind, Milestone, Element, StrongsLang, Strongs,
│             parse, walk, find_all, strongs_numbers
│             re-exports: decode_entities, Action, Filter, Flat, Flattener, Point, Span,
│                         HtmlOptions, HtmlRenderer, NodePath, TextOptions, plain_text
├── filter    Action, Filter
├── flatten   Point, Span, Flat, Flattener, flatten
├── html      HtmlOptions, HtmlRenderer
├── path      NodePath
├── refs      TaggedReference, tagged_references, passages
├── text      TextOptions, plain_text
└── verses    VerseSpan, verses, slice, passage_ranges, chapter_verses, usx_chapter_verses
```

The modules `filter`, `flatten`, `html`, `path`, `refs`, `text`, and `verses` are public. There is
no public `entities` module: `decode_entities` is reachable only at the root. The crate-level
documentation is the README (`#![doc = include_str!("../README.md")]`), so the README's example is
a doc test.

## Dependents and what they use

Checked in October 2026; nothing here may be removed or changed in type.

| Dependent | Uses |
|---|---|
| topos `topos-formats` (feature `markup`) | `Flat` (`text`, `source_range`), `Point` (as a public field type; needs `Clone`, `Debug`, `PartialEq`, `Eq`), `flatten::flatten`, `parse`, `Markup`, `TextOptions`; doc links to `Filter`, `Flattener`, `HtmlRenderer::mark` |
| topos `topos-ffi` (feature `markup`) | `Filter::all/drop`, `HtmlOptions` (struct literal with both fields), `HtmlRenderer::with_options/render`, `Kind`, `Markup::from_name`, `Node`, `TextOptions` (struct update syntax), `find_all`, `parse`, `plain_text`, `Element::attr`, `verses::{VerseSpan, chapter_verses, passage_ranges, slice, usx_chapter_verses, verses}`, `VerseSpan.start/.end` |
| logoxide `logoxide` | re-exports the crate as `logoxide::markup`; `Filter::text`, `Kind`, `Node` (matches `Node::Element`), `Element.kind`, `Element::strongs`, `TextOptions`, `plain_text`, `parse`, `Markup::Usx`, `verses::{VerseSpan, chapter_verses, slice, usx_chapter_verses, verses}`; stores `Vec<Node>` and `Vec<VerseSpan>` with serde (postcard) |
| logoxide `logoxide-lsp` | `Filter::all/drop/unwrap`, `Flat` (`text`, `spans`, `entries`), `Flattener::new/push/finish`, `flatten::flatten`, `Kind` (as a `BTreeMap` key in `TextOptions::templates`), `NodePath` (as a `HashMap` key: `Hash` + `Eq`), `TextOptions` (fields `filter`, `verse_label`, `chapter_label`, `templates`), `Span` (`range`, `kind`, `path`), `VerseSpan` (`chapter`, `verse`, `last`, `marker`, `start`, `end`; `part` and `last_chapter` since October 2026), `passage_ranges`, `slice` |
| sword-modules-cleanroom | re-exports the crate as `markup` and `Filter`, `HtmlRenderer`, `Kind`, `Node`, `Strongs`, `TextOptions`, `Markup`; `parse`, `plain_text`, `strongs_numbers`, `HtmlRenderer::new/render`, `Markup::from_name`, `Markup::{Gbf, Plaintext}` |
| topos `topos-index`, `topos-cli` | none |

`Filter`, `TextOptions`, `HtmlOptions`, `HtmlRenderer`, `Flattener`, `Flat`, and every tree type
must be `Send + Sync` (dependents keep them in shared state).

## The `serde` feature

`serde = ["dep:serde"]`, off by default. With it, these types derive `serde::Serialize` and
`serde::Deserialize` with serde's derive defaults (no renames, no skipped fields, externally
tagged enums, `NodePath` as a newtype struct):

- `Node`, `Element`, `Kind`, `Milestone` (the tree)
- `NodePath`
- `verses::VerseSpan`

Nothing else derives them: not `Markup`, `Strongs`, `StrongsLang`, `Filter`, `Action`,
`TextOptions`, `Flat`, `Span`, `Point`, `TaggedReference`, or the HTML types. Because derived
formats such as postcard depend on field and variant order, the field order of `Element` and
`VerseSpan` and the variant order of `Node`, `Kind`, and `Milestone` below are part of the
interface (logoxide caches parsed books this way).

## Root

```rust
pub use entities::decode_entities; // see below
pub use filter::{Action, Filter};
pub use flatten::{Flat, Flattener, Point, Span};
pub use html::{HtmlOptions, HtmlRenderer};
pub use path::NodePath;
pub use text::{TextOptions, plain_text};

/// Decode entities in `s`.
pub fn decode_entities(s: &str) -> String;

/// The markup a text is written in
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Markup {
    Osis,
    Thml,
    Gbf,
    Tei,
    /// Paratext's XML form of USFM
    Usx,
    Plaintext,
}

impl Markup {
    /// A markup by name, ignoring case: `osis`, `thml`, `gbf`, `tei`, `usx`, `plaintext` or
    /// `plain`
    pub fn from_name(name: &str) -> Option<Markup>;
}

/// A node in parsed markup
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Node {
    Text(String),
    Element(Element),
}

impl Node {
    pub fn as_element(&self) -> Option<&Element>;
    pub fn as_text(&self) -> Option<&str>;
}

/// What an element means, the same across markup formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Kind {
    /// A word, usually annotated with Strong's numbers (`lemma`, USX `strong`) and morphology
    /// (`morph`)
    Word,
    /// Footnote, study note, or cross-reference note; see [`Element::note_type`]
    Note,
    /// Link to scripture or another entry; see [`Element::reference_target`]
    Reference,
    /// Heading or title; [`Element::title_type`] says which (`psalm`, USX `s1`, ...)
    Title,
    /// Words of Christ ("red letter"): OSIS `<q who="Jesus">`, GBF `<FR>`, USX `wj`
    WordsOfJesus,
    /// Other quotation (OSIS `<q>`, GBF `<FO>`, ThML `<q>`/`<blockquote>`, USX `qt`)
    Quote,
    /// Translator additions (KJV italics): OSIS `<transChange>`, USX `add`
    Added,
    /// LORD in small caps: OSIS `<divineName>`, USX `nd`
    DivineName,
    /**
    Visual emphasis; the style is normalised to `italic`, `bold`, `underline`, `super`, `sub`,
    `small-caps`, ... (see [`Element::style`])
    */
    Highlight,
    Foreign,
    Paragraph,
    LineGroup,
    /// A poetic line (OSIS `<l>`, USX `q1`)
    Line,
    LineBreak,
    /// A generic division (`<div>`) that is not a paragraph
    Division,
    /// A chapter (usually a milestone)
    Chapter,
    /// A verse (a milestone in USX and some OSIS)
    Verse,
    /// OSIS `<milestone>` (e.g. `type="x-p"` pilcrow, `type="pb"`)
    Milestone,
    Image,
    /// Dictionary entry container (TEI `<entryFree>`/`<entry>`)
    Entry,
    /// Content that isn't for reading, like USX's `<book>` identification line
    Metadata,
    /// Anything else; inspect [`Element::name`] and attributes
    Other,
}

impl Kind {
    /// The class name the HTML renderer gives this kind (before the prefix)
    pub fn class_name(self) -> &'static str;
}

/// Which half of a split element this is
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Milestone {
    /// `sID` milestone: the element starts here and continues past this point
    Start,
    /// `eID` milestone, or an end tag whose start was in an earlier entry
    End,
}

/// An element of parsed markup
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Element {
    pub kind: Kind,
    /// Original tag name (GBF: the token code such as `FR`, `RF`, `CM`; ThML/GBF Strong's-tagged
    /// words are synthesised as `w`; USX `para`/`char` elements keep their tag, and their
    /// `style` attribute says which)
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    pub milestone: Option<Milestone>,
    /// Opened in this entry but not closed before its end
    pub unclosed: bool,
}

impl Element {
    /// Attribute value by name (exact match first, then ASCII case-insensitive)
    pub fn attr(&self, name: &str) -> Option<&str>;

    /// Concatenated text of all descendants (notes included)
    pub fn text(&self) -> String;

    /// Strong's numbers from `lemma` (OSIS `strong:H07225`, synthesised for ThML and GBF) and
    /// USX's `strong`
    pub fn strongs(&self) -> Vec<Strongs>;

    /// Morphology codes from `morph`, e.g. `robinson:V-AAI-3S`; the scheme prefix is kept
    pub fn morph(&self) -> Vec<&str>;

    /// For notes: `type` (`crossReference`, `explanation`, `study`, `translation`, `footnote`, ...)
    pub fn note_type(&self) -> Option<&str>;

    /// Whether this note is a list of cross references
    pub fn is_cross_reference(&self) -> bool;

    /// For titles: `type` (OSIS `psalm`, `chapter`, ...; USX's paragraph style, like `s1`)
    pub fn title_type(&self) -> Option<&str>;

    /// Link target: `osisRef`, ThML `passage`, TEI `target`, USX `loc` or `link-href`, or `href`
    pub fn reference_target(&self) -> Option<&str>;

    /// Normalised highlight style (`italic`, `bold`, ...) for [`Kind::Highlight`]
    pub fn style(&self) -> Option<&str>;

    /// This element and all descendant elements, depth-first (pre-order)
    pub fn descendants(&self) -> Vec<&Element>;
}

/// Visit every element in a node list depth-first (pre-order)
pub fn walk<'a>(nodes: &'a [Node], f: &mut impl FnMut(&'a Element));

/// All elements of a kind, depth-first
pub fn find_all(nodes: &[Node], kind: Kind) -> Vec<&Element>;

/// All Strong's numbers in a node list, in reading order (duplicates kept)
pub fn strongs_numbers(nodes: &[Node]) -> Vec<Strongs>;

/**
Parses text in the given markup; this never fails
- Fragments are fine: an end tag without a start becomes a [`Milestone::End`] element, and a
  start tag still open at the end is closed and marked [`Element::unclosed`]
- Text that can't be read as XML from some point on is kept as text
- In plain text, line breaks become [`Kind::LineBreak`] elements
*/
pub fn parse(markup: Markup, text: &str) -> Vec<Node>;

/// Strong's language
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StrongsLang {
    Hebrew,
    Greek,
}

/// A Strong's number such as `H7225` or `G25`, with an optional suffix (`H0430a` → `a`)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Strongs {
    pub lang: StrongsLang,
    pub number: u32,
    pub suffix: Option<String>,
}

impl Strongs {
    /// Parses one token: `strong:H07225`, `x-Strongs:G25`, `G25`, `H0430a`
    pub fn parse(token: &str) -> Option<Strongs>;

    /// Parses a whitespace-separated `lemma` attribute, keeping only Strong's tokens
    pub fn parse_list(lemma: &str) -> Vec<Strongs>;

    /// Key for looking the number up in SWORD's StrongsGreek/StrongsHebrew modules
    pub fn dictionary_key(&self) -> String;

    /// Conventional dictionary module name: `StrongsGreek` / `StrongsHebrew`
    pub fn dictionary_module(&self) -> &'static str;
}

impl std::fmt::Display for Strongs { /* `G25`, `H430a`: see strongs.md */ }
```

## `filter`

```rust
//! Which elements are rendered, flattened and searched.

/// What happens to an element
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Render the element and its contents
    Keep,
    /// Leave out the element (no tag, class, or decoration) but keep its contents
    Unwrap,
    /// Leave out the element and its contents
    Drop,
}

/**
Decides, for each element, whether it is kept, unwrapped, or dropped

The first that applies wins:
1. Custom rules ([`Filter::rule`]), latest first
2. Note types ([`Filter::note_type`]), for [`Kind::Note`]
3. Kinds ([`Filter::kind`])
4. [`Action::Keep`]

```
use topos_bible_markup::{Action, Filter, Kind, Markup, TextOptions, parse, plain_text};

let nodes = parse(Markup::Osis, r#"<title>The Word</title>In the beginning<note type="crossReference">Gen 1:1</note><note type="explanation">Or, origin</note>"#);
// The default is the verse text: no notes or titles
assert_eq!(plain_text(&nodes, &TextOptions::default()), "In the beginning");
// Footnotes but not cross references
let filter = Filter::text().keep(Kind::Note).note_type("crossReference", Action::Drop);
let opts = TextOptions { filter, ..TextOptions::default() };
assert_eq!(plain_text(&nodes, &opts), "In the beginning [Or, origin]");
```
*/
#[derive(Clone, Default)]
pub struct Filter { /* private fields */ }

impl Filter {
    /// Everything is kept
    pub fn all() -> Self;

    /// The text to read: notes, titles, and metadata are dropped
    pub fn text() -> Self;

    /// Sets what happens to a kind of element
    pub fn kind(self, kind: Kind, action: Action) -> Self;

    pub fn keep(self, kind: Kind) -> Self;

    pub fn unwrap(self, kind: Kind) -> Self;

    pub fn drop(self, kind: Kind) -> Self;

    /// Sets what happens to notes of a type (`crossReference`, `footnote`, `study`, ...; ignoring
    /// case), whatever is set for [`Kind::Note`]
    pub fn note_type(self, note_type: &str, action: Action) -> Self;

    /// A custom rule, checked before everything else; [`None`] passes the element on
    pub fn rule(
        self,
        rule: impl Fn(&Element) -> Option<Action> + Send + Sync + 'static,
    ) -> Self;

    /// What happens to this element
    pub fn action(&self, element: &Element) -> Action;
}

impl std::fmt::Debug for Filter { /* see filters.md */ }
```

## `flatten`

```rust
//! Plain text with a map back to the tree: what plain-text rendering, search, and editor
//! highlighting are built on.

/// A position in a tree: a byte offset in the text node at `path`, in entry `entry`
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Point {
    /// Which pushed entry (0 for the first), see [`Flattener::push`]
    pub entry: usize,
    /// The text node
    pub path: NodePath,
    /// A byte offset into the text node's text
    pub offset: usize,
}

/// Where an element's content is in the flattened text
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Byte range in [`Flat::text`], without surrounding whitespace; milestones (verse and
    /// chapter markers) are empty ranges where they are
    pub range: Range<usize>,
    pub kind: Kind,
    pub entry: usize,
    /// The element; for words of Jesus between milestones, the start milestone (or the first
    /// entry's top level when it started in an earlier entry)
    pub path: NodePath,
}

/**
Flattened text
- Runs of whitespace are one space, and line breaks are kept (at most one blank line); the text
  has no leading or trailing whitespace
- Text that isn't in the tree (note brackets, Strong's numbers) has no source
*/
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Flat {
    pub text: String,
    /// Every kept element and milestone, in document order
    pub spans: Vec<Span>,
    /// Where each pushed entry's text is, without the whitespace around it (an empty range where
    /// it would be, for an entry with no text)
    pub entries: Vec<Range<usize>>,
    /* private fields (what `source` and `source_range` read) */
}

impl Flat {
    /// Where a byte of [`Flat::text`] came from, or [`None`] for added text and whitespace
    /// between runs
    pub fn source(&self, byte: usize) -> Option<Point>;

    /**
    Where a range of [`Flat::text`] came from: the start of its first source text and the end of
    its last, or [`None`] if none of it is from the tree
    - The end is exclusive, like the range
    */
    pub fn source_range(&self, range: Range<usize>) -> Option<(Point, Point)>;

    /// The spans that contain a byte range, outermost first
    pub fn spans_at(&self, range: Range<usize>) -> impl Iterator<Item = &Span>;

    /// The last milestone or element of a kind that starts at or before a byte, like the
    /// [`Kind::Verse`] a search match is in
    pub fn last_before(&self, kind: Kind, byte: usize) -> Option<&Span>;
}

/**
Flattens entries one after another, carrying words of Jesus that continue from one entry into the
next (with `sID`/`eID` milestones or an unclosed `<q>`)

```
use topos_bible_markup::{Flattener, Kind, Markup, TextOptions, parse};

let mut flat = Flattener::new(TextOptions::default());
flat.push(&parse(Markup::Osis, r#"He said, <q who="Jesus">Follow"#));
flat.push(&parse(Markup::Osis, "me.</q> And they did."));
let flat = flat.finish();
assert_eq!(flat.text, "He said, Follow me. And they did.");
let red: Vec<&str> = flat.spans.iter()
    .filter(|s| s.kind == Kind::WordsOfJesus)
    .map(|s| &flat.text[s.range.clone()])
    .collect();
assert_eq!(red, ["Follow", "me."]);
```
*/
#[derive(Debug, Clone)]
pub struct Flattener { /* private fields */ }

impl Flattener {
    pub fn new(opts: TextOptions) -> Self;

    /// Adds an entry (a verse, say), separated from the previous one by a space
    pub fn push(&mut self, nodes: &[Node]) -> &mut Self;

    pub fn finish(self) -> Flat;
}

/// Flattens one entry (see [`Flattener`])
pub fn flatten(nodes: &[Node], opts: &TextOptions) -> Flat;
```

Note on two doc comments, which are kept as they are: `Flat::spans` says "in document order" and
`Flat::spans_at` says "outermost first", but the behaviour (flatten.md) is that a span is listed
when its element ends, so an element's span comes after the spans inside it, and `spans_at`
yields spans in `Flat::spans` order.

## `html`

```rust
//! HTML with classes for applications to style.

/// Options for [`HtmlRenderer`]
#[derive(Debug, Clone)]
pub struct HtmlOptions {
    /**
    Put before every class name; default: `topos-`
    - The wrapper's class is the prefix without its trailing `-` or `_` (`topos`)
    - An empty prefix gives bare class names (`woj`) and a wrapper without a class
    - Data attributes are never prefixed
    */
    pub class_prefix: Cow<'static, str>,
    /// Which elements are rendered; default: [`Filter::all`] except metadata
    pub filter: Filter,
}

impl Default for HtmlOptions;

/**
Renders HTML with classes and data attributes, never inline styles

Classes (with the prefix, `topos-` by default):

| Element | Tag | Classes | Data attributes |
|---|---|---|---|
| word | `span` | `w` | `data-strong` (`H7225 H853`), `data-lemma`, `data-morph` |
| note | `span` | `note`, and `crossref`, `footnote`, `study`, ... from its type | `data-note-type`, `data-n`, `data-osis-ref` |
| reference | `a` | `ref` | `data-osis-ref` |
| title | `div` | `title`, and its type (`psalm`, `s1`) | `data-title-type` |
| words of Jesus | `span` | `woj` | |
| quote, translator addition, divine name, foreign | `span` | `quote`, `added`, `divine-name`, `foreign` | |
| highlight | `i`, `b`, `sup`, `sub`, `span` | `hi`, and its style (`italic`, `small-caps`) | |
| paragraph, line group, line | `p`, `div`, `span` | `p`, `lg`, `l` | |
| verse, chapter (milestones) | `span` | `verse`, `chapter` | `data-osis-id`, `data-n` |
| dictionary entry | `div` | `entry` | |
| mark ([`HtmlRenderer::mark`]) | `mark` | `mark` | `data-mark` (`0` for the first mark) |

Keep one renderer for a passage, so words of Jesus that continue from one verse into the next
(`sID`/`eID` milestones or an unclosed `<q>`) stay marked. [`HtmlRenderer::wrap`] puts a
passage in one element whose class is the bare prefix (`<div class="topos">`).

```
use topos_bible_markup::{HtmlRenderer, Markup, parse};

let mut html = HtmlRenderer::new();
let verse = html.render(&parse(Markup::Osis, r#"<w lemma="strong:G2424">Jesus</w> said, <q who="Jesus">Follow me</q>"#));
assert_eq!(
    verse,
    r#"<span class="topos-w" data-strong="G2424" data-lemma="strong:G2424">Jesus</span> said, <span class="topos-woj">Follow me</span>"#
);
assert_eq!(html.wrap(&verse), format!(r#"<div class="topos">{verse}</div>"#));
```
*/
#[derive(Debug, Default, Clone)]
pub struct HtmlRenderer {
    pub opts: HtmlOptions,
    /* private fields */
}

impl HtmlRenderer {
    pub fn new() -> Self;

    pub fn with_options(opts: HtmlOptions) -> Self;

    /// One entry's nodes; each call is the next entry, as in [`Flattener::push`](crate::Flattener::push)
    pub fn render(&mut self, nodes: &[Node]) -> String;

    /**
    Marks the text from `start` to `end` (exclusive) with `<mark class="topos-mark"
    data-mark="n">`, where `n` counts marks in the order they were added
    - The points are where a search match starts and ends, as `topos-bible-formats`'
      `MarkupLocation` gives them for a [`Flat`](crate::Flat) of the same entries (in the same
      order, with any filter): highlight search results without searching the HTML again
    - A mark across elements is one `<mark>` per text node, all with the same `data-mark`
    - Add marks before rendering the entries they are in

    ```
    use topos_bible_markup::{HtmlRenderer, Markup, TextOptions, flatten::flatten, parse};

    let nodes = parse(Markup::Osis, "See <hi type=\"italic\">John</hi> 3:16 today.");
    let flat = flatten(&nodes, &TextOptions::default());
    // A search match for `John 3:16` (`MarkupLocation` gives these points)
    let (start, end) = flat.source_range(4..13).unwrap();
    let mut html = HtmlRenderer::new();
    html.mark(start, end);
    assert_eq!(
        html.render(&nodes),
        r#"See <i class="topos-hi topos-italic"><mark class="topos-mark" data-mark="0">John</mark></i><mark class="topos-mark" data-mark="0"> 3:16</mark> today."#
    );
    ```
    */
    pub fn mark(&mut self, start: Point, end: Point) -> usize;

    /// Removes every mark
    pub fn clear_marks(&mut self);

    /// Wraps rendered HTML in a `div` whose class is the prefix without its trailing `-` or `_`
    pub fn wrap(&self, html: &str) -> String;
}
```

## `path`

```rust
//! Positions in a parsed tree.

/**
A node's position in a tree: the index of each node on the way down from the top-level list
- `[2, 0]` is the first child of the third top-level node
- Displayed as `/2/0`
*/
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodePath(pub Vec<usize>);

impl NodePath {
    /// The node at this path
    pub fn resolve<'a>(&self, nodes: &'a [Node]) -> Option<&'a Node>;

    /// The paths of the elements containing this node, outermost first
    pub fn ancestors(&self) -> impl Iterator<Item = NodePath> + '_;
}

impl std::fmt::Display for NodePath { /* `/2/0`; see node-paths.md */ }
```

## `refs`

```rust
//! References the markup tags itself: OSIS `<reference osisRef>`, ThML `<scripRef passage>`,
//! TEI `<ref target>`, and USX `<ref loc>` / `<char style="xt">`.

use topos_bible::{data::BibleData, segments::Passage};

/// A [`Kind::Reference`] element and the passages it points to
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedReference {
    pub path: NodePath,
    /// The target as written (`osisRef`, `passage`, `loc`, ...)
    pub target: Option<String>,
    /// One passage per book; empty when neither the target nor the text is a reference
    pub passages: Vec<Passage>,
}

/**
Every reference element, with the passages it points to

The target is read as OSIS (`John.3.16-John.3.18 Rom.8.28`, with or without a work prefix like
`Bible:`), then as USFM (USX's `JHN 3:16-18`), then as a written reference (`John 3:16`); failing
all three, the element's text is
```
use topos_bible::data::BibleData;
use topos_bible_markup::{Markup, parse, refs::tagged_references};

let data = BibleData::default();
let nodes = parse(Markup::Osis, r#"See <reference osisRef="Bible:John.3.16-John.3.18 Rom.8.28">these</reference>."#);
let refs = tagged_references(&nodes, &data);
let osis: Vec<String> = refs[0].passages.iter().map(|p| p.to_osis(data.books()).unwrap()).collect();
assert_eq!(osis, ["John.3.16-John.3.18", "Rom.8.28"]);
```
*/
pub fn tagged_references(nodes: &[Node], data: &BibleData) -> Vec<TaggedReference>;

/// The passages an element points to
pub fn passages(e: &Element, data: &BibleData) -> Vec<Passage>;
```

## `text`

```rust
//! Plain text, for terminals and editors.

/// Options for [`plain_text`] and [`Flattener`](crate::Flattener)
#[derive(Debug, Clone)]
pub struct TextOptions {
    /**
    Which elements are included; the default, [`Filter::text`], is the text to read (no notes,
    titles, or metadata)
    - Kept notes are in square brackets: `light [John 1:5]`
    - Kept titles are on their own line
    */
    pub filter: Filter,
    /// Append Strong's numbers after tagged words, like `beginning [H7225]`; default: false
    pub strongs: bool,
    /// Keep line and paragraph breaks as `\n`; otherwise they become spaces; default: true
    pub line_breaks: bool,
    /**
    Text written where each verse starts, like `[{verse}]`; default: none (verse markers are
    empty spans)
    - `{verse}` is the verse's number as written (`16`, or `4-5` for a bridge), `{chapter}` its
      chapter, and `{book}` the book in its id (`JHN` from `JHN 3:16`, `John` from `John.3.16`)
    - The verse's [`Span`](crate::Span) covers the label, which has no source in the tree
    */
    pub verse_label: Option<String>,
    /// Text written on its own line where each chapter starts, like `Chapter {chapter}` (with
    /// `{chapter}` and `{book}`); default: none
    pub chapter_label: Option<String>,
    /**
    How kept elements of a kind are written: a template with `{text}` for the element's text,
    like `“{text}”` for [`Kind::WordsOfJesus`] or `## {text}` for [`Kind::Title`]
    - Notes are `[{text}]` without one
    - The template's text is glued to the element's: `“ I am ”` is written `“I am”`
    - Without `{text}`, the template goes before the text
    - Not for verse and chapter markers (see the labels), paragraphs, lines, or line breaks
    */
    pub templates: BTreeMap<Kind, String>,
}

impl Default for TextOptions;

/// Plain text, with runs of whitespace collapsed (see [`Flat`](crate::Flat))
pub fn plain_text(nodes: &[Node], opts: &TextOptions) -> String;
```

## `verses`

The module's doc comment (kept verbatim, including its doc test):

````rust
/*!
Verses in a whole book's markup (USX, or OSIS with verse milestones), and the tree between two
places in it.

A book is one tree, and a verse is not an element in it: USX marks where each verse starts
(`<verse sid>`) and ends (`<verse eid>`), and a verse can start in one paragraph and end in the
next. [`verses`] finds where each verse is, and [`slice`] copies the tree between two places,
keeping the elements around the cut (a paragraph cut in two is two paragraphs), so a passage
renders with its paragraphs and poetry intact.

```
use topos_bible_markup::{HtmlRenderer, Markup, parse, verses::{slice, verses}};

let usx = r#"<usx><book code="JHN"/><chapter number="3" sid="JHN 3"/>
<para style="s1">Jesus and Nicodemus</para>
<para style="p"><verse number="16" sid="JHN 3:16"/>For God so loved the world.<verse eid="JHN 3:16"/><verse number="17" sid="JHN 3:17"/>For God did not send His Son.<verse eid="JHN 3:17"/></para>
<chapter eid="JHN 3"/></usx>"#;
let nodes = parse(Markup::Usx, usx);
let spans = verses(&nodes);
assert_eq!((spans[1].chapter, spans[1].verse), (3, 17));

// A verse starts where the one before it ends, so the heading before 3:16 belongs to it
let html = HtmlRenderer::new().render(&slice(&nodes, &spans[0].start, Some(&spans[0].end)));
assert!(html.contains("Jesus and Nicodemus") && html.contains("so loved") && !html.contains("did not send"));
```
*/
````

```rust
use std::ops::Range;
use topos_bible::segments::Passage;

/// Where one verse is in a book's tree
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VerseSpan {
    pub chapter: u8,
    pub verse: u8,
    /// The last verse, for a bridge like `<verse number="4-5">` (else the same as `verse`); in
    /// `last_chapter`, which a range across chapters ends in
    pub last: u8,
    /**
    The chapter the verse ends in: the same as `chapter`, unless its id is a range into a later
    chapter (`osisID="John.3.36-John.4.2"`, a commentary's note on both chapters' verses), when
    it is that chapter (4) and `last` is the verse there (2)
    */
    pub last_chapter: u8,
    /**
    Which part of `verse` this is: 0 for the whole verse, 1 for `a` (`<verse number="20a">`,
    USFM `\v 20a`), 2 for `b`, and so on to 26 for `z`
    - In a bridge like `20b-21` it is the first verse's part; the last verse counts whole
    - A verse's parts are separate spans with the same `chapter` and `verse`
    */
    pub part: u8,
    /// The verse's marker
    pub marker: NodePath,
    /**
    The first place in the verse: right after the verse before it ends (or after its chapter's
    marker), so a heading between two verses belongs to the second
    - Without end markers, the verse's own marker
    - A place is a path in document order; it can be one past the last child of an element
    */
    pub start: NodePath,
    /// The first place after the verse: its end marker, the next verse's or chapter's marker, or
    /// one past the last top-level node
    pub end: NodePath,
}

/**
Every verse in a book's tree, in order
- Chapters come from chapter markers (`<chapter number>`, OSIS `osisID="John.3"`); verse numbers
  from `number`, `n`, or the last part of `osisID`/`sID` (`John.3.16`, `JHN 3:16`)
- Verses that are elements with their text inside (OSIS without milestones) work too: the
  verse is that element
- A partial verse (`20a`, `20b`) is its own span, told apart by [`VerseSpan::part`]
- An id that is a range into a later chapter (`John.3.36-John.4.2`) is one span, from the first
  chapter to [`VerseSpan::last_chapter`]
*/
pub fn verses(nodes: &[Node]) -> Vec<VerseSpan>;

/**
A copy of the tree from `start` up to (not including) `end`; `None` is the end of the tree
- An element the cut goes through is copied with only its children inside the range, and
  dropped if none are left
- Places compare in document order, so `start` and `end` can be anywhere (see [`VerseSpan`])
*/
pub fn slice(nodes: &[Node], start: &NodePath, end: Option<&NodePath>) -> Vec<Node>;

/**
The places each part of a passage covers, as `start..end` for [`slice`], in the passage's order
- Verses the book doesn't have are left out; a part with none of its verses is left out
- A whole chapter is every verse the book has in it, whatever the versification says
- A verse's parts are all in it: `Genesis 1:20` covers both `20a` and `20b`
- A span across chapters is in a part that names any of its verses: `John 4:1` finds
  `John.3.36-John.4.2`
*/
pub fn passage_ranges(spans: &[VerseSpan], passage: &Passage) -> Vec<Range<usize>>;

/**
The number of verses in each chapter, from chapter 1 to the last chapter with verses, as the
book's own markers number them (a bridge like `4-5` counts to 5; a chapter without verses is 0)
- A span across chapters counts its first verse in its first chapter and its last in its last
- This is the book's versification, for checking references against this translation, like
  `Translations::set_chapter_verses` in topos-bible
*/
pub fn chapter_verses(spans: &[VerseSpan]) -> Vec<u8>;

/**
A USX book's verse counts, like [`chapter_verses`], from its `<chapter number>` and
`<verse number>` markers alone, without parsing the rest (fast enough to count a whole Bible)
*/
pub fn usx_chapter_verses(usx: &str) -> Vec<u8>;
```

Note: `passage_ranges` returns ranges of indexes into `spans` (not places); a range `r` is used as
`slice(nodes, &spans[r.start].start, Some(&spans[r.end - 1].end))`.

## Dependencies (Cargo.toml)

- `topos-bible` 0.5.0 (path `../topos/crates/topos-lib`): `BibleData`, `Passage`, `Books`
  (`parse_osis`, `parse_usfm`), `BibleData::parse`, `Passage::ranges`.
- `quick-xml` 0.42 (optional to use; it is what the XML-like readers' lexical behaviour in
  parsing.md was observed with).
- `serde` 1 with `derive`, optional, behind the `serde` feature.
