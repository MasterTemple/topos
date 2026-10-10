# Verses in a book's tree

A whole book (a USX file, or an OSIS book) is one tree. Verses are not elements in it: USX marks
where each verse starts (`<verse sid>`) and ends (`<verse eid>`) (USX 3.0, "verse" milestones), and
OSIS can do the same with `sID`/`eID` milestones (OSIS 2.1.1 manual, milestoneable elements), or
wrap a verse's text in a `<verse>` element. A verse may start in one paragraph and end in the next.

## Places

A **place** is a `NodePath` used as a position in document order. Places compare as paths do
(node-paths.md): element by element, a prefix before anything longer. A place can be one past the
last child of an element (`[3]` when the top level has three nodes; `[0, 2]` when node 0 has two
children). "After a node" means the node's path with its last index plus one.

## `verses(nodes)`

### Markers

The markers are the elements of kind `Verse` or `Chapter`, in document order, found like this:

- Every node list is searched in order; an element of kind `Verse` or `Chapter` is a marker.
- The search does not go inside `Verse` elements (a verse element's own content is never searched
  for more markers). It does go inside `Chapter` elements that have children, and inside every
  other element.
- A marker with no children counts as a **milestone**: its `milestone`, or `Start` when it has
  none (a USX 2 `<verse number="1"/>` with no `sid` is a start). A marker with children counts as
  what its `milestone` says, usually `None`: an element that holds its text.

Each marker's **number** is read from its first present attribute of `number`, `n`; else from its
id: the last whitespace-separated word of `osisID`, else of `sID` (all lookups via `attr`), and the
part of that word after its last `.` or `:` (`John.3.16` → `16`, `JHN 3:16` → `16`, `Gen.1` → `1`,
`Gen.1.2 Gen.1.3` → `3`). Then:

- the text is split at the first `-`, `–` (U+2013), or `,` into a first part and the rest;
- each part's value is its leading ASCII digits after trimming whitespace, as a `u8`;
- if the first part has no digits or its value is over 255, the marker has no number;
- the first verse's **part** is the letter right after the first part's digits, when that is one
  ASCII letter not followed by another ASCII letter: `a` or `A` is 1, `b` or `B` is 2, … `z` or
  `Z` is 26 (a partial verse, as USFM's `\v 20a` and USX's `number="20a"` write it). Anything
  else (no letter, two letters, a space before the letter) is part 0, the whole verse;
- the last verse is the rest's value when it has one (else the first), and never less than the
  first. A letter after the rest's digits is not read: a bridge's last verse counts whole.

| Written | (first, part, last) |
|---|---|
| `16` | (16, 0, 16) |
| `4-5`, `4–5`, `4,5` | (4, 0, 5) |
| `2a` | (2, 1, 2) |
| `2B` | (2, 2, 2) |
| `2ab`, `2 a`, `2.a` | (2, 0, 2) |
| `20b-21` | (20, 2, 21) |
| `20-21a` | (20, 0, 21) |
| `20a-b` | (20, 1, 20) (the rest has no digits) |
| ` 9 ` | (9, 0, 9) |
| `5-3` | (5, 0, 5) |
| `7-x` | (7, 0, 7) |
| `256`, `x`, `a`, `` | none |
| `255` | (255, 0, 255) |
| `4-300` | (4, 0, 4) (the rest does not fit a `u8`) |

The id is read the same way, so `osisID="Gen.1.20a"` and `sID="GEN 1:20a"` give verse 20, part 1.
A bridge's part belongs to its first verse: `20b-21` is the second part of verse 20 through the
whole of verse 21.

### Building the spans

The markers are read in order with: the current chapter (0 at first), the place where the next
verse starts (none at first), and the verse that is open (none at first).

- **Chapter marker that is an `End` milestone**: ignored.
- **Any other chapter marker**: the open verse, if any, ends at this marker's path and is added.
  The chapter becomes the marker's first number, or the previous chapter plus one (saturating at
  255) if it has none. The next verse starts after this marker. When the marker is an element
  with children (an OSIS `<chapter osisID>` holding its verses), it becomes the **holding
  chapter**; any other chapter marker leaves no holding chapter.
- **Verse marker that is an `End` milestone**: if a verse is open, it ends at this marker's path,
  is added, and the next verse starts after this marker. If none is open, nothing happens.
