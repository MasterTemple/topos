# topos-bible

Find, parse, and complete Bible references in text, files, and folders. Fast (written in Rust),
typed, and without dependencies.

```sh
pip install topos-bible
```

Wheels are published for Linux (x86_64 and aarch64), macOS (Apple Silicon and Intel), and
Windows, on Python 3.10 and later. The package is imported as `topos_bible`.

## Usage

```python
from topos_bible import BookStyle, OffsetUnit, Topos

topos = Topos()

for m in topos.search("Read Jn 3:16-18 and Rom 8:28", OffsetUnit.CHAR):
    print(m.passage.reference, m.passage.osis, m.start, m.end)
# John 3:16-18 John.3.16-John.3.18 5 15
# Romans 8:28 Rom.8.28 20 28

print(topos.parse("1 Cor 13:4-7", BookStyle.NAME).reference)
# 1 Corinthians 13:4-7

# Completions replace text[start:end] with completion.text
print([c.text for c in topos.complete("Gen 1:", 6, OffsetUnit.CHAR, BookStyle.NAME, 2)])
# ['Genesis 1:1', 'Genesis 1:2']
```

The examples below go on from this one; the comment after each `print` is what it prints. All
of them together are
[`examples/python/example.py`](https://github.com/MasterTemple/topos-bible/blob/main/crates/topos-ffi/examples/python/example.py).

`search` finds abbreviations (`Jn`, `1 Co`), ranges and lists (`5:1-3,5; 6:6`), `ff`, Roman
numerals (`Matth. x, 8`), and words split across lines, while skipping false positives like
`is 2.5%`.

## Finding references

`search(text, unit)` returns every reference as a `Match(passage, start, end, line, column)`.
Pass `OffsetUnit.CHAR` so offsets match Python string indices (`BYTE` and `UTF16` are also
there); `line` and `column` are 1-based.

```python
text = "Notes 📖: read Jn 3:16-18 and Rom 8:28."
for m in topos.search(text, OffsetUnit.CHAR):
    print(m.passage.reference, m.start, m.end, m.line, m.column, text[m.start : m.end])
# John 3:16-18 14 24 1 15 Jn 3:16-18
# Romans 8:28 29 37 1 30 Rom 8:28
```

`parts(text, unit)` splits each reference into the parts written between `;` and `,`, as
`MatchPart(passage, start, end, first, last, last_start)`: `first` and `last` are where the part
starts and ends, and `last_start` is where the end of a range is written. It is slower than
`search`, so use it on a line at a time, such as when a click on part of a reference opens it.

```python
for part in topos.parts("John 3:16; 4:5-7", OffsetUnit.CHAR):
    print(part.passage.reference, part.start, part.end, part.first.reference, part.last.reference, part.last_start)
# John 3:16 0 9 John 3:16 John 3:16 None
# John 4:5-7 11 16 John 4:5 John 4:7 15
```

## Parsing a reference

`parse(reference, style)` reads one reference the way search does, or returns `None` if it
doesn't exist. A `Passage` has `book_id` (1 for Genesis to 66 for Revelation), `book`,
`reference`, `segments`, and the OSIS and USFM forms:

```python
passage = topos.parse("John 3:16-18,20-4:2; 5", BookStyle.NAME)
print(passage.book_id, passage.book, "|", passage.reference, "|", passage.osis, "|", passage.usfm)
# 43 John | John 3:16-18,20-4:2; 5 | John.3.16-John.3.18 John.3.20-John.4.2 John.5 | JHN 3:16-18; JHN 3:20-4:2; JHN 5
```

`reference` is written in the `BookStyle` you ask for:

```python
for style in [BookStyle.NAME, BookStyle.ABBREVIATION, BookStyle.OSIS, BookStyle.USFM]:
    print(style.name, topos.parse("1 Cor 13:4", style).reference)
# NAME 1 Corinthians 13:4
# ABBREVIATION 1 Cor 13:4
# OSIS 1Cor 13:4
# USFM 1CO 13:4
```

OSIS and USFM references parse too. Single-chapter books' numbers are verses, and verses past a
chapter's end don't exist:

```python
print(topos.parse("John.3.16", BookStyle.NAME).reference, "|", topos.parse("JHN 3:16", BookStyle.NAME).reference)
# John 3:16 | John 3:16
print(topos.parse("Jude 5", BookStyle.NAME).reference, "|", topos.parse("John 3:99", BookStyle.NAME))
# Jude 1:5 | None
```

A book's name alone is the whole book, since nothing else is in the text. A single-chapter
book's whole book is written as its verses, since `Jude 1` is its first verse:

```python
print(topos.parse("Gen", BookStyle.NAME).reference, "|", topos.parse("Jude", BookStyle.NAME).reference)
# Genesis 1-50 | Jude 1:1-25
```

`passage.segments` lists each part of the reference as written. Each is either a
`PassageSegmentVerses(start, end)` with `ChapterVerse(chapter, verse)` ends, or a
`PassageSegmentChapters(start, end)` with chapter numbers; `end` is `None` for a single verse or
chapter:

```python
from topos_bible import PassageSegmentChapters, PassageSegmentVerses

for segment in passage.segments:
    match segment:
        case PassageSegmentVerses(start=start, end=end):
            print("verses", start.chapter, start.verse, end and (end.chapter, end.verse))
        case PassageSegmentChapters(start=start, end=end):
            print("chapters", start, end)
# verses 3 16 (3, 18)
# verses 3 20 (4, 2)
# chapters 5 None
```

## Verses

`verse_ranges(passage)` writes each segment as a `VerseRange(start, end)` with both ends, so whole
chapters run from verse 1 to their last verse; `verses(passage)` lists every verse.
`verse_count(book_id, chapter)` says how many verses a chapter has (`None` for one that isn't in the
data). `contains(outer, inner)` and `overlaps(a, b)` compare passages verse by verse.
`passage_from_verses(book_id, verses)` makes one passage of a list of verses (sorted, without
repeats, and with verses that follow each other as ranges), and `merge(passages)` joins passages
into one per book, in order: the union of their verses.

```python
john = topos.parse("John 3; 4:1-3", BookStyle.NAME)
print([(r.start.chapter, r.start.verse, r.end.chapter, r.end.verse) for r in topos.verse_ranges(john)])
# [(3, 1, 3, 36), (4, 1, 4, 3)]
print(len(topos.verses(john)), [(v.chapter, v.verse) for v in topos.verses(topos.parse("John 3:16-18", BookStyle.NAME))])
# 39 [(3, 16), (3, 17), (3, 18)]
print(topos.verse_count(43, 3), topos.verse_count(43, 22))
# 36 None

john3 = topos.parse("John 3", BookStyle.NAME)
print(topos.contains(john3, topos.parse("John 3:16", BookStyle.NAME)), topos.contains(john3, john))
# True False
print(topos.overlaps(john3, john), topos.overlaps(john3, topos.parse("John 4", BookStyle.NAME)))
# True False

from topos_bible import ChapterVerse

listed = [ChapterVerse(chapter=8, verse=verse) for verse in [32, 28, 29, 30, 29]]
print(topos.passage_from_verses(45, listed).reference)
# Romans 8:28-30,32
parts = [topos.parse(r, BookStyle.NAME) for r in ["Rom 8:30-32", "John 3:16", "Rom 8:28-29", "John 4", "John 3"]]
print([p.reference for p in topos.merge(parts)])
# ['John 3-4', 'Romans 8:28-32']
```

## Books and genres

`books()` lists every book as a `BookInfo(id, name, abbreviation, osis, usfm, chapters)`, and
`genres()` every group of books as a `GenreInfo(name, book_ids)` (`book_ids` is `bytes`).
`find_book(name)` and `find_genre(name)` look one up by name or abbreviation, ignoring case.

```python
books = topos.books()
print(len(books), books[42])
# 66 BookInfo(id=43, name='John', abbreviation='Jn', osis='John', usfm='JHN', chapters=21)
print([genre.name for genre in topos.genres()][:4])
# ['Pentateuch', 'History', 'OT Narrative', 'Wisdom']
print(topos.find_book("1 cor"), topos.find_book("Hezekiah"))
# 46 None
gospels = topos.find_genre("gospels")
print(gospels.name, list(gospels.book_ids))
# Gospels [40, 41, 42, 43]
```

## Completing

`complete(text, cursor, unit, style, limit)` completes the reference that ends at `cursor`
(`limit` 0 means no limit). Accept a `Completion(label, kind, start, end, text)` by replacing
`text[start:end]` with `text`; `kind` is a `CompletionKind` (`BOOK`, `CHAPTER`, `VERSE`, or
`TRANSLATION`).

```python
from topos_bible import CompletionKind

typed = "Jn 3:16 and Rom"
for c in topos.complete(typed, len(typed), OffsetUnit.CHAR, BookStyle.NAME, 3):
    print(c.kind.name, repr(c.label), c.start, c.end, repr(c.text), repr(typed[: c.start] + c.text + typed[c.end :]))
# BOOK 'Romans' 12 15 'Romans ' 'Jn 3:16 and Romans '
# CHAPTER 'Romans 1' 12 15 'Romans 1' 'Jn 3:16 and Romans 1'
# CHAPTER 'Romans 2' 12 15 'Romans 2' 'Jn 3:16 and Romans 2'

completions = topos.complete("see Gen 1:", 10, OffsetUnit.CHAR, BookStyle.NAME, 0)
print(len(completions), completions[0].text, completions[0].kind == CompletionKind.VERSE)
# 31 Genesis 1:1 True
```

## Queries

A `ToposQuery` chooses which references `search_with(text, unit, query)` keeps, like the CLI's
filter options. Each method returns a new query, so one can be reused and extended:

```python
from topos_bible import Testament, ToposQuery

text = "Gen 1:1; John 3; John 3:16 ESV; Rom 8:28; Rom 8:1-4; 1 Cor 13:4; Jude 25; Ps 23"


def found(query):
    return [m.passage.reference for m in topos.search_with(text, OffsetUnit.CHAR, query)]


every = ToposQuery.create()
print(found(every))
# ['Genesis 1:1', 'John 3', 'John 3:16', 'Romans 8:28', 'Romans 8:1-4', '1 Corinthians 13:4', 'Jude 1:25', 'Psalm 23']
```

Testaments (`testament(Testament.OLD)`, `old_testament()`, `new_testament()`) narrow the rest;
genres and books add up; exclusions always win:

```python
print(found(every.old_testament()), found(every.testament(Testament.OLD)))
# ['Genesis 1:1', 'Psalm 23'] ['Genesis 1:1', 'Psalm 23']
print(found(every.new_testament().exclude_testament(Testament.OLD).genre("Pauline Epistles")))
# ['Romans 8:28', 'Romans 8:1-4', '1 Corinthians 13:4']
print(found(every.book("Jn").book("Ps")), found(every.exclude_genre("pauline").exclude_book("Genesis")))
# ['John 3', 'John 3:16', 'Psalm 23'] ['John 3', 'John 3:16', 'Jude 1:25', 'Psalm 23']
```

Passages: `inside` keeps references entirely inside one, `any_overlap` those sharing a verse,
`explicit_overlap` those naming one of its verses (whole chapters don't count), and
`exact_overlap` those that are exactly it, however written. These are joined with OR;
`exclude_overlap` then drops references that share a verse:

```python
print(found(every.inside("Romans 8")), found(every.any_overlap("John 3:16")))
# ['Romans 8:28', 'Romans 8:1-4'] ['John 3', 'John 3:16']
print(found(every.explicit_overlap("John 3")), found(every.exact_overlap("Romans 8:1,2,3,4")))
# ['John 3:16'] ['Romans 8:1-4']
print(found(every.exclude_overlap("Romans 8").exclude_overlap("John")))
# ['Genesis 1:1', '1 Corinthians 13:4', 'Jude 1:25', 'Psalm 23']
```

`verses(single_chapter_books)` keeps references that name a verse, as the CLI does without
`--chapters` (`False` also drops single-chapter books' references like `Jude 25`), and
`translation(name)` keeps references followed by a translation's name:

```python
print(found(every.verses(True)))
# ['Genesis 1:1', 'John 3:16', 'Romans 8:28', 'Romans 8:1-4', '1 Corinthians 13:4', 'Jude 1:25']
print(found(every.verses(False)), found(every.translation("ESV")))
# ['Genesis 1:1', 'John 3:16', 'Romans 8:28', 'Romans 8:1-4', '1 Corinthians 13:4'] ['John 3:16']
```

`contradiction(query)` says why nothing can match, if the query's filters contradict each other:

```python
print(topos.contradiction(every.old_testament().genre("Pauline Epistles")), topos.contradiction(every))
# no included genre or book is in the included testaments None
```

`books_in(query)` lists the books its testaments, genres, and books keep (its passage filters
aside), by id (as `bytes`): every book for an empty query, and none when they contradict each
other. An unknown genre or book raises `ToposErrorInvalidQuery`:

```python
print(list(topos.books_in(every.genre("Gospels").exclude_book("John"))), len(topos.books_in(every.old_testament().genre("Pauline Epistles"))))
# [40, 41, 42] 0
```

## Formats

A `ToposFormat` says how references are written, like the CLI's `--psg-fmt` and `--fmt-*`
options; `format_passage(passage, format)` writes one. Chain the methods, or start from the CLI's
JSON with `with_json(json)`:

```python
from topos_bible import ToposFormat

passage = topos.parse("John 3:16,17,18; 4", BookStyle.NAME)
default = ToposFormat.create()
print(topos.format_passage(passage, default))
# John 3:16,17,18; 4
short = default.book(BookStyle.ABBREVIATION).join_adjacent(True).range("–")
print(topos.format_passage(passage, short))
# Jn 3:16–18; 4
dotted = default.book_separator(".").chapter_verse(".").verse_separator(", ").chapter_separator(" & ")
print(topos.format_passage(passage, dotted))
# John.3.16, 17, 18 & 4
print(topos.format_passage(passage, default.with_json('{"join_adjacent": true, "chapter_verse": "."}')))
# John 3.16-18; 4

print(topos.format_passage(topos.parse("John 1:1-2:3", BookStyle.NAME), default.omit_first_verse_of_chapter_range(True)))
# John 1-2:3
print(topos.format_passage(topos.parse("Jude 5", BookStyle.NAME), default.chapter_in_single_chapter_books(False)))
# Jude 5
```

| Method | CLI | Default |
|---|---|---|
| `book(style)` | `-f` | `BookStyle.NAME` |
| `book_separator(text)` | `--fmt-book-separator` | `" "` |
| `chapter_verse(text)` | `--fmt-chapter-verse` | `":"` |
| `range(text)` | `--fmt-range` | `"-"` |
| `verse_separator(text)` | `--fmt-verse-separator` | `","` |
| `chapter_separator(text)` | `--fmt-chapter-separator` | `"; "` |
| `join_adjacent(bool)` | `--fmt-join-adjacent` | `False` |
| `omit_first_verse_of_chapter_range(bool)` | `--fmt-omit-first-verse-of-chapter-range` | `False` |
| `chapter_in_single_chapter_books(bool)` | `--fmt-chapter-in-single-chapter-books` | `True` |

`complete_with(text, cursor, unit, format, limit)` completes like `complete`, written with a
format:

```python
typed = "see Jn 3:1"
print([c.text for c in topos.complete_with(typed, len(typed), OffsetUnit.CHAR, short, 3)])
# ['Jn 3:1', 'Jn 3:10', 'Jn 3:11']
```

## Options

A `ToposOptions` sets up an instance, like the CLI's data and context options. Chain the methods,
then `build()`. `merge_data(json)` adds names, books, genres, or chapter counts, and
`remove_data(json)` removes values, or a book or genre listed alone:

```python
from topos_bible import ToposOptions

custom = (
    ToposOptions.create()
    .merge_data('{"books":[{"book":"John","abbreviations":["jhn"]}]}')
    .remove_data('{"books":[{"book":"Jude"}]}')
    .build()
)
print([m.passage.reference for m in custom.search("Jhn 3:16 and Jude 5", OffsetUnit.CHAR)])
# ['John 3:16']
```

`context_book(name)` gives bare references like `3:16` a book, and `context_heading(pattern)`
lets headings set it for what follows:

```python
in_john = ToposOptions.create().context_book("John").build()
print([m.passage.reference for m in in_john.search("see 3:16 and 4:1", OffsetUnit.CHAR)])
# ['John 3:16', 'John 4:1']
headings = ToposOptions.create().context_heading("^#+ {book}$").build()
print([m.passage.reference for m in headings.search("# Romans\nsee 8:28\n# John\nand 3:16", OffsetUnit.CHAR)])
# ['Romans 8:28', 'John 3:16']
```

`format(format)` sets how `search` and `search_with` write references (`parse` and `complete`
use it too, with the book style they're given):

```python
abbreviated = ToposOptions.create().format(short).build()
print([m.passage.reference for m in abbreviated.search("John 3:16,17,18", OffsetUnit.CHAR)])
# ['Jn 3:16–18']
```

`books_alone(True)` makes searches find books named alone too, each as the whole book. It's
off by default, since in running text `Mark`, `Job`, and `Acts` are also words; turn it on for
text meant to be references, like a search box:

```python
alone = ToposOptions.create().books_alone(True).build()
print([m.passage.reference for m in alone.search("Romans and Jude, Titus 2:11", OffsetUnit.CHAR)])
# ['Romans 1-16', 'Jude 1:1-25', 'Titus 2:11']
```

`data(json)` replaces the default `books`, `genres`, or `chapter_verses` (each part the JSON
has; the same JSON as the CLI's `--data`). `Topos.with_config(json)` does the same:

```python
two_books = '{"books":[{"id":1,"book":"Genesis","abbreviation":"Gn","osis":"Gen","abbreviations":["gen"]},{"id":2,"book":"Exodus","abbreviation":"Ex","osis":"Exod","abbreviations":["exod"]}],"chapter_verses":{"Genesis":[31,25],"Exodus":[22]}}'
small = ToposOptions.create().data(two_books).build()
print([b.name for b in small.books()], [m.passage.reference for m in small.search("Gen 2:4, Exod 1:1, Exod 2:1, John 3:16", OffsetUnit.CHAR)])
# ['Genesis', 'Exodus'] ['Genesis 2:4', 'Exodus 1:1']
print(len(Topos.with_config(two_books).books()))
# 2
```

## Translations

A `ToposTranslations` holds the translations a reference can be followed by (`John 3:16 KJV`,
`(KJV)`, `, KJV`), like the Bibles an app has: `add(name, aliases)` adds one, and
`chapter_verses(name, book_id, counts)` gives a book its own verse counts in it. Names and aliases
match ignoring case; `names()` lists them and `get(name)` finds the name an alias belongs to.

```python
from topos_bible import ToposTranslations

translations = (
    ToposTranslations.create()
    .add("ESV", [])
    .add("KJV", ["AV"])
    .add("LXX", [])
    .chapter_verses("ESV", 64, [15])  # 3 John has 15 verses in the ESV
)
print(translations.names(), translations.get("av"), translations.get("NIV"))
# ['ESV', 'KJV', 'LXX'] KJV None
```

`parse_with_translation(reference, unit, translations, style)` is like `parse`, and returns a
`TranslatedPassage(passage, translation)` with the `Translation(name, start, end)` named after
it. A reference naming a translation with its own verse counts is checked against them, so
3 John 15 exists in the ESV but not in the KJV (which uses the default counts), and
`verse_count_in(book_id, chapter, translations, name)` counts a chapter's verses in one:

```python
named = topos.parse_with_translation("3 John 15 (esv)", OffsetUnit.CHAR, translations, BookStyle.NAME)
print(named.passage.reference, named.translation)
# 3 John 1:15 Translation(name='ESV', start=11, end=14)
print(topos.parse_with_translation("3 John 15 (av)", OffsetUnit.CHAR, translations, BookStyle.NAME))
# None
print(topos.verse_count(64, 1), topos.verse_count_in(64, 1, translations, "esv"), topos.verse_count_in(64, 1, translations, "AV"))
# 14 15 14
```

`search_with_translations(text, unit, query, translations)` returns each reference as a
`TranslatedMatch(reference, translation)`. A name ends a reference, so `Ps 22, LXX` is Psalm 22
in the LXX (not Psalms 22 and 70). With it, a query's `translation(name)` also matches aliases:

```python
text = "Ps 22, LXX; John 3:16 AV"
for m in topos.search_with_translations(text, OffsetUnit.CHAR, ToposQuery.create(), translations):
    print(m.reference.passage.reference, m.translation.name, text[m.translation.start : m.translation.end])
# Psalm 22 LXX LXX
# John 3:16 KJV AV
kjv = topos.search_with_translations(text, OffsetUnit.CHAR, ToposQuery.create().translation("KJV"), translations)
print([m.reference.passage.reference for m in kjv])
# ['John 3:16']
```

`complete_with_translations(text, cursor, unit, format, limit, translations)` is like
`complete_with`, and offers translations first after a reference:

```python
typed = "John 3:16 K"
for c in topos.complete_with_translations(typed, len(typed), OffsetUnit.CHAR, ToposFormat.create(), 1, translations):
    print(c.kind.name, c.text, c.start, c.end)
# TRANSLATION KJV 10 11
```

## Errors

Methods that can fail raise `ToposErrorException`, with the error in `error`:
`ToposErrorInvalidConfig` (data, options, or a format's JSON that can't be used),
`ToposErrorInvalidQuery` (an unknown book, genre, or passage, or a bad path pattern),
`ToposErrorInvalidIndex` (an index file that's damaged or from another version), or
`ToposErrorFiles` (a bad glob or ignore file), each with a `message`.

```python
from topos_bible import (
    ToposErrorException,
    ToposErrorFiles,
    ToposErrorInvalidConfig,
    ToposErrorInvalidIndex,
    ToposErrorInvalidQuery,
)

try:
    ToposOptions.create().merge_data('{"books":[{"book":"John","abbreviations":["gen"]}]}').build()
except ToposErrorException as error:
    print(isinstance(error.error, ToposErrorInvalidConfig), error.error.message)
# True invalid book data: `gen` would mean both Genesis and John

try:
    topos.search_with("John 3:16", OffsetUnit.CHAR, ToposQuery.create().book("Hezekiah"))
except ToposErrorException as error:
    print(isinstance(error.error, ToposErrorInvalidQuery), error.error.message)
# True unknown book "Hezekiah"

for attempt in [lambda: Topos.with_config("{"), lambda: ToposFormat.create().with_json('{"bold": true}')]:
    try:
        attempt()
    except ToposErrorException as error:
        print(type(error.error).__name__, error.error.message.split(",")[0])
# ToposErrorInvalidConfig EOF while parsing an object at line 1 column 1
# ToposErrorInvalidConfig unknown field `bold`
```

## Files and folders

`search_files(paths, files, query, unit)` searches files and folders like the CLI: every file
the options allow, by its format (EPUBs with CFIs, subtitles with times, PDFs in builds with
MuPDF, anything else as text), keeping the references the query keeps. Files come back sorted
by path as `FileMatches(path, matches, error)`. `cache_dir(folder)` keeps each file's results
until it changes, in a folder of your choice.

These examples search a folder `notes` with `sermon.md`, `talk.srt`, `book.epub`, `run.log`,
`data.bin` (binary), `old/draft.txt`, a hidden `.private.md`, `scratch.md` (in its
`.toposignore`), `huge.txt` (in an `.ignore` in the folder above), and a `.gitignore` of
`*.log`; beside `notes`, a file `more-ignores` lists `*.srt`.

```python
from topos_bible import ToposFiles

files = ToposFiles.create().cache_dir("cache")
for result in topos.search_files(["notes"], files, ToposQuery.create(), OffsetUnit.CHAR):
    print(result.path, result.error, [(m.passage.reference, m.line, m.column, m.label) for m in result.matches])
# notes/book.epub None [('John 3:16', 1, 5, 'epubcfi(/6/2!/4/2,/1:4,/1:11)'), ('Romans 8:28', 3, 5, 'epubcfi(/6/4!/4/2,/1:4,/1:15)')]
# notes/old/draft.txt None [('Genesis 1:1', 1, 1, None)]
# notes/run.log None [('Mark 1:1', 1, 1, None)]
# notes/sermon.md None [('Romans 8:28', 1, 7, None), ('Psalm 23:1', 2, 6, None)]
# notes/talk.srt None [('John 3:16', 3, 9, '00:00:01.000')]
```

Each `FileMatch(passage, start, end, line, column, label, epub)` has offsets in your unit (for
EPUBs, UTF-16 offsets through the book's text; none for PDFs), a `label` in the format's own
terms (a subtitle's time, a PDF's page, an EPUB's CFI), and for EPUBs a
`FileEpubPlace(spine_index, cfi, chapter, line_text)`. A file that can't be read has an `error`:

```python
sermon = topos.search_files(["notes/sermon.md"], files, ToposQuery.create().old_testament(), OffsetUnit.CHAR)[0]
match = sermon.matches[0]
print(match.passage.reference, match.start, match.end, match.line, match.column, match.label, match.epub)
# Psalm 23:1 20 27 2 6 None None
epub = topos.search_files(["notes/book.epub"], files, ToposQuery.create(), OffsetUnit.CHAR)[0].matches[0].epub
print(epub.spine_index, epub.cfi, epub.chapter, repr(epub.line_text))
# 0 epubcfi(/6/2!/4/2,/1:4,/1:11) None 'See Jn 3:16.'
print(topos.search_files(["missing.md"], files, ToposQuery.create(), OffsetUnit.CHAR))
# [FileMatches(path='missing.md', matches=[], error='No such file or directory (os error 2)')]
```

`list_files(paths, files)` lists the files `search_files` would search (the CLI's `--files`).
Like ripgrep, hidden files are skipped and `.gitignore`, `.ignore`, and `.toposignore` files are
respected, unless told otherwise; files named directly are always searched. `ToposFiles` has
the CLI's walking options, each returning new options:

| Method | CLI | |
|---|---|---|
| `hidden(bool)` | `--hidden` | Search hidden files and folders |
| `no_ignore(bool)` | `--no-ignore` | Don't respect ignore files |
| `no_ignore_vcs(bool)` | `--no-ignore-vcs` | Don't respect `.gitignore` and other version control ignore files |
| `no_ignore_parent(bool)` | `--no-ignore-parent` | Don't respect ignore files in parent folders |
| `no_require_git(bool)` | `--no-require-git` | Respect `.gitignore` outside git repositories too |
| `ignore_file(path)` | `--ignore-file` | Another ignore file, in `.gitignore`'s format |
| `follow_links(bool)` | `--follow` | Follow symbolic links |
| `max_depth(n)` | `--max-depth` | How deep to go into folders (0 is only the paths given) |
| `max_filesize(bytes)` | `--max-filesize` | Skip bigger files |
| `one_file_system(bool)` | `--one-file-system` | Don't cross into other file systems |
| `glob(glob)`, `iglob(glob)` | `--glob`, `--iglob` | Only files matching a glob (`!` excludes; `iglob` ignores case). As in ripgrep, globs override ignore files and hidden files |
| `path_regex(regex)` | `--path-regex` | Only files whose path matches a regular expression anywhere (`!` excludes) |
| `extension(ext)`, `exclude_extension(ext)` | `--ext`, `--exclude-ext` | Only files with one of these extensions, or without |
| `binary(bool)` | `--binary` | Search files that look binary as text |
| `cfi_assertions(bool)` | `--cfi-assertions` | Write `[id]` assertions in EPUB CFIs |
| `cache(bool)` | `--cache` | Keep each file's results until it changes, in the CLI's cache folder (`~/.cache/topos`) |
| `cache_dir(folder)` | | Keep the cache in this folder (turns the cache on) |

```python
every = ToposFiles.create()
print(topos.list_files(["notes"], every))
# ['notes/book.epub', 'notes/data.bin', 'notes/old/draft.txt', 'notes/run.log', 'notes/sermon.md', 'notes/talk.srt']
print(topos.list_files(["notes"], every.hidden(True).no_ignore(True)))
# ['notes/.gitignore', 'notes/.private.md', 'notes/.toposignore', 'notes/book.epub', 'notes/data.bin', 'notes/huge.txt', 'notes/old/draft.txt', 'notes/run.log', 'notes/scratch.md', 'notes/sermon.md', 'notes/talk.srt']
print(topos.list_files(["notes"], every.no_require_git(True)), topos.list_files(["notes"], every.no_require_git(True).no_ignore_vcs(True)) == topos.list_files(["notes"], every))
# ['notes/book.epub', 'notes/data.bin', 'notes/old/draft.txt', 'notes/sermon.md', 'notes/talk.srt'] True
print(topos.list_files(["notes"], every.no_ignore_parent(True).ignore_file("more-ignores")))
# ['notes/book.epub', 'notes/data.bin', 'notes/huge.txt', 'notes/old/draft.txt', 'notes/run.log', 'notes/sermon.md']
print(topos.list_files(["notes"], every.max_depth(1).max_filesize(100)))
# ['notes/data.bin', 'notes/run.log', 'notes/sermon.md', 'notes/talk.srt']
print(topos.list_files(["notes"], every.glob("*.md")), topos.list_files(["notes"], every.glob("*.md").glob("!.*")))
# ['notes/.private.md', 'notes/scratch.md', 'notes/sermon.md'] ['notes/scratch.md', 'notes/sermon.md']
print(topos.list_files(["notes"], every.glob("!*.md").glob("!*.txt")), topos.list_files(["notes"], every.iglob("*.EPUB")))
# ['notes/book.epub', 'notes/data.bin', 'notes/run.log', 'notes/talk.srt'] ['notes/book.epub']
print(topos.list_files(["notes"], every.path_regex("old/")), topos.list_files(["notes"], every.extension("md").extension("srt")))
# ['notes/old/draft.txt'] ['notes/sermon.md', 'notes/talk.srt']
print(topos.list_files(["notes"], every.exclude_extension("md").exclude_extension("srt").exclude_extension("epub")))
# ['notes/data.bin', 'notes/old/draft.txt', 'notes/run.log']
print(topos.list_files(["notes"], every.follow_links(True).one_file_system(True).cache(False)) == topos.list_files(["notes"], every))
# True

binary = topos.search_files(["notes/data.bin"], every.binary(True), ToposQuery.create(), OffsetUnit.CHAR)
print([m.passage.reference for m in binary[0].matches])
# ['John 3:16']
asserted = topos.search_files(["notes/book.epub"], every.cfi_assertions(True), ToposQuery.create(), OffsetUnit.CHAR)
print(asserted[0].matches[0].epub.cfi)
# epubcfi(/6/2[a]!/4/2[p],/1:4,/1:11)
```

A pattern that can't be read raises `ToposErrorFiles`:

```python
try:
    topos.list_files(["notes"], every.glob("["))
except ToposErrorException as error:
    print(isinstance(error.error, ToposErrorFiles), error.error.message)
# True --glob [: error parsing glob '[': unclosed character class; missing ']'
```

## An index of many files

A `ToposIndex` keeps the references of many files between runs, compactly (about 15 bytes each),
so only files that changed are searched again, and devices that sync the same files share it.
The app reads and writes the files; the index takes their text or bytes, sizes, and modification
times in milliseconds.

`ToposIndex.create(device, engine, unit)` makes an empty index for a device (`device` names its
packs; `engine`, a version, is written in them). EPUB and PDF entries count offsets in UTF-16,
so an index that holds them is created with `OffsetUnit.UTF16` (one with only text files can use
`CHAR`). `check(path, size, mtime)` says whether its entry for a file is current
(`IndexStatus.FRESH`), and `index_text(topos, path, size, mtime, text, written)` searches a text
file and keeps its references:

```python
import os
import pathlib
import time

from topos_bible import EpubReference, IndexKind, IndexOrder, IndexScope, IndexStatus, ToposIndex, ToposPaths


def stamp(path):
    """A file's size and modification time in milliseconds, as the index takes them"""
    stat = os.stat(path)
    return stat.st_size, stat.st_mtime_ns // 1_000_000


index = ToposIndex.create("laptop", "1.0", OffsetUnit.UTF16)
for path in ["notes/sermon.md", "notes/old/draft.txt"]:
    size, mtime = stamp(path)
    if index.check(path, size, mtime) != IndexStatus.FRESH:
        print(path, index.index_text(topos, path, size, mtime, pathlib.Path(path).read_text(encoding="utf-8"), time.time() * 1000))
# notes/sermon.md 2
# notes/old/draft.txt 1
print(index.check("notes/sermon.md", *stamp("notes/sermon.md")).name)
# FRESH
```

`Topos.index_entry(path, size, mtime, text, written, unit)` searches a text file into bytes
that `insert(bytes, path, size, mtime)` adds, so another thread or process can search while the
index stays where it's queried. `index_epub(path, size, mtime, bytes, written)` does the same for
an EPUB's bytes (its references get CFIs and the text around them), and `index_pdf` for a PDF's,
only in builds with MuPDF (the `pdf` feature; the published wheels don't have it):

```python
size, mtime = stamp("notes/talk.srt")
entry = topos.index_entry("notes/talk.srt", size, mtime, pathlib.Path("notes/talk.srt").read_text(encoding="utf-8"), time.time() * 1000, OffsetUnit.UTF16)
index.insert(entry, "notes/talk.srt", size, mtime)

size, mtime = stamp("notes/book.epub")
entry = topos.index_epub("notes/book.epub", size, mtime, pathlib.Path("notes/book.epub").read_bytes(), time.time() * 1000)
index.insert(entry, "notes/book.epub", size, mtime)

try:
    topos.index_pdf("notes/paper.pdf", 4, 0, b"%PDF", time.time() * 1000)
except ToposErrorException as error:
    print(isinstance(error.error, ToposErrorInvalidIndex), error.error.message)
# True PDFs need topos built with the pdf feature (MuPDF), natively
```

`set_epub(path, size, mtime, written, references)` keeps an EPUB's references found elsewhere
(by a reader that has the book open), each an `EpubReference`. `paths()`, `file_count()`, and
`reference_count()` say what the index has:

```python
john = topos.parse("John 1:1", BookStyle.NAME)
reference = EpubReference(passage=john, start=0, end=8, line=1, column=1, spine_index=0, chapter="Prologue", cfi="epubcfi(/6/2!/4/2,/1:0,/1:8)", line_text="John 1:1 says")
index.set_epub("notes/other.epub", 2048, 0, time.time() * 1000, [reference])

print(index.paths(), index.file_count(), index.reference_count())
# ['notes/sermon.md', 'notes/old/draft.txt', 'notes/talk.srt', 'notes/book.epub', 'notes/other.epub'] 5 7
```

`query(topos, query, scope, path, order)` keeps the references a `ToposQuery` keeps, in every
file (`IndexScope.ALL`), one file (`FILE`), or a folder (`FOLDER`), sorted by file
(`IndexOrder.FILE`) or by where passages start in the Bible (`BIBLE`). Its `IndexResults` has
`len()`, `is_empty()`, and `file_count()`, and `page(topos, results, offset, limit)` reads them
as `IndexHit(path, passage, start, end, line, column, line_text, epub, pdf)`; EPUBs' hits have an
`IndexEpub(spine_index, cfi, chapter)`, and PDFs' an `IndexPdf(page, locator)`.

```python
results = index.query(topos, ToposQuery.create().new_testament(), IndexScope.ALL, "", IndexOrder.BIBLE)
print(results.len(), results.is_empty(), results.file_count())
# 5 False 4
for hit in index.page(topos, results, 0, 3):
    print(hit.path, hit.passage.reference, hit.start, hit.end, hit.line, hit.column, repr(hit.line_text), hit.epub, hit.pdf)
# notes/other.epub John 1:1 0 8 1 1 'John 1:1 says' IndexEpub(spine_index=0, cfi='epubcfi(/6/2!/4/2,/1:0,/1:8)', chapter='Prologue') None
# notes/book.epub John 3:16 4 11 1 5 'See Jn 3:16.' IndexEpub(spine_index=0, cfi='epubcfi(/6/2!/4/2,/1:4,/1:11)', chapter=None) None
# notes/talk.srt John 3:16 40 49 3 9 '' None None
```

`query_in(topos, query, scope, path, extensions, order)` keeps only files with those
extensions, `extension_counts(results)` counts references and files by extension, and
`file_hits(topos, path)` gives one file's references:

```python
in_folder = index.query(topos, ToposQuery.create(), IndexScope.FOLDER, "notes/old", IndexOrder.FILE)
in_file = index.query(topos, ToposQuery.create(), IndexScope.FILE, "notes/sermon.md", IndexOrder.FILE)
print([h.passage.reference for h in index.page(topos, in_folder, 0, 10)], [h.passage.reference for h in index.page(topos, in_file, 0, 10)])
# ['Genesis 1:1'] ['Romans 8:28', 'Psalm 23:1']
markdown = index.query_in(topos, ToposQuery.create(), IndexScope.ALL, "", ["md", "txt"], IndexOrder.FILE)
print([h.passage.reference for h in index.page(topos, markdown, 0, 10)])
# ['Genesis 1:1', 'Romans 8:28', 'Psalm 23:1']
print([(c.extension, c.references, c.files) for c in index.extension_counts(results)])
# [('epub', 3, 2), ('md', 1, 1), ('srt', 1, 1)]
print([h.passage.reference for h in index.file_hits(topos, "notes/sermon.md")])
# ['Romans 8:28', 'Psalm 23:1']
```

`query_paths(topos, query, paths, order)` keeps the files a `ToposPaths` keeps: files and
folders (`path`), globs like ripgrep's (`glob`, and `iglob` ignoring case), regular expressions
matched anywhere in the path (`regex`), and extensions (`extension`, any case). A file is kept
when it's one of the paths (if any are given), matches an included glob or regular expression
(if any are), matches no excluded one (`!` excludes), and has one of the extensions (if any are).
`problems()` says why a pattern can't be read; queries raise `ToposErrorInvalidQuery` for it.

```python
paths = ToposPaths.create().path("notes/old").path("notes/sermon.md").glob("!*.txt")
print([h.path for h in index.page(topos, index.query_paths(topos, ToposQuery.create(), paths, IndexOrder.FILE), 0, 10)])
# ['notes/sermon.md', 'notes/sermon.md']
paths = ToposPaths.create().iglob("*.EPUB").regex("talk")
print(sorted({h.path for h in index.page(topos, index.query_paths(topos, ToposQuery.create(), paths, IndexOrder.FILE), 0, 10)}))
# ['notes/book.epub', 'notes/other.epub', 'notes/talk.srt']
paths = ToposPaths.create().extension("TXT")
print([h.path for h in index.page(topos, index.query_paths(topos, ToposQuery.create(), paths, IndexOrder.FILE), 0, 10)])
# ['notes/old/draft.txt']
print(ToposPaths.create().glob("[").regex("(").problems()[0])
# bad glob "[": unclosed character class; missing ']'
```

The index doesn't store translations, so its queries can't take `translation(name)`:

```python
try:
    index.query(topos, ToposQuery.create().translation("ESV"), IndexScope.ALL, "", IndexOrder.FILE)
except ToposErrorException as error:
    print(error.error.message)
# the index doesn't store translations, so it can't filter by them
```

**Saving.** `is_dirty()` says whether this device has packs to write; `take_dirty_packs()` and
`take_unsaved_details()` give them as `IndexFile(name, bytes)` (a pack is
`<device>-<kind>-<number>.bin`, an EPUB's or PDF's details `<name>.bin`). Each kind of file
(`IndexKind.TEXT`, `EPUB`, `PDF`) has packs of its own: `pack_kind(name)` says which a pack is
for (`None` for another file), and `kind_of(path)` which a file is. `detail_names()` lists the
details entries use, so others can be deleted. `is_current(results)` says whether the index is
unchanged since a query.

```python
print(index.is_current(results), index.is_dirty())
# True True
pathlib.Path("index").mkdir()
for file in index.take_dirty_packs() + index.take_unsaved_details():
    pathlib.Path("index", file.name).write_bytes(file.bytes)
print([name for name in sorted(os.listdir("index")) if name.startswith("laptop-")], index.is_dirty())
# ['laptop-epub-10.bin', 'laptop-epub-29.bin', 'laptop-text-14.bin', 'laptop-text-16.bin', 'laptop-text-18.bin'] False
print(index.pack_kind("laptop-epub-10.bin").name, index.pack_kind("notes.bin"), index.kind_of("notes/talk.srt").name, len(index.detail_names()))
# EPUB None TEXT 2
```

**Loading.** `load_pack(bytes)` adds a pack, this device's from an earlier run or another
device's; load only the kinds the app searches. A loaded entry is used once `check` finds it
current. After syncing, a file's time can differ: `check` then says `UNCONFIRMED` (the entry is
used meanwhile), and `confirm(path, size, mtime, text)` checks it by the file's text (or, with
`None`, by its size alone, for files too big to read again).

```python
phone = ToposIndex.create("phone", "1.0", OffsetUnit.UTF16)
for name in sorted(os.listdir("index")):
    if phone.pack_kind(name) is not None:
        phone.load_pack(pathlib.Path("index", name).read_bytes())
for path in ["notes/sermon.md", "notes/old/draft.txt", "notes/talk.srt", "notes/book.epub"]:
    phone.check(path, *stamp(path))
print(sorted(phone.paths()), phone.reference_count())
# ['notes/book.epub', 'notes/old/draft.txt', 'notes/sermon.md', 'notes/talk.srt'] 6

size, mtime = stamp("notes/sermon.md")
print(phone.check("notes/sermon.md", size, mtime + 5000).name)
# UNCONFIRMED
print(phone.confirm("notes/sermon.md", size, mtime + 5000, pathlib.Path("notes/sermon.md").read_text(encoding="utf-8")))
# True
```

EPUBs' and PDFs' details (CFIs, locators, and the text around each reference) are read on
demand: `missing_details(results, offset, limit)` names those a page needs that aren't loaded,
`missing_file_detail(path)` those a file needs, and `load_detail(name, bytes)` loads one.

```python
everything = phone.query(topos, ToposQuery.create(), IndexScope.ALL, "", IndexOrder.FILE)
print(phone.missing_details(everything, 0, 10) == [phone.missing_file_detail("notes/book.epub")])
# True
for name in phone.missing_details(everything, 0, 10):
    phone.load_detail(name, pathlib.Path("index", name + ".bin").read_bytes())
print([h.line_text for h in phone.file_hits(topos, "notes/book.epub")], phone.missing_file_detail("notes/book.epub"))
# ['See Jn 3:16.', 'And Romans 8:28'] None
```

`rename(old, new)`, `remove(path)` (a file deleted, or no longer searched), and `retain(paths)`
(every file not in `paths` taken out) follow the files:

```python
phone.rename("notes/old/draft.txt", "notes/draft.txt")
print(phone.is_current(everything), sorted(phone.paths()))
# False ['notes/book.epub', 'notes/draft.txt', 'notes/sermon.md', 'notes/talk.srt']
phone.remove("notes/talk.srt")
phone.retain(["notes/sermon.md", "notes/draft.txt"])
print(sorted(phone.paths()))
# ['notes/draft.txt', 'notes/sermon.md']
```

Packs that are damaged, from another version, or counted in another unit raise
`ToposErrorInvalidIndex`:

```python
try:
    phone.load_pack(b"not a pack")
except ToposErrorException as error:
    print(isinstance(error.error, ToposErrorInvalidIndex), error.error.message)
# True not a topos index file
```

Source, the CLI, and other languages: [github.com/MasterTemple/topos-bible](https://github.com/MasterTemple/topos-bible).
Released under [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).
