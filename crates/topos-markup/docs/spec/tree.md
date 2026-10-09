# The tree

## Types

- `Node` is either `Text(String)` or `Element(Element)`.
- `Element` has public fields `kind`, `name`, `attrs`, `children`, `milestone`, `unclosed`
  (api.md). Trees can be built by hand with struct literals; every function must accept any such
  tree, including ones the parser would never make (an element with both children and a
  milestone, two adjacent text nodes, empty text nodes, empty names).
- `Kind` is what an element means, the same in every format. It is ordered (`PartialOrd`/`Ord`)
  in declaration order (`Word` < `Note` < … < `Other`), which is the order of `BTreeMap<Kind, _>`
  keys such as `TextOptions::templates`.
- `Milestone::Start` marks an element that starts here and continues past it (an `sID`
  milestone); `Milestone::End` marks the end of one (an `eID` milestone, or an orphan end tag).

`Node::as_element` and `Node::as_text` return the inside of the matching variant, else `None`.

## Element accessors

| Accessor | Result |
|---|---|
| `attr(name)` | The value of the first attribute whose key equals `name` exactly; if there is none, of the first whose key equals `name` ignoring ASCII case; else `None` |
| `text()` | The concatenation of every text node below the element, in document order, whatever their kind (notes and metadata included). Nothing is added between nodes |
| `strongs()` | Strong's numbers from `lemma` then from `strong` (strongs.md) |
| `morph()` | The `morph` attribute split on whitespace (empty if absent); scheme prefixes such as `robinson:` are kept |
| `note_type()` | `attr("type")` |
| `is_cross_reference()` | `kind == Note` and `note_type()` equals `crossReference` ignoring ASCII case |
| `title_type()` | `attr("type")` |
| `reference_target()` | The first present of `attr("osisRef")`, `attr("passage")`, `attr("target")`, `attr("loc")`, `attr("link-href")`, `attr("href")` |
| `style()` | `attr("type")`, else `attr("rend")` |
| `descendants()` | The element itself, then every element below it, in document order (text nodes are skipped) |

None of these look at `kind` except `is_cross_reference`. Because `attr` falls back to a
case-insensitive match, USX's lowercase `sid` and `eid` answer to `attr("sID")` and `attr("eID")`.

Free functions:

- `walk(nodes, f)` calls `f` on every element in the list and below, in document order.
- `find_all(nodes, kind)` is every element of that kind, in document order.
- `strongs_numbers(nodes)` is `strongs()` of every `Word` element, concatenated in document order
  (duplicates kept; nested words each contribute).

`Kind::class_name` (also the HTML class, html.md):

| Kind | Class name | Kind | Class name |
|---|---|---|---|
| Word | `w` | Paragraph | `p` |
| Note | `note` | LineGroup | `lg` |
| Reference | `ref` | Line | `l` |
| Title | `title` | LineBreak | `lb` |
| WordsOfJesus | `woj` | Division | `div` |
| Quote | `quote` | Chapter | `chapter` |
| Added | `added` | Verse | `verse` |
| DivineName | `divine-name` | Milestone | `milestone` |
| Highlight | `hi` | Image | `image` |
| Foreign | `foreign` | Entry | `entry` |
| | | Metadata | `metadata` |
| | | Other | `other` |

`Markup::from_name` maps `osis`, `thml`, `gbf`, `tei`, `usx`, `plaintext`, and `plain`, ignoring
ASCII case, to their variants; anything else (including surrounding whitespace) is `None`.

## Classification

Classification gives a complete element its kind. Two helper rules recur:

- **Milestone rule** (OSIS, ThML, TEI, USX; not GBF): if the element's `milestone` is not already
  set (orphan ends already have `End`), it becomes `Start` when `attr("sID")` exists, else `End`
  when `attr("eID")` exists. The lookup is `attr`'s, so `sid`, `SID`, and `eid` count too, and an
  element with both is `Start`. This applies to any element, whatever its name, and to elements
  with children.
