# Parsing

`parse(markup, text) -> Vec<Node>` reads one entry. It never fails and never panics. This file
says how each markup's text becomes the tree; tree.md says what kind each element gets and which
attributes classification adds.

Four of the markups are XML-like (OSIS, ThML, TEI, USX) and share one reader with small
differences. GBF has its own token syntax. Plain text is split into lines.

## 1. Building the tree (all markups but plain text)

The readers produce a stream of events: text, start tag, empty-element tag, end tag. The tree is
built from them by these rules, which are the same for every markup.

### Text

- Text is added as a `Node::Text` to the innermost open element, or to the top level when nothing
  is open.
- Empty text adds nothing.
- Text that directly follows text in the same container is appended to it: a container never has
  two text nodes in a row. This holds across things that make no node, such as comments,
  processing instructions, ignored end tags (ThML void elements), and entity references, so
  `text<!-- c --> more` is one text node `text more`.

### Start tags

A start tag opens an element: its children are everything until its end. Its `name` is the tag
name exactly as written (case kept), its `attrs` are its attributes in the order written
(duplicates kept, values entity-decoded; see section 2.3), `milestone` is unset and `unclosed` is
false until it is closed.

### Empty-element tags

An empty-element tag (`<x/>`) adds an element with no children at the current position.

### End tags

An end tag closes the innermost open element whose name equals the end tag's name, ignoring ASCII
case (`<hi>…</HI>` closes). Elements opened inside it that are still open are closed at the same
point and marked `unclosed = true` (they did not end where they should). The element itself gets
`unclosed = false`.

When no open element has that name, the end tag becomes an **orphan end**: an element with the end
tag's name as written, no attributes, no children, and `milestone = Some(Milestone::End)`, added at
the current position. It marks the end of something that began in an earlier entry.

Example (OSIS): `<hi type="bold"><w lemma="strong:G1">x</hi></w>y`

```text
Highlight hi type="bold"
  Word w @unclosed lemma="strong:G1"
    text "x"
Word w @end
text "y"
```

### End of input

Every element still open at the end of the input is closed there, innermost first, and marked
`unclosed = true`.

Example (OSIS): `end of quote.</q> New <q who="Jesus">start` gives a text node, an orphan end
`q` (kind `Quote`, `@end`), a text node, and a `q` of kind `WordsOfJesus` with `unclosed = true`
whose text is `start`.

### Depth limit

