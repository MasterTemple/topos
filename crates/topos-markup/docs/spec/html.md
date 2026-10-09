# HTML

`HtmlRenderer` writes HTML fragments with classes and `data-` attributes, never inline styles.
Class names are API: applications style them. One renderer is meant to be kept for a passage:
`render` is called once per entry, in order, and words of Jesus and marks carry across calls.

## Options

`HtmlOptions { class_prefix, filter }`, default `topos-` and `Filter::all().drop(Kind::Metadata)`.
`HtmlRenderer::new()` (= `default()`) uses the default options; `with_options` others. The
options are the public field `opts` and can be changed between calls.

## Output conventions

- A tag is written `<tag`, then ` class="…"` when the class list is not empty, then each data
  attribute that has a value, in the order listed below, as ` name="value"`, then `>`.
- The class list is the element's class names, each with the prefix put before it, separated by
  single spaces; empty names are left out. With an empty prefix the names are bare.
- Text and attribute values are escaped: `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`, `"` →
  `&quot;`. Nothing else is escaped (`'` is written as is; non-ASCII is written as UTF-8).
- `br` is written without an end tag (`<br class="topos-lb">`); every other element gets one,
  even when empty (`<span class="topos-verse" data-n="1"></span>`).
- Nothing is added between elements: no newlines, no indentation.
- Data attribute values are the attribute values as stored (decoded), escaped as above; they are
  never prefixed.

**Class-safe** form of a value (for classes made from `type` and similar): each character that is
an ASCII letter, ASCII digit, or `-` is kept (letters lowercased); every other character
(including each non-ASCII character) becomes one `-`. `x-Odd Style!` → `x-odd-style-`;
`small-caps` → `small-caps`; `Italic` → `italic`.

## Elements

Each element goes through the filter first: `Drop` writes nothing; `Unwrap` writes its children
(with the current state) and nothing of its own; `Keep` writes by kind. In the table, `P` is the
prefix, "children" means the children rendered in order, and data attributes are listed in output
order.

| Kind and shape | Output |
|---|---|
| `WordsOfJesus` or `Quote` with a milestone | nothing (state only, below); its children are not written |
| `WordsOfJesus` | `<span class="Pwoj">` children `</span>`; children are inside words of Jesus |
| `Word` | `<span class="Pw">` children `</span>`; `data-strong`: `Element::strongs()` displayed and joined by single spaces (omitted when there are none); `data-lemma`: the `lemma` attribute; `data-morph`: the `morph` attribute |
| `Note` with children | `<span class="Pnote Ptype">` children `</span>`, where `type` is `crossref` when the note type is `crossReference` (ignoring ASCII case), else the class-safe note type (no second class when there is no type); `data-note-type`: `note_type()`; `data-n`: `n`, else `caller`; `data-osis-ref`: `osisRef` |
| `Reference` | `<a class="Pref">` children `</a>`; `data-osis-ref`: `reference_target()` (whichever attribute it comes from). No `href` |
| `Title` with children and no milestone | `<div class="Ptitle Ptype">` children `</div>`, `type` the class-safe `title_type()`; `data-title-type`: `title_type()` |
| `Quote`, `Added`, `DivineName`, `Foreign` (no milestone for quotes) | `<span class="Pquote">`, `Padded`, `Pdivine-name`, `Pforeign`, children, `</span>` |
| `Highlight` | the class-safe `style()`, call it `s`; the tag is `i` for `italic`, `b` for `bold`, `sup` for `super`, `sub` for `sub`, else `span`; `<tag class="Phi Ps">` children `</tag>` (just `Phi` without a style) |
| `LineBreak` | `<br class="Plb">` (children not written) |
| `Paragraph`, `End` milestone | nothing |
| `Paragraph`, no milestone, with children | `<p class="Pp">` children `</p>` |
| `Paragraph`, otherwise (a `Start` milestone, or empty) | `<br class="Pp">` (children of a `Start` milestone are not written) |
| `LineGroup` with no milestone | `<div class="Plg">` children `</div>` (also when empty) |
| `Line`, `End` milestone | `<br class="Pl">` |
| `Line`, `Start` milestone | nothing (children not written) |
| `Line`, no milestone | `<span class="Pl">` children `</span><br class="Pl">` |
| `Verse` or `Chapter` with no children, not an `End` milestone | `<span class="Pverse">` (or `Pchapter`) `</span>`; `data-osis-id`: `osisID`, else `sID`; `data-n`: `n`, else `number` |
| `Verse` or `Chapter` with no children, `End` milestone | nothing |
| `Entry` | `<div class="Pentry">` children `</div>` |
| everything else: `Division`, `Image`, `Metadata` (when kept), `Other`, `Milestone`, `Note` without children, `Title` with a milestone or without children, `LineGroup` with a milestone, `Verse`/`Chapter` with children | children only, no tag of its own |

Notes on the table:

- Images are not rendered as `img` (there is no source to point to); their children, if any, are.
- An empty note writes nothing; an empty title writes nothing; an empty `<lg/>` writes
  `<div class="Plg"></div>`.
- Verses and chapters that contain their text (OSIS without milestones) write only their text.

## Words of Jesus across milestones and entries