- **Default style**: "sets `type` to X" means: if `attr("type")` is absent, an attribute
  `("type", X)` is added at the end of `attrs`; an existing `type` (in any case) is left alone.

Unless a row says otherwise, the element keeps its name and attributes unchanged.

### OSIS

Names are compared exactly (case-sensitive), following the OSIS 2.1.1 manual's element names.

| Name | Kind |
|---|---|
| `w` | Word |
| `note` | Note |
| `reference` | Reference |
| `title` | Title |
| `q` | WordsOfJesus when `who` equals `jesus` ignoring ASCII case; else Quote |
| `transChange` | Added |
| `divineName` | DivineName |
| `hi` | Highlight |
| `foreign` | Foreign |
| `p` | Paragraph |
| `div` | Paragraph when `type` is exactly `paragraph` or `x-p`; else Division |
| `lg` | LineGroup |
| `l` | Line |
| `lb` | LineBreak |
| `chapter` | Chapter |
| `verse` | Verse |
| `milestone` | Milestone |
| `figure` | Image |
| anything else (`seg`, `inscription`, `W`, `osis:w`, …) | Other |

The `type` value of `div` is compared exactly (via `attr("type")`, so the key may be in any case).
OSIS sets no default styles: a `hi` without `type` has no style.

Orphan ends have no attributes, so an orphan `</q>` is always Quote, never WordsOfJesus.

### ThML

