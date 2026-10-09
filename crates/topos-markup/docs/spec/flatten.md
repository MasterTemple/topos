# Flattening: `Flat`, `Flattener`, spans, and points

A `Flat` is the plain text of one or more entries (plain-text.md) together with:

- `spans`: where each kept element is in the text;
- `entries`: where each entry's text is;
- a private map from runs of the text to the text nodes they came from, read with `source`,
  `source_range`.

`Flattener::new(opts)` starts one; `push(nodes)` adds an entry and returns the flattener for
chaining; `finish()` gives the `Flat`. `flatten(nodes, opts)` is a flattener with one entry.

## Entries

- Entries are numbered from 0 in push order. The trees of different entries are separate: paths
  are relative to each entry's own node list.
- Between entries a space is requested (plain-text.md): `push` of every entry after the first
  starts with a space request, so the last word of one entry and the first of the next are
  separated by a space unless a break is pending.
- `entries[i]` is the range of entry `i`'s text: from the start of its first piece (after any
  separator) to the end of its last piece. An entry that writes no piece gets an empty range at
  the position where the next piece starts (after that piece's separator), or at the end of the
  text if no piece follows. `entries` has one range per pushed entry.

Example: pushing `In the beginning `, `<note>dropped</note>`, ` was <q who="Jesus">the Word`, and
`</q>` (OSIS, default options) gives the text `In the beginning was the Word` and entries
`0..16`, `17..17`, `17..29`, `29..29`: entry 1 is empty where entry 2 starts, entry 3 is empty at
the end.

A piece is counted in the entry being processed when it is written, with one exception: the
after-text written by `finish` for words of Jesus still open at the end extends the last entry.

## Spans

A span is `{ range, kind, entry, path }`: the byte range in `text`, the element's kind, the entry
it is in, and its path in that entry's tree.

Which elements get spans (only kept elements, see filters.md; unwrapped and dropped ones never
do):

- every kept element whose `milestone` is `None` and whose kind is not `LineBreak`, covering
  everything written while it was processed: its template text, label, contents, and Strong's
  list;
- every kept `Verse`, `Chapter`, or `Milestone` element with no children, whatever its
  `milestone` (verse end markers included), covering its label (an empty range where it is,
  without a label);
- words of Jesus: see below.

Elements with a milestone other than those markers (paragraph, line, and title milestones, quote
milestones) get no span. Notes without children get no span.

**Order.** A span is added to `spans` when its element has been processed, so an element's span
comes after the spans of the elements inside it (`Line` before its `LineGroup`), and empty markers
come where they are. Words-of-Jesus spans are added when the words of Jesus close.

**Trimming.** When the flat is finished, each span's range is narrowed to exclude leading and
trailing whitespace of the text it covers. A span covering only whitespace becomes the empty range
at its old end (its start moves past the whitespace, and its end is never before its start). A
paragraph's span does not include the newline before or after it.

### Words of Jesus

Words of Jesus written as one element (`<q who="Jesus">…</q>`, USX `wj`, GBF `<FR>…<Fr>`) get an
ordinary span, unless the element is `unclosed`. Words of Jesus can also run between milestones, or
from an unclosed element into later entries; then the flattener keeps them **open**:

- A kept `WordsOfJesus` **start milestone** opens them, when they are not already open (a second
  start while open is ignored). The open state remembers where the text was (the start of the
  span), the entry, the milestone's path, and its `sID` (if any). The template's before-text for
  `WordsOfJesus` is written.
- A kept `WordsOfJesus` element that is **unclosed** is processed with its before-text but not its
  after-text, and then opens them (replacing any open state) with the span starting where the
  element started, its path, and no id. It gets no span of its own at that point.
- A kept **end milestone** of kind `WordsOfJesus` or `Quote` (an `eID` milestone or an orphan end
  tag) closes them when they are open and it **matches**:
  - when the open state has an id and the end has an `eID`: they match when the two are equal;
  - when the open state has no id and the end has no `eID`: they match (an orphan `</q>` ends an
    unclosed `<q who="Jesus">`);
  - otherwise: they match only when the end's kind is `WordsOfJesus`.
  Closing writes the after-text and adds a `WordsOfJesus` span from the open start to the current
  end of the text, with the open entry and path. A non-matching end leaves them open.
- A **quote start milestone** (kind `Quote`) does nothing.
- At each `push` after the first, after the space request: if words of Jesus are open, the part in
  the previous entry is closed without the after-text (a span from the open start to the current
  end of text, in the previous entry, with its path) and they are opened again at the current end
  of the text, in the new entry, with the **empty path** (the new entry's top level) and the same
  id.
- At `finish`, open words of Jesus are closed with the after-text.

So words of Jesus that run across entries have one span per entry, and the template's opening and
closing text are written once, where they really begin and end.

Examples (OSIS, default options):

| Entries | Text | Words-of-Jesus spans (text) |
|---|---|---|
| `He said, <q who="Jesus" sID="q1"/>I am <q sID="q2"/>the way<q eID="q2"/>` and `and the life.<q eID="q1"/> Then` | `He said, I am the way and the life. Then` | `I am the way` (entry 0, path `/1`), `and the life.` (entry 1, empty path) |
| `He said, <q who="Jesus">Follow` and `me.</q> And they did.` | `He said, Follow me. And they did.` | `Follow`, `me.` |
| `a<q who="Jesus" sID="q1"/>b<q eID="q9"/>c<q eID="q1"/>d` | `abcd` | `bc` (`q9` does not match) |

With the template `“{text}”`: `He said, <q who="Jesus" sID="q1"/>Follow` then `me.<q eID="q1"/> And
they did.` is `He said, “Follow me.” And they did.`

## Sources

Each piece copied from a text node (a run of non-whitespace characters) is recorded with its
entry, the text node's path, and the byte offset in the text node where the run starts. Nothing
else (separators, template text, labels, brackets, Strong's lists) has a source.

- `source(byte)`: the recorded piece containing that byte of `text` gives
  `Point { entry, path, offset: run start + (byte − piece start) }`; `None` if no piece contains
  it. The byte need not be a character boundary (the offset is then inside a character too).
- `source_range(range)`: the pieces that overlap `range` (a piece overlaps when it ends after
  `range.start` and starts before `range.end`). If there are none, `None`. Otherwise the start
  point is the first such piece's point at `max(range.start, piece start)`, and the end point is
  the last such piece's point at `min(range.end, piece end)`, an exclusive offset in that text
  node. An empty range inside a piece overlaps it, and gives two equal points.

Example: OSIS `See <hi type="italic">John</hi>   3:16\n<note>n</note> now` flattens (default
options) to `See John 3:16 now`. `source_range(4..13)` (`John 3:16`) is `(entry 0, /1/0, offset 0)`
to `(entry 0, /2, offset 7)`: `John` is the text node in the `hi`, and `3:16` is in the top-level
text node `   3:16\n`, where it ends at byte 7. `source(8)` (the space after `John`) is `None`.

## Lookups

- `spans_at(range)`: the spans whose range contains `range` (`span.start <= range.start` and
  `range.end <= span.end`), in `spans` order (so inner elements before outer ones, whatever the
  doc comment says).
- `last_before(kind, byte)`: of the spans of that kind whose start is at or before `byte`, the one
  with the greatest start; among equal starts, the one later in `spans`. Used to find the verse a
  search match is in: with USX verse milestones, the `Verse` span found for a match is a start or
  end marker at or before it.

## Guarantees

For any input:

- every span's range lies within `text` and on character boundaries, and `start <= end`;
- `entries.len()` is the number of pushes, each range within `text` on character boundaries;
- every `Point` from `source` or `source_range` has a path that resolves in that entry's tree to a
  text node, with an offset at most the node's length; when the bytes asked about are character
  boundaries of `text`, the offsets are character boundaries of the node's text.