At most **1,000 elements** are open at once, in every markup (GBF's paired codes included), so
crafted input can't nest deep enough to overflow the stack in `parse`, in the tree's derived
`Drop`, `Clone`, `PartialEq`, and `Debug`, or in the crate's traversals. A start tag met while
1,000 elements are open is added as an element with no children, marked `unclosed = true` (it
could not hold its contents); what follows it is added beside it, at the same depth. Its end tag
is then handled by the usual rule: it closes the innermost open element of that name, or becomes
an orphan end. Real markup nests a few levels deep, so the limit never changes a real text's tree.

Example: 100,000 nested `<q>` start tags followed by `x` give a chain of 1,000 open `q` elements
whose innermost holds 99,000 empty `q` elements (each `unclosed`) and then the text `x`, so the
tree is 1,001 levels deep counting those empty elements; parsing,
rendering, cloning, comparing, and dropping it don't overflow the stack.

### Classification

Every element gets its `kind` (and possibly `milestone` and added attributes) from its markup's
classification rules (tree.md) when it is complete: when it is closed, when it is an empty
element, or when it is an orphan end. An element's kind depends only on its name, its attributes,
and (for the `sID`/`eID` rule) whether it is already an orphan end.

## 2. The XML-like reader (OSIS, ThML, TEI, USX)

The input is read as XML, leniently: nothing about well-formedness is checked (end tag names need
not match, end tags may be unmatched, names and attributes are not validated). The lexical rules
below are what the golden files record; they are the behaviour of the `quick-xml` 0.42 reader with
end-name checking off, unmatched end tags allowed, empty elements reported as such (not expanded
to a start and an end), and text not trimmed, with attributes read without validity checks.

### 2.1 Markup constructs

A `<` starts markup. What follows decides which:

| Form | Result |
|---|---|
| `<!--` … `-->` | A comment: ignored. The `-->` must come after the `<!--` (so `<!-->` is not a complete comment). `--` inside is allowed. |
| `<![CDATA[` … `]]>` (uppercase `CDATA`) | Its content is text, exactly as written: no entity decoding |
| `<!DOCTYPE` … `>` (the word in any case) | Ignored (an internal subset in `[` … `]` may contain `>`) |
| `<?` … `?>` | A processing instruction or XML declaration: ignored |
| any other `<!` | Not readable (see 2.4) |
| `</` … `>` | An end tag |
| `<` … `/>` | An empty-element tag |
| `<` … `>` | A start tag |

A tag ends at the first `>` that is not inside a quoted attribute value (`"…"` or `'…'`).

**Tag names.** The name is everything after `<` (or `</`) up to the first whitespace character or
the end of the tag (for an empty-element tag, the final `/` is not part of the tag's content). A
name can therefore be empty or contain unusual characters:

| Input | Event |
|---|---|
| `<>` | start tag, name empty |
| `</>` | end tag, name empty |
| `< />` | empty-element tag, name empty |
| `<lb/ >` | start tag named `lb/` (the `/` is not last, so this is not an empty-element tag) |
| `a < b <hi>x</hi>` | text `a `, then a start tag with an empty name (its content is ` b <hi`), then text `x`, then `</hi>` (an orphan end, since no `hi` is open) |
| `<<hi>` | a start tag named `<hi` |
| `</q >` | end tag `q` (whitespace after the name is ignored) |
| `<osis:w>` | name `osis:w` (namespace prefixes are kept as part of the name) |

**Attributes.** After the name, attributes are `key="value"` or `key='value'`, with optional
whitespace around `=`, separated by whitespace or nothing (`b="x"c="y"` is two attributes). An
attribute that is not in that form (an unquoted value as in `b=c`, a key with no value as in
`<note placement>`, `b=="1"`, or `="1"` with no key) is left out, and reading goes on with the
next one: `<w lemma=strong:G1 morph="ok">` has only `morph="ok"`, `<a b c="1">` has only `c="1"`,
and a stray `=` between attributes is skipped (`<a b="1" = c="2">` has both). Duplicate
keys are all kept, in order. Keys are kept exactly as written (case, prefixes like `xml:lang`).
Values are entity-decoded with `decode_entities` (entities.md), never failing: `b="&"`,
`b="R&D"`, and `b="&bogus;"` are kept literally.

### 2.2 Text and entity references

Text between markup is entity-decoded with `decode_entities`. CDATA content is not decoded.

An `&` in text starts a reference that runs to the next `;`. If the input ends, or a `<` or a
second `&` comes, before that `;`, the text is not readable from there (see 2.4). Otherwise the
reference (whatever its content, `&a b;` included) is decoded by `decode_entities`, which keeps
unknown or invalid references literally. Examples:

| Text | Result |
|---|---|
| `a &amp; b` | `a & b` |
| `a &bogus; b <hi>x</hi>` | text `a &bogus; b `, then a `hi` element |
| `a &a b; <hi>x</hi>` | text `a &a b; `, then a `hi` element |
| `a &#x110000; <hi>x</hi>` | text `a &#x110000; `, then a `hi` element |
| `R&D <hi>x</hi>` | one text node `R&D <hi>x</hi>` (no `;` before `<`) |
| `a &amp b <hi>x</hi>` | one text node `a &amp b <hi>x</hi>` |
| `&amp;lt;` | `&lt;` (decoded once) |
| `x > y` | `x > y` (a lone `>` is text) |

The current crate panics on two forms of reference: one with an empty name (`&;`) and one whose
name starts with a non-ASCII character (`&é;`). That is a defect, not behaviour to keep: the
required result is that they are kept literally, as any other unknown reference (`a &; b` and
`a &é; b` are text). See entities.md and `tests/regressions.rs`.

### 2.3 Byte order mark

A U+FEFF byte order mark at the very start of the input is dropped. (Elsewhere it is ordinary
text.)

### 2.4 Unreadable input

When the reader meets something it cannot read, everything from there to the end of the input is
kept as text: it is entity-decoded with `decode_entities` and added like any other text (so it
joins a preceding text node). Elements open at that point stay open and are closed at the end as
usual (`unclosed = true`), so the trailing text lands inside them.

Things that are not readable:

- a tag with no closing `>` (`text <hi`, `x<hi type="bo`, `x</a`, `x<a/`);
- an unterminated quoted attribute value (`x<a b="1>`);
- an unterminated comment, CDATA section, processing instruction, or doctype (`x<!-- …`);
- `<!` followed by anything but `--`, `[CDATA[`, or `DOCTYPE` (`<!foo>`, `<![cdata[`);
- an `&` in text with no `;` before the next `<`, `&`, or the end (2.2).

"From there" means from the start of the text that contains the bad `&`, or from the `<` that
starts the bad markup. Since the text before the bad `&` would have been text anyway, the result
is the same as: everything from the end of the last complete tag, comment, or reference is text.

Examples (OSIS):

| Input | Tree |
|---|---|
| `x<!-- unclosed <hi>y</hi>` | `text "x<!-- unclosed <hi>y</hi>"` |
| `<p>q</p>R&D <hi>x</hi>` | `Paragraph p` (with `q`), then `text "R&D <hi>x</hi>"` |
| `<p>unclosed <b>bold`… (ThML) `… <<x>> & raw &amp; stray </i>` | see the `thml/malformed` golden: `<<x>` is a start tag named `<x`, and the bare `& ` makes the rest, `> & raw & stray </i>`, text inside it |

### 2.5 Format differences

The four XML-like markups differ only in:

- **Classification** (tree.md): OSIS, TEI, and USX compare element names exactly (case-sensitive);
  ThML compares them ignoring ASCII case.
- **ThML void elements and `<sync>`** (below).

Nothing else is format-specific: OSIS milestones, USX `sid`/`eid`, and so on are classification.

### 2.6 ThML void elements

ThML is HTML-like and writes some elements without a closing slash. These names, compared
ignoring ASCII case, are void:

`br`, `hr`, `img`, `sync`, `pb`, `meta`, `link`, `col`, `area`, `input`

(the HTML void elements as ThML uses them; ThML 1.04 DTD).

- A start tag with a void name is treated as an empty-element tag (`<br>` is the same as `<br/>`).
- An end tag with a void name is ignored entirely (no orphan end): `<br></br>` is one line break,
  and `a</br>b` is the text `ab`.

### 2.7 ThML `<sync>`: Strong's numbers and morphology after the word

ThML tags a word by putting `<sync>` right after it (ThML 1.04: `sync` with `type` `Strongs`,
`morph`, …, and `value`). A `<sync>` (start or empty-element tag; it is void) is read as follows,
with `type` compared ignoring ASCII case:

| `type` | `value` present | Effect |
|---|---|---|
| `Strongs` | yes | the previous word is tagged with `lemma` = `strong:` + value |
| `morph` | yes | the previous word is tagged with `morph` = value, or `class` lowercased + `:` + value when the sync has a `class` attribute (`class="Robinson" value="N-NSM"` gives `robinson:N-NSM`) |
| anything else, or no `value` | | an ordinary empty element named as written (`sync`, kind `Other`) |

"Tagging the previous word" (also used by GBF, section 3.3) works on the last node of the current
container (the innermost open element, or the top level):

1. If the last node is an element of kind `Word`, it is the word.
2. Otherwise a new element is made: name `w`, kind `Word`, no attributes, `milestone` unset,
   `unclosed` false. If the last node is a text node that does not end in whitespace, its last
   word (the text after its last whitespace character, or all of it if it has none) is moved out
   of that text node into the new element as its only child; if that empties the text node, the
   text node is removed. If the last node is text ending in whitespace, or an element of another
   kind, or there is no last node, the new `w` is empty. The new element is added at the end of
   the container.
3. The attribute is set on the word: if the word already has an attribute with that name (found
   as `Element::attr` finds it), the new value is appended after a space (`strong:G1080
   strong:G5656`); otherwise the attribute is added at the end of its attributes.

So consecutive syncs after one word all tag that word, and a `<sync>` after `<w>…</w>` tags that
`w`.

Example (ThML): `ιουδας<sync type="Strongs" value="G2455" /><sync type="morph" class="Robinson"
value="N-NSM" /> δε<sync type="Strongs" value="G1161" />`

```text
Word w lemma="strong:G2455" morph="robinson:N-NSM"
  text "ιουδας"
text " "
Word w lemma="strong:G1161"
  text "δε"
```

Example: `word <sync type="Strongs" value="G1"/>next` gives `text "word "`, an empty
`Word w lemma="strong:G1"`, and `text "next"`.

## 3. GBF

GBF (General Bible Format) is not XML. Its markup is tokens `<XY…>`: a two-letter code, usually
followed by a value. Paired codes open with two capitals (`<FR>`) and close with the second
letter in lowercase (`<Fr>`). Strong's numbers (`<WG25>`, `<WH430>`) and morphology
(`<WTV-PAI-3S>`) follow the word they tag.

### 3.1 Tokens and text

The input is scanned for `<`:

- Text before a `<` is entity-decoded (`decode_entities`) and added.
- If there is no `>` after the `<`, everything from the `<` to the end is entity-decoded and added
  as text, and reading stops.
- Otherwise the **body** is the text between the `<` and the first `>` after it. The body is a
  token when it is at least two bytes long, its first character is an ASCII capital, its second an
  ASCII letter, and it contains no whitespace and no `<`.
- A token is handled (3.2) and reading continues after its `>`.
- If the body is not a token, a literal `<` is added as text (not decoded), and reading continues
  right after that `<`. The rest is read again, so a later `<` inside it can start a token:
  `<<Ge>` is the text `<` and then the token `Ge`.
- After the last `<`, the rest is entity-decoded and added.

Not tokens: `<ab>`, `<A1>`, `<A>`, `<a>`, `<AB c>` (whitespace), `</FI>`, `<i>`, `<<Geschrieben
von Rom.>` (whitespace). They stay in the text as written (entities in them decoded).

### 3.2 Token meanings

In order:

1. **Strong's**: body starts `WG` or `WH` followed by an ASCII digit: tag the previous word
   (2.7) with `lemma` = `strong:` + the body without its leading `W` (`<WH0430>` gives
   `strong:H0430`; whatever follows the digits is kept, `<WG12x-y>` gives `strong:G12x-y`).
2. **Morphology**: body starts `WT` and is longer than two bytes: tag the previous word with
   `morph` = the body after `WT` (`<WTN-M>` gives `N-M`). `<WT>` alone is not morphology (rule 4).
3. **Paired codes**: the first two characters, uppercased, are one of
   `FI FB FU FS FV FC FN FR FO RF RX TS TT TH PP RB`.
   - If the second character is lowercase, it is an end tag for the uppercased code (closing the
     innermost open element of that name, or making an orphan end, as in section 1). Anything
     after the code is ignored.
   - Otherwise it is a start tag for an element named by the code, with one attribute `value` =
     the rest of the body when there is any (`<RXGen.1.1>` is `RX` with `value="Gen.1.1"`).
4. **Anything else**: an empty element named by the first two characters as written, with
   `value` = the rest of the body when there is any (`<CM>`, `<CL>`, `<PI2>` is `PI` with
   `value="2"`, `<WGabc>` is `WG` with `value="abc"`, `<Ge…>` is `Ge`).

Unclosed paired elements and orphan ends follow section 1: `<FR>never closed` is a `FR`
element with `unclosed = true`.

Examples:

| Input | Tree |
|---|---|
| `In the beginning<WH7225> God<WH430><WTN-M>` | `text "In the "`, `Word w lemma="strong:H7225"` (`beginning`), `text " "`, `Word w lemma="strong:H430" morph="N-M"` (`God`) |
| `two words<WH1><WH2a> more` | `text "two "`, `Word w lemma="strong:H1 strong:H2a"` (`words`), `text " more"` |
| `<WG25>start` | an empty `Word w lemma="strong:G25"`, `text "start"` |
| `<FR>a<FI>b<Fr>c<Fi>` | `WordsOfJesus FR` containing `a` and `Highlight FI @unclosed` (`b`), then `text "c"`, then an orphan `Highlight FI @end` |
| `<FIx>a<Fi>` | `Highlight FI value="x" type="italic"` containing `a` |

## 4. Plain text

The input is split at each `\n`. Each line loses one trailing `\r` if it has one. Non-empty lines
become text nodes; between consecutive lines there is an element named `br` of kind `LineBreak`
(no attributes, no children). Nothing else is interpreted: no entities, no markup.

| Input | Tree |
|---|---|
| `one\r\ntwo` | `text "one"`, `LineBreak br`, `text "two"` |
| `\n\na\n` | `LineBreak br`, `LineBreak br`, `text "a"`, `LineBreak br` |
| `a\rb` | `text "a\rb"` (a lone `\r` is not a line end) |
| (empty) | nothing |

## 5. What is not done

- No namespace processing, DTD processing, or entity definitions from a doctype.
- No whitespace normalisation in the tree: whitespace-only text between elements is kept as text
  nodes (`<usx>\n  <para>` has a text node `\n  `). Collapsing happens when text is written
  (plain-text.md).
- No validation of attribute values or nesting.