Names are compared ignoring ASCII case (ThML 1.04 elements, which are largely HTML's).

| Name | Kind | Default style |
|---|---|---|
| `scripRef` | Reference | |
| `a` | Reference | |
| `note` | Note | |
| `i`, `em`, `cite` | Highlight | `italic` |
| `b`, `strong` | Highlight | `bold` |
| `u` | Highlight | `underline` |
| `sup` | Highlight | `super` |
| `sub` | Highlight | `sub` |
| `font`, `span` | Highlight | |
| `br` | LineBreak | |
| `p` | Paragraph | |
| `h1` … `h6` | Title | |
| `div` | Title when its `class` contains the substring `sechead` (case-sensitive); else Division | |
| `added` | Added | |
| `foreign` | Foreign | |
| `q`, `blockquote` | Quote | |
| `img` | Image | |
| `w` | Word (also the synthesised words of `<sync>`, parsing.md) | |
| anything else (`hr`, `pb`, `sync`, …) | Other | |

### TEI

Names are compared exactly (TEI P5, chapter 9, Dictionaries).

| Name | Kind | Default style |
|---|---|---|
| `entryFree`, `entry`, `superentry` | Entry | |
| `ref`, `xr` | Reference | |
| `hi` | Highlight | |
| `emph` | Highlight | `italic` |
| `title` | Highlight | `italic` |
| `lb` | LineBreak | |
| `p` | Paragraph | |
| `note` | Note | |
| `foreign` | Foreign | |
| `q`, `quote` | Quote | |
| `w` | Word | |
| `figure`, `graphic` | Image | |
| anything else (`orth`, `pron`, `def`, `sense`, `etym`, `superEntry`, …) | Other | |

Note `superentry` is lowercase; TEI's `superEntry` is Other. After the table, a Highlight with no
`type` (in any case) gets `type` = the value of `attr("rend")`, when there is one: `<hi
rend="italic">` gets `type="italic"`. (So `emph` and `title` get `italic` from the table first, and
their `rend` is not copied.)

### USX

Element names are compared exactly. `para` and `char` say what they are with `style`, a USFM
marker (USX 3.0; USFM 3.0 marker reference). The **marker** is the `style` value (`attr("style")`,
empty if absent) with any trailing ASCII digits removed: `q1` → `q`, `s2` → `s`, `mt1` → `mt`,
`toc3` → `toc`, `wj1` → `wj`. Style comparisons are exact (case-sensitive).

| Element | Condition | Kind | Default style |
|---|---|---|---|
| `book` | | Metadata | |
| `chapter` | | Chapter | |
| `verse` | | Verse | |
| `note` | | Note | `crossReference` when `style` is exactly `x` or `ex`; `study` when exactly `ef`; otherwise (`f`, `fe`, no style, any other) `footnote` |
| `ref` | | Reference | |
| `figure` | | Image | |
| `optbreak` | | LineBreak | |
| `para` | marker `s`, `ms`, `mt`, `mte`, `mr`, `sr`, `r`, `d`, `sp`, `cl`, `cd`, `is`, `imt`, `sd` | Title | the full `style` (`s1`, `mt2`, …) |
| `para` | marker `q`, `qr`, `qc`, `qm`, `qa`, `qd` | Line | |
| `para` | marker `b` | LineBreak | |
| `para` | marker `rem`, `h`, `toc`, `toca`, `ide`, `sts`, `restore`, `cat`, `usfm` | Metadata | |
| `para` | anything else (`p`, `m`, `pi`, `li`, `nb`, empty, no style, …) | Paragraph | |
| `char` | marker `wj` | WordsOfJesus | |
| `char` | marker `add` | Added | |
| `char` | marker `nd` | DivineName | |
| `char` | marker `w` | Word | |
| `char` | marker `xt` | Reference | |
| `char` | marker `qt` | Quote | |
| `char` | marker `tl` | Foreign | |
| `char` | marker `it`, `em`, `bk` | Highlight | `italic` |
| `char` | marker `bd` | Highlight | `bold` |
| `char` | marker `bdit` | Highlight | `bold-italic` |
| `char` | marker `sc` | Highlight | `small-caps` |
| `char` | marker `sup` | Highlight | `super` |
| `char` | marker `no` | Highlight | `normal` |
| `char` | anything else (`fr`, `ft`, `fq`, `xo`, `k`, `pn`, `jmp`, …) | Other | |
| `table`, `sidebar`, `periph` | | Division | |
| `row` | | Paragraph | |
| anything else (`usx`, `cell`, `ms`, …) | | Other | |

The milestone rule applies first, so `<verse sid="…"/>` is Verse with `Start` and `<verse
eid="…"/>` is Verse with `End`; `<verse number="6"/>` without `sid` has no milestone.

### GBF

Names are the token codes (parsing.md, section 3). There is no milestone rule.

| Name | Kind | Default style |
|---|---|---|
| `FI` | Highlight | `italic` |
| `FB` | Highlight | `bold` |
| `FU` | Highlight | `underline` |
| `FS` | Highlight | `super` |
| `FV` | Highlight | `sub` |
| `FC` | Highlight | `small-caps` |
| `FN` | Highlight | |
| `FR` | WordsOfJesus | |
| `FO` | Quote | |
| `RF` | Note | |
| `RX` | Reference | |
| `TS`, `TT`, `TH` | Title | |
| `PP` | LineGroup | |
| `CM` | Paragraph | |
| `CL` | LineBreak | |
| `w` | Word (synthesised by Strong's and morphology tokens) | |
| anything else (`RB`, `CI`, `PI`, `Ge`, …) | Other | |

Orphan GBF ends get the same default style (`<Fi>` alone is `Highlight FI @end type="italic"`).

### Plain text

The only elements are the `br` line breaks, of kind LineBreak.

## Worked example (OSIS)

Input: `<div sID="gen4" type="x-p"/>And <w lemma="strong:H0430">God</w> said, <q who="Jesus"
sID="q1"/>Let there be light<note type="crossReference" n="a"><reference
osisRef="John.1.5">John 1:5</reference></note>`

```text
Paragraph div @start sID="gen4" type="x-p"
text "And "
Word w lemma="strong:H0430"
  text "God"
text " said, "
WordsOfJesus q @start who="Jesus" sID="q1"
text "Let there be light"
Note note type="crossReference" n="a"
  Reference reference osisRef="John.1.5"
    text "John 1:5"
```

(The textual form is golden-format.md's.)