The renderer has an **open** state for words of Jesus that are not one element:

- A kept `WordsOfJesus` start milestone opens it, remembering its `sID` (if any). Unlike the
  flattener, a second start replaces the first.
- A kept `WordsOfJesus` element that is `unclosed` is rendered as usual and then opens it, with no
  id.
- A kept end milestone of kind `WordsOfJesus` or `Quote` closes it when it is open and matches, by
  the same matching rule as the flattener (flatten.md): ids equal when both exist; both absent
  matches; otherwise only a `WordsOfJesus` end matches.
- The state is kept between `render` calls.

While it is open, every text node that is not inside a `WordsOfJesus` element and is not
whitespace-only (by `trim`) is written as `<span class="Pwoj">` text `</span>`, one span per text
node. Whitespace-only text nodes are written plainly.

Example (one renderer, two entries):

```text
He said, <q who="Jesus" sID="q1"/>I am <q sID="q2"/>the way<q eID="q2"/>
and the life.<q eID="q1"/> Then
```

```html
He said, <span class="topos-woj">I am </span><span class="topos-woj">the way</span>
<span class="topos-woj">and the life.</span> Then
```

(The `q2` quote milestones are a quote inside the words of Jesus and do not end them.) And
`end of quote.</q> New <q who="Jesus">start` renders `… <span class="topos-woj">start</span>`;
rendering `more words</q> narration` next with the same renderer gives
`<span class="topos-woj">more words</span> narration`.

## Marks

`mark(start, end)` records a range to highlight and returns its number: 0 for the first mark added
to this renderer, then 1, 2, … (`clear_marks` empties the list; numbering then restarts at 0).
The points are positions as a `Flat` gives them (flatten.md) for the same entries in the same
order: `entry` counts `render` calls on this renderer (the first call is entry 0; the count is
never reset), `path` is the text node's path in that entry's tree, `offset` a byte offset in it.
The filter used to make the `Flat` does not matter, because paths are positions in the tree
itself (dropped and unwrapped elements still take up their index).

When a text node is written, at `(entry, path)` = the current entry and the node's path, each
mark is compared with it, ordering positions by entry and then by path (paths in document order):

- marks that start after this node or end before it do not apply;
- otherwise the marked bytes of this node are from the start offset (if the mark starts in this
  node) or 0, to the end offset (if it ends in this node) or the node's length, capped at the
  length;
- a part that is empty, or whose ends are not character boundaries of the text, is left out.

The parts are sorted by start (then end, then mark number) and overlapping or touching parts are
merged (a part that starts at or before the previous part's end extends it); a merged part keeps
the number of its first part. Each part is written as `<mark class="Pmark" data-mark="n">` text
`</mark>`, with the unmarked text around it escaped as usual. A mark that spans several text nodes
gives one `mark` element per node, all with the same number. Marks are inside any words-of-Jesus
span of the text node.

Example: entries `Read <w lemma="strong:G1">John</w>` and `3:16 and Rom 8:28 &amp; Rom 8:28.`,
flattened together to `Read John 3:16 and Rom 8:28 & Rom 8:28.`; marks added for the source ranges
of `5..14`, `19..27`, `30..38`, `28..38` (numbers 0 to 3); rendering both entries gives

```html
Read <span class="topos-w" data-strong="G1" data-lemma="strong:G1"><mark class="topos-mark" data-mark="0">John</mark></span>
<mark class="topos-mark" data-mark="0">3:16</mark> and <mark class="topos-mark" data-mark="1">Rom 8:28</mark> <mark class="topos-mark" data-mark="3">&amp; Rom 8:28</mark>.
```

(one line per entry here). Mark 2 is merged into mark 3, which starts first.

## `wrap`

`wrap(html)` is `<div class="X">` + `html` (as is, not escaped) + `</div>`, where `X` is the prefix
with all trailing `-` and `_` characters removed, escaped. If that leaves nothing, it is
`<div>` + `html` + `</div>`. Prefix `topos-` gives `<div class="topos">`, `bible_` gives
`<div class="bible">`, and the empty prefix gives `<div>`.

## Full example

OSIS `<title type="psalm">A Psalm</title><w lemma="strong:H3068" morph="x">LORD</w><note
type="crossReference" n="a">Ps 1:1</note><note type="explanation">Or, Jehovah</note> <hi
type="small-caps">x</hi><transChange type="added">is</transChange>` with the default renderer:

```html
<div class="topos-title topos-psalm" data-title-type="psalm">A Psalm</div><span class="topos-w" data-strong="H3068" data-lemma="strong:H3068" data-morph="x">LORD</span><span class="topos-note topos-crossref" data-note-type="crossReference" data-n="a">Ps 1:1</span><span class="topos-note topos-explanation" data-note-type="explanation">Or, Jehovah</span> <span class="topos-hi topos-small-caps">x</span><span class="topos-added">is</span>
```

USX verse markers: `<verse number="16" style="v" sid="JHN 3:16"/>` renders
`<span class="topos-verse" data-osis-id="JHN 3:16" data-n="16"></span>`; the `<verse eid>` after
it renders nothing; the `<book>` line (Metadata) is not rendered with the default filter.
