# Plain text and `TextOptions`

`plain_text(nodes, opts)` is the `text` of flattening the nodes as one entry
(`flatten(nodes, opts).text`, flatten.md). This file says how the text is written; flatten.md adds
the spans, entries, sources, and what carries from one entry to the next.

## Options

| Field | Default | Meaning |
|---|---|---|
| `filter` | `Filter::text()` | Which elements are kept, unwrapped, or dropped (filters.md) |
| `strongs` | `false` | Write each word's Strong's numbers after it |
| `line_breaks` | `true` | Breaks are newlines; when false every break is a space |
| `verse_label` | `None` | Template written where a verse marker is |
| `chapter_label` | `None` | Template written on its own line where a chapter marker is |
| `templates` | empty | Per kind, a template written around a kept element |

## Words and separators

The text is a sequence of **pieces** with **separators** between them.

Pieces are:

- each run of non-whitespace characters in a text node (a text node `  a  b ` gives pieces `a` and
  `b`);
- template text, labels, note brackets, and Strong's lists (below).

Separators are requested by whitespace and by elements, and are only written when a piece
follows:

- **Space**: requested by each whitespace character in a text node (before, between, or after
  words), and by some elements. A space request does nothing if a separator is already pending
  (a pending line break is never weakened to a space) or if the text is **glued** (below).
- **Break**: requested by elements (below). With `line_breaks` false, a break request is a space
  request instead. Otherwise breaks count up: the pending separator becomes one break if it was
  nothing or a space, or one more break if it was breaks.

When a piece is written:

- if nothing has been written yet, the pending separator is discarded (no leading whitespace);
- otherwise a pending space is written as ` `, one or two pending breaks as `\n`, and three or more
  as `\n\n` (at most one blank line).

The separator is then cleared. A separator still pending at the end is discarded (no trailing
whitespace). Pieces with nothing requested between them are written together: `Let there<FI>be<Fi>`
gives `Let therebe`.

So: runs of whitespace become one space; whitespace inside a text node never makes a newline
(`See John   3:16\n now` is `See John 3:16 now`); two block boundaries in a row (one element's
end and the next one's start) make one newline; three line breaks make one blank line.

## Elements

Each element is first given to the filter (filters.md). `Drop` skips it and its contents. `Unwrap`
processes its children only (none of the behaviour below for the element itself, no label, no
span). `Keep` processes it by kind as below. One exception comes before the filter: a chapter
marker that is not an `End` milestone updates the current book and chapter for labels (below)
even when it is dropped or unwrapped.

In this table "contents" means the element's children processed in order, and "templated
contents" means: the template's text before, the contents, the template's text after (see
Templates). "Block" means: a break before (only if the element has children, or is a `Start`
milestone), the contents, and a break after (only if it has children).

| Kind and shape | Written |
|---|---|
| `WordsOfJesus` or `Quote` with a `milestone` | nothing of its own; opens or closes words of Jesus (flatten.md). Its children, if any, are not written |
| `Verse`, `Chapter`, `Milestone` with no children | the label (verse and chapter only, below) |
| `Note` with no children | nothing |
| `Note` with children | a space, then templated contents (the default template for notes is `[{text}]`) |
| `Title` | a break, templated contents, a break |
| `LineBreak` | a break (its children, if any, are not written) |
| `Verse`, `Chapter` with children | the label, then the contents (no template) |
| `Paragraph`, `LineGroup`, `Entry` | block |
| `Line` | the contents, then a break unless it is a `Start` milestone |
| `Word` | the contents; then, when `strongs` is on and the word has Strong's numbers, a space and `[` + the numbers as displayed, joined by single spaces, + `]` (`beginning [H7225]`, `a [H7225 H853]`) |
| anything else (`Quote`, `Added`, `DivineName`, `Highlight`, `Foreign`, `Reference`, `Division`, `Image`, `Metadata`, `Other`, `WordsOfJesus` without a milestone, `Milestone` with children) | templated contents |

Consequences worth stating:

- With the default filter, notes, titles, and metadata are gone; everything else is its text.
- Verse and chapter markers write nothing without labels: USX `…world.<verse eid/><verse
  sid/>For…` stays `world.For` if the markup has no whitespace there.
- An empty `<p/>` (no children, no milestone) requests no breaks; a paragraph `Start` milestone
  (`<div type="x-p" sID="…"/>`) requests one break; a paragraph `End` milestone requests none.
- An `End` milestone `Line` (`<l eID="…"/>`) requests a break; a `Start` milestone line does not.
- `Word` elements ignore templates.

## Templates

`templates` maps a kind to a template string. A template is split at its first `{text}`: the part
before is written before the element's contents and the part after is written after them. A
template without `{text}` is all "before". Only the first `{text}` is special; later ones are
literal.

Templates apply to elements whose row above says "templated contents". For notes without a
template the template `[{text}]` is used. No other kind has a default.

Gluing:

- The before-text is written as a piece (with the pending separator before it), and then the text
  is glued: space requests are ignored until the next piece is written. Break requests are not
  affected (`“` + break + `para` is `“\npara`).
- The after-text is written immediately after the element's last piece, with no separator, and
  whatever separator was pending at that point is pending again after it.