- **Any other verse marker** (a start milestone, or an element):
  1. The open verse, if any, ends at this marker's path and is added. (The next verse's start is
     not changed by this.)
  2. If the marker has no number, nothing more happens (the remembered start is kept for the next
     verse).
  3. Otherwise a verse is made: chapter = the current chapter; verse, part, and last from the
     number;
     marker = its path; start = the remembered start, which is then forgotten, or the marker's own
     path when there is none; end = one past the last top-level node (`[nodes.len()]`). But when
     there is a holding chapter and this marker is inside it (the chapter's path is a proper
     prefix of the marker's), start is the chapter's first child (the chapter's path plus `0`),
     so the verse holds what comes before it in the chapter, as it would after a chapter
     milestone. Either way the holding chapter and the remembered start are forgotten.
  4. If the marker is an element with its text inside (its counted milestone is `None`), the
     verse ends after the marker, is added at once, and the next verse starts after it.
     Otherwise the verse is open.
- At the end, a verse still open is added with its end as it is (one past the last top-level
  node).

Verses are returned in the order they were added, which is document order.

Consequences:

- With end markers, a verse starts right after the previous verse's end marker, so a heading
  between two verses belongs to the second. After a chapter marker, the first verse starts right
  after the chapter marker, so the chapter's opening heading belongs to verse 1. In a chapter
  element holding its verses, the first verse starts at the chapter's first child, so the same
  holds. Text before the
  first chapter marker (a book title) belongs to no verse.
- Without end markers (USX 2, OSIS milestones without `eID`), each verse starts at its own marker
  and ends at the next verse's or chapter's marker, except that the first verse after a chapter
  marker starts right after that chapter marker.
- The parts of a verse (`2a`, `2b`) are listed as two verses with the same chapter and verse and
  parts 1 and 2, so (chapter, verse, part) tells them apart; the same chapter, verse, and part can
  still appear twice (a repeated marker), and both are listed.
- Chapter numbers are not checked; markers inside notes or other elements count.

### Example (USX)

```text
<usx version="3.0"><book code="PSA" style="id"/>
<para style="mt1">Psalms</para>
<chapter number="1" style="c" sid="PSA 1"/>
<para style="d">A heading</para>
<para style="q1"><verse number="1" style="v" sid="PSA 1:1"/>Blessed is the man</para>
<para style="q2">who does not walk<verse eid="PSA 1:1"/></para>
<para style="s1">The way</para>
<para style="q1"><verse number="2-3" style="v" sid="PSA 1:2-3"/>But his delight<verse eid="PSA 1:2-3"/><verse number="4" style="v" sid="PSA 1:4"/>Not so<verse eid="PSA 1:4"/></para>
<chapter eid="PSA 1"/>
<chapter number="2" style="c" sid="PSA 2"/>
<para style="q1"><verse number="1" style="v" sid="PSA 2:1"/>Why do the nations rage<verse eid="PSA 2:1"/></para>
<chapter eid="PSA 2"/></usx>
```

`verses` gives (chapter, verse, last) = (1, 1, 1), (1, 2, 3), (1, 4, 4), (2, 1, 1). Sliced and
written with `Filter::all()` (plain-text.md), they are `A heading\nBlessed is the man\nwho does not
walk` (two `Line` elements kept), `The way\nBut his delight`, `Not so`, and `Why do the nations
rage`; none contains `Psalms`. `chapter_verses` is `[4, 1]`.

### Example (OSIS elements)

`<div type="book" osisID="John"><chapter osisID="John.3"><title>Born again</title><verse
osisID="John.3.16">For God</verse><verse osisID="John.3.17">For God sent</verse></chapter></div>`
gives (3, 16) and (3, 17); sliced and written with `Filter::all()` they are `Born again
For God`
(the chapter's heading goes with its first verse) and `For God sent`. With milestones (`<chapter sID="John.3" osisID="John.3"/><verse
sID="John.3.16" osisID="John.3.16"/>For God<verse eID="John.3.16"/>…`) the same verses, and the
first sliced is `For God`.

## `slice(nodes, start, end)`

A copy of the part of the tree from place `start` up to, not including, place `end` (`None`: to
the end of the tree). For each node, at path `p`:

- if `end` is given and `p >= end`, this node and every later sibling are left out (an ancestor of
  `end` is before `end`, so it is not left out by this);
- if `p < start` and `p` is not a proper prefix of `start` (not an ancestor of the start place),
  the node is left out;
- if the node is an element and `p` is a proper prefix of `start` or of `end` (the cut goes
  through it), it is copied with the same name, attributes, kind, milestone, and `unclosed`, and
  with its children sliced by these same rules; if no children are left it is left out entirely;
- otherwise the node is copied whole (text nodes always whole).

So a paragraph cut in two is copied as a paragraph with only the part inside. `start` and `end`
can be any paths, in any relation; `start >= end` gives an empty result (except for ancestors of
`start` that contain nodes before `end`, which cannot happen when `start >= end`).

Slicing `[spans[i].start, spans[i].end)` from `verses` gives verse `i`'s content: its heading (if
it owns one), its text, and the elements around them, without its end marker and without the next
verse's start marker. Slicing from `spans[a].start` to `spans[b].end` gives verses `a` to `b`.

## `passage_ranges(spans, passage)`

For each part of `passage`, in the passage's order, as given by topos-bible's
`Passage::ranges(None)` (an inclusive range from (chapter, verse) to (chapter, verse); a whole
chapter ends at verse 255, `VerseRange::END_OF_CHAPTER`):

- a verse span is **in** the part when `(span.chapter, span.last) >= (start chapter, start verse)`
  and `(span.chapter, span.verse) <= (end chapter, end verse)` (tuples compared in order), so a
  bridged verse is in when any of its verses is;
- the result for the part is `first..last + 1`, where `first` and `last` are the indexes of the
  first and last spans in it; a part with none is left out.

A span's `part` is not compared: a reference names whole verses, so `Genesis 1:20` is in both
`20a` and `20b`, and `Genesis 1:21` is in `20b-21`.

Indexes, not places, are returned: use `spans[r.start].start` and `spans[r.end - 1].end`.

With the Psalm example above and topos's default data: `Psalm 1:3` → `[1..2]`; `Psalm 1:1-3` →
`[0..2]`; `Psalm 1` → `[0..3]`; `Psalm 1:4; 2` → `[2..3, 3..4]`; `Psalm 3` → `[]`. Slicing the
whole of `Psalm 1` (`spans[0].start` to `spans[2].end`) and writing it with `Filter::all()` gives
`A heading\nBlessed is the man\nwho does not walk\nThe way\nBut his delightNot so`.

## `chapter_verses(spans)`

The verse count of each chapter, as a `Vec<u8>` whose index `i` is chapter `i + 1`: for each span
with chapter at least 1, the count of its chapter is raised to `max(span.last, span.verse)`
(parts are not counted: `20a` and `20b` are verse 20). The
vector is as long as the highest chapter that has a span; chapters with no spans are 0. Spans in
chapter 0 are ignored. No spans → empty.

## `usx_chapter_verses(usx)`

The same counts read straight from USX text, without parsing (fast enough for a whole Bible):

- The text is cut at each `<`; for each piece after the first, the tag is the piece up to its
  first `>` (or all of it). Comments, CDATA, and quoting are not considered.
- The tag's name is its text up to the first whitespace or `/`. Only names exactly `chapter` and
  `verse` count (an end tag `</chapter>` has an empty name).
- The `number` attribute is found in the tag's text: an occurrence of `number` preceded by a
  whitespace character, followed by optional whitespace, `=`, optional whitespace, and a quote
  (`"` or `'`); the value runs to the next same quote or the end of the tag. Occurrences inside
  other names (`altnumber`, `pubnumber`) or values are skipped because of the whitespace rule. If an
  occurrence has `=` but no quote after it, the tag has no number. A tag without a number is
  skipped.
- The value's number is the part after its last `-` or `–` (a bridge counts to its last verse),
  trimmed, then its leading ASCII digits; no digits skips the tag. Values over 255 count as 255.
  (`4-5` → 5, `4a` → 4, `4,5` → 4.)
- A chapter tag sets the current chapter (0 at first; `number="0"` sets 0). A verse tag, when the
  current chapter is at least 1, raises that chapter's count to the verse's number, growing the
  vector as needed (chapters without verses are 0). Verse tags before any chapter, or in chapter
  0, are ignored.

It agrees with `chapter_verses(&verses(&parse(Markup::Usx, usx)))` on ordinary USX; they can
differ on odd numbers (`4,5`, values over 255, chapters without numbers, markers in comments).
