# Gaps

Questions the specification leaves open, and how each was settled. The implementer adds an entry
for every gap found while implementing (what was unclear, what was decided, and why: which golden
or standard settled it) before implementing it.

## Known when the specification was written

1. **Panics in the original.** `decode_entities` (and so `parse` for every markup but plain text)
   panics on a reference with an empty name (`&;`) or a name starting with a non-ASCII character
   (`&é;`). Settled: they are kept literally like any unknown reference (entities.md). Tested in
   `tests/regressions.rs`; these inputs are not in the goldens.
2. **Two doc comments disagree with behaviour.** `Flat::spans` says "in document order" and
   `Flat::spans_at` says "outermost first"; spans are actually listed when their element ends
   (inner before outer), and `spans_at` follows that order. Settled: keep the doc comments
   (api.md) and the behaviour (flatten.md, goldens); dependents rely on neither order beyond what
   the goldens show.
3. **XML lexical corners.** parsing.md lists the lexical rules that were observed. Inputs outside
   them (other malformed constructs) were not all probed. Settled in advance: the goldens are the
   reference; where they say nothing, behave as `quick-xml` 0.42's reader with end-name checking
   off, unmatched ends allowed, empty elements not expanded, text not trimmed, and attributes read
   without checks, as parsing.md describes. Never panic.
4. **`Filter`'s `Debug` format** is not specified beyond showing kinds, note types, and the number
   of rules.
5. **`usx_chapter_verses` and `chapter_verses` can disagree** on unusual numbers (`4,5`, values over
   255, chapters without numbers). Both behaviours are specified (verses.md) and both are
   recorded in the USX goldens; keep both.
6. **`HtmlRenderer`'s and `Flattener`'s handling of a second words-of-Jesus start milestone
   differ** (the renderer replaces the open id, the flattener ignores the second start). Both are
   specified; keep both.

## Found while implementing

Each entry says what was unclear, what was decided, and what settled it. "Unsettled by the
goldens" means the alternative was tried and every golden and test still passed, so the decision
rests on the prose and the standards alone.

7. **How the XML-like markups are lexed.** Gap 3 names quick-xml 0.42's reader as the reference
   where the prose and goldens are silent. Decided: use that reader itself, configured as
   parsing.md says (end-name checks off, unmatched ends allowed, empty elements not expanded, text
   not trimmed, comment checks off, dangling `&` an error), with attributes read with duplicate
   checks off and malformed ones skipped. On the first error, everything from the end of the last
   complete construct (tag, comment, CDATA section, processing instruction, doctype, or reference)
   is decoded and added as text, which is parsing.md 2.4's rule. Settled by gap 3 and every
   lexical golden passing.
8. **End-tag names with more after the name** (`</q x>`). parsing.md says a name runs to the
   first whitespace; quick-xml's end tag is everything but trailing whitespace (`q x`). Decided:
   the first whitespace (`q`), as the prose says. Unsettled by the goldens (`</q >` gives `q`
   either way).
9. **Which whitespace ends a tag name.** README.md's terms say whitespace is Unicode unless a rule
   says otherwise; parsing.md section 2 says the lexical rules are quick-xml's, which uses XML's
   `S` (space, tab, CR, LF; XML 1.0 production 3). Decided: XML's whitespace for tag names and
   between attributes, since that is the lexical layer parsing.md describes. Text pieces
   (plain-text.md) still split on Unicode whitespace.
10. **A second byte order mark.** Decided: only one U+FEFF at the very start is dropped (parsing.md
    2.3); a second one right after it is text.
11. **`<!DOCTYPE>` with no name.** quick-xml reports an error for it, at the `>`. Decided: it is
    unreadable from its `<` (parsing.md 2.4: "from the `<` that starts the bad markup"), so it and
    everything after it is text. Not in the goldens.
12. **Does a template's closing text end the glue of its opening text?** (`a<q who="Jesus"></q> b`
    with `“{text}”`.) plain-text.md lists template text among the pieces, and gluing lasts "until
    the next piece is written". Decided: yes, so that example is `a“” b`. Unsettled by the goldens.
13. **A second words-of-Jesus start milestone while they are open**: is the template's opening
    text written? Decided: no, the milestone is ignored entirely. Settled by
    `osis/woj-two-starts` (`text-all`).
14. **HTML: "a text node not inside a `WordsOfJesus` element"** when that element is unwrapped by
    the filter. Decided: the tree decides, so text inside an unwrapped words-of-Jesus element is
    not wrapped again by open words of Jesus. Unsettled by the goldens.
15. **OSIS-form label ids with more than three parts** (`a.b.c.d`). Decided: split into at most
    three, so the verse is `c.d`. Unsettled by the goldens.
16. **Label placeholders whose values contain placeholders** (a book id `{verse}`). Decided: one
    pass over the template, so values are never substituted again. Not in the goldens.
17. **Verse numbers from an `osisID` with no words.** Decided: when `osisID` is present it is the
    id (giving no number), and `sID` is not tried. Not in the goldens.
18. **ThML `<sync type="morph" class>`**: "lowercased" is read as Unicode lowercasing
    (`str::to_lowercase`); for ASCII classes, the only ones in the goldens, it is the same as
    ASCII lowercasing.
19. **Entry ranges and template closing text.** Decided: closing text counts as a piece of the
    entry being processed (flatten.md's exception for `finish` implies it), and an empty entry's
    range is placed where the next piece of any kind starts. Settled by `osis/test-entry-ranges`
    and the flatten tests, as far as they go.
20. **Very deep nesting.** Nothing bounded the tree's depth, and the tree's derived `Drop`,
    `Clone`, `PartialEq`, and `Debug`, like the crate's traversals, recurse: around 50,000 levels
    of nesting overflowed an 8 MB stack, which aborts the process. **The owner decided on a
    limit** (October 2026): at most 1,000 open elements while parsing (parsing.md, "Depth
    limit"). Trees built by hand aren't limited.
21. **`tests/depth.rs` disagrees with itself about the depth limit.** parsing.md's "Depth limit"
    allows 1,000 open elements, and a start tag met at the limit is added as an empty element
    inside the 1,000th; its example says the innermost open `q` holds the 99,000 cut elements.
    The implementation does exactly that. The test's `depth` helper also counts those cut
    elements, one level below the 1,000th open element, so it measures 1,001 where the test
    expects 1,000 (three assertions, one per test). Allowing only 999 open elements satisfies
    those assertions but breaks the same test's check that the 1,000th element in the chain holds
    99,001 nodes (it is then a cut, empty element). No limit satisfies every assertion. Decided:
    follow the prose and its example (1,000 open elements); the three `depth(&nodes) == LIMIT`
    assertions fail, and the test is left as handed over. Open: the specification side should
    say whether the limit counts the cut elements (999 open) or not (1,000 open), and correct
    either the prose or the test.
    **Resolved by the spec side:** the test was wrong. The limit is 1,000 open elements, and the
    tree is 1,001 levels deep counting the cut elements; the test now expects that.