So with `“{text}”` for words of Jesus, `said, <char style="wj"> I am the light</char>, and` is
written `said, “I am the light”, and`. An element with no contents still gets both parts (`“”`).
An empty before or after part writes nothing (and an empty before part does not glue).

A words-of-Jesus element that is unclosed (it continues into the next entry) gets its before-text
but not its after-text; the after-text is written where the words of Jesus finally end
(flatten.md). Words of Jesus between milestones get the before-text at the start milestone and the
after-text at the matching end.

## Labels

`verse_label` and `chapter_label` are templates with placeholders `{book}`, `{chapter}`, and
`{verse}`. Every occurrence of each placeholder is replaced (`{verse}` is always empty in a
chapter label).

A label is written for a kept `Verse` or `Chapter` element that is not an `End` milestone (both
the empty markers and the elements with children). If the label, after replacement, is empty or
only whitespace, nothing is written. A verse label is written as its own word: a space request,
the label as a piece, a space request. A chapter label is on its own line: a break request, the
label, a break request.

### The values

From the marker element:

- **Its id**: `attr("sID")`, else `attr("osisID")` (so USX `sid` counts, and an OSIS marker
  with both uses `sID`). The id is read in one of two forms:
  - USX form, when the id contains a space and the part after the first space contains no `.`:
    the book is the part before the first space and the rest is split at `:` into chapter and
    verse (`JHN 3:16` → `JHN`, `3`, `16`; `JHN 3` → `JHN`, `3`).
  - Otherwise OSIS form: the first whitespace-separated word, split at `.` into book, chapter, and
    verse (`John.3.16` → `John`, `3`, `16`; `Gen.1.2 Gen.1.3` → `Gen`, `1`, `2`; `gen2` →
    book `gen2` only).
  Empty parts count as absent.
- **Its number**: `attr("number")`, else `attr("n")`, as written (`16`, `4-5`, `2a`).

For a chapter marker: `{book}` is the id's book, else the current book; `{chapter}` is the
number, else the id's chapter. For a verse marker: `{book}` is the id's book, else the current
book; `{chapter}` is the id's chapter, else the current chapter; `{verse}` is the number, else the
id's verse. Absent values are empty.

The **current book and chapter** start unset for each `Flattener` and are updated by every
chapter marker that is not an `End` milestone, whether it is kept, unwrapped, or dropped: the
book becomes the id's book when the id has one (otherwise it stays); the chapter becomes the
number, else the id's chapter, else unset. They carry across entries.

Examples:

| Markup | Label template | Written |
|---|---|---|
| USX `<chapter number="3" sid="JHN 3"/>…<verse number="16" sid="JHN 3:16"/>For God…<verse number="17-18" sid="JHN 3:17-18"/>` | chapter `{book} {chapter}`, verse `[{verse}]` | `JHN 3\n[16] For God … [17-18] For God` |
| OSIS `<verse osisID="John.3.16">For God</verse> <verse osisID="John.3.17">sent</verse>` (no chapter marker) | verse `{book} {chapter}:{verse}` | `John 3:16 For God John 3:17 sent` |
| `<chapter osisID="Gen.1" sID="gen2"/>` (osis2mod's ids) | chapter `{book} {chapter}` | `gen2` (the `sID` is the id, and has no chapter) |
| `<verse/>` | verse `[{book} {chapter}:{verse}]` | `[ :]` (not only whitespace, so written) |

## Strong's numbers

With `strongs` on, see the `Word` row. The list is not from the tree (it has no source) and is
inside the word's span.

## Worked examples

OSIS `<w lemma="strong:H0430">God</w> <w lemma="strong:H0559" morph="x:1">said</w>, <q
who="Jesus" sID="q1"/>Let there be light<note type="crossReference" n="a"><reference
osisRef="John.1.5">John 1:5</reference></note>: &amp; <transChange type="added">it
was</transChange> <divineName>Lord</divineName>`, after a paragraph milestone and `And `:

- default: `And God said, Let there be light: & it was Lord`
- `Filter::text().keep(Kind::Note)`: `… Let there be light [John 1:5]: & it was Lord`

TEI `<entryFree n="00003"> <orth>Ἀβαδδών</orth> <pron rend="italic">ab-ad-dohn'</pron><lb/>
<def>of Hebrew origin (<ref target="StrongsHebrew:011">H11</ref>); a destroying angel</def>
</entryFree>`: `Ἀβαδδών ab-ad-dohn'\nof Hebrew origin (H11); a destroying angel`.

Plain text `one\r\ntwo  words\n\n\nthree`: `one\ntwo words\n\nthree`.

ThML sync example (parsing.md) with `line_breaks` false: `ιουδας δε εγεννησεν τον`.

USX with templates `## {text}` (Title), `“{text}”` (WordsOfJesus), `^({text})` (Note) and
`Filter::all().drop(Kind::Metadata)`: `<para style="s1">The Word</para><para style="p">Jesus said,
<char style="wj"> I am the light</char>, and<note style="f"><char style="ft">Or
bread</char></note> went.</para>` is `## The Word\nJesus said, “I am the light”, and ^(Or bread)
went.`; without templates the note is `[Or bread]`.
