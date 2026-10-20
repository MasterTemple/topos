# topos-bible

Find, parse, and complete Bible references in text. Runs anywhere JavaScript does, via
WebAssembly, and ships with TypeScript types.

```sh
npm install topos-bible   # or: bun add topos-bible
```

## Usage

```ts
import {
  BookStyle, CompletionKind, IndexKind, IndexOrder, IndexScope, IndexStatus, OffsetUnit, Testament, Topos,
  ToposErrorException, ToposFiles, ToposFormat, ToposIndex, ToposOptions, ToposPaths, ToposQuery, ToposTranslations,
  type EpubReference, type IndexResults,
} from "topos-bible";

const topos = Topos.new();

for (const m of topos.search("Read Jn 3:16-18 and Rom 8:28", OffsetUnit.Utf16)) {
  console.log(m.passage.reference, m.passage.osis, m.start, m.end);
}
// John 3:16-18 John.3.16-John.3.18 5 15
// Romans 8:28 Rom.8.28 20 28

console.log(topos.parse("1 Cor 13:4-7", BookStyle.Name)?.reference);
// 1 Corinthians 13:4-7

// Completions replace text.slice(start, end) with completion.text
const input = "see Gen 1:";
const [first] = topos.complete(input, input.length, OffsetUnit.Utf16, BookStyle.Name, 10);
console.log(input.slice(0, first.start) + first.text + input.slice(first.end));
// see Genesis 1:1
```

The examples below go on from this one; the comment after each `console.log` is what it prints.
All of them together are
[`examples/node/index.ts`](https://github.com/MasterTemple/topos-bible/blob/main/crates/topos-ffi/examples/node/index.ts),
which is run before each release.

`search` finds abbreviations (`Jn`, `1 Co`), ranges and lists (`5:1-3,5; 6:6`), `ff`, Roman
numerals (`Matth. x, 8`), and words split across lines, while skipping false positives like
`is 2.5%`.

### Browsers and bundlers

In Node and Bun the WebAssembly module loads as the package is imported, so it is ready
right away. In browsers and with bundlers it is fetched asynchronously; wait for it before the
first call:

```ts
import { initialized } from "topos-bible";

await initialized; // before the first call
```

## Finding references

`search(text, unit)` returns every reference as a `Match { passage, start, end, line, column }`.
Pass `OffsetUnit.Utf16` so offsets match JavaScript string indices (`Byte` and `Char` are also
there); `line` and `column` are 1-based.

```ts
let text = "Notes 📖: read Jn 3:16-18 and Rom 8:28.";
for (const m of topos.search(text, OffsetUnit.Utf16)) {
  console.log(m.passage.reference, m.start, m.end, m.line, m.column, text.slice(m.start, m.end));
}
// John 3:16-18 15 25 1 16 Jn 3:16-18
// Romans 8:28 30 38 1 31 Rom 8:28
```

`parts(text, unit)` splits each reference into the parts written between `;` and `,`, as
`MatchPart { passage, start, end, first, last, lastStart }`: `first` and `last` are where the part
starts and ends, and `lastStart` is where the end of a range is written. It is slower than
`search`, so use it on a line at a time, such as when a tap on part of a reference opens it.

```ts
for (const part of topos.parts("John 3:16; 4:5-7", OffsetUnit.Utf16)) {
  console.log(part.passage.reference, part.start, part.end, part.first.reference, part.last.reference, part.lastStart);
}
// John 3:16 0 9 John 3:16 John 3:16 null
// John 4:5-7 11 16 John 4:5 John 4:7 15
```

## Parsing a reference

`parse(reference, style)` reads one reference the way search does, or returns `null` if it
doesn't exist. A `Passage` has `bookId` (1 for Genesis to 66 for Revelation), `book`,
`reference`, `segments`, and the OSIS and USFM forms:

```ts
let passage = topos.parse("John 3:16-18,20-4:2; 5", BookStyle.Name)!;
console.log(passage.bookId, passage.book, "|", passage.reference, "|", passage.osis, "|", passage.usfm);
// 43 John | John 3:16-18,20-4:2; 5 | John.3.16-John.3.18 John.3.20-John.4.2 John.5 | JHN 3:16-18; JHN 3:20-4:2; JHN 5
```

`reference` is written in the `BookStyle` you ask for:

```ts
for (const style of [BookStyle.Name, BookStyle.Abbreviation, BookStyle.Osis, BookStyle.Usfm]) {
  console.log(Object.keys(BookStyle)[style], topos.parse("1 Cor 13:4", style)?.reference);
}
// Name 1 Corinthians 13:4
// Abbreviation 1 Cor 13:4
// Osis 1Cor 13:4
// Usfm 1CO 13:4
```

OSIS and USFM references parse too. Single-chapter books' numbers are verses, and verses past a
chapter's end don't exist:

```ts
console.log(topos.parse("John.3.16", BookStyle.Name)?.reference, "|", topos.parse("JHN 3:16", BookStyle.Name)?.reference);
// John 3:16 | John 3:16
console.log(topos.parse("Jude 5", BookStyle.Name)?.reference, "|", topos.parse("John 3:99", BookStyle.Name));
// Jude 1:5 | null
```

A book's name alone is the whole book, since nothing else is in the text. A single-chapter
book's whole book is written as its verses, since `Jude 1` is its first verse:

```ts
console.log(topos.parse("Gen", BookStyle.Name)?.reference, "|", topos.parse("Jude", BookStyle.Name)?.reference);
// Genesis 1-50 | Jude 1:1-25
```

`passage.segments` lists each part of the reference as written. Each is either verses
(`tag: "Verses"`, with `ChapterVerse { chapter, verse }` ends) or whole chapters
(`tag: "Chapters"`, with chapter numbers), and `end` is `null` for a single verse or chapter:

```ts
for (const segment of passage.segments) {
  if (segment.tag === "Verses") {
    console.log("verses", segment.start.chapter, segment.start.verse, JSON.stringify(segment.end));
  } else {
    console.log("chapters", segment.start, segment.end);
  }
}
// verses 3 16 {"chapter":3,"verse":18}
// verses 3 20 {"chapter":4,"verse":2}
// chapters 5 null
```

## Verses

`verseRanges(passage)` writes each segment as a `VerseRange { start, end }` with both ends, so
whole chapters run from verse 1 to their last verse; `verses(passage)` lists every verse.
`contains(outer, inner)` and `overlaps(a, b)` compare passages verse by verse. `passageFromVerses(bookId, verses)` makes one passage of a
list of verses (sorted, without repeats, and with verses that follow each other as ranges), and
`merge(passages)` joins passages into one per book, in order: the union of their verses.

```ts
const john = topos.parse("John 3; 4:1-3", BookStyle.Name)!;
console.log(JSON.stringify(topos.verseRanges(john)));
// [{"start":{"chapter":3,"verse":1},"end":{"chapter":3,"verse":36}},{"start":{"chapter":4,"verse":1},"end":{"chapter":4,"verse":3}}]
console.log(topos.verses(john).length, JSON.stringify(topos.verses(topos.parse("John 3:16-18", BookStyle.Name)!)));
// 39 [{"chapter":3,"verse":16},{"chapter":3,"verse":17},{"chapter":3,"verse":18}]

const john3 = topos.parse("John 3", BookStyle.Name)!;
console.log(topos.contains(john3, topos.parse("John 3:16", BookStyle.Name)!), topos.contains(john3, john));
// true false
console.log(topos.overlaps(john3, john), topos.overlaps(john3, topos.parse("John 4", BookStyle.Name)!));
// true false

const listed = [32, 28, 29, 30, 29].map((verse) => ({ chapter: 8, verse }));
console.log(topos.passageFromVerses(45, listed)?.reference);
// Romans 8:28-30,32
const parts = ["Rom 8:30-32", "John 3:16", "Rom 8:28-29", "John 4", "John 3"].map((r) => topos.parse(r, BookStyle.Name)!);
console.log(topos.merge(parts).map((p) => p.reference).join(" | "));
// John 3-4 | Romans 8:28-32
```

## Books and genres

`books()` lists every book as a `BookInfo { id, name, abbreviation, osis, usfm, chapters }`, and
`genres()` every group of books as a `GenreInfo { name, bookIds }`. `findBook(name)` and
`findGenre(name)` look one up by name or abbreviation, ignoring case.

```ts
const books = topos.books();
console.log(books.length, JSON.stringify(books[42]));
// 66 {"id":43,"name":"John","abbreviation":"Jn","osis":"John","usfm":"JHN","chapters":21}
console.log(JSON.stringify(topos.genres().slice(0, 4).map((genre) => genre.name)));
// ["Pentateuch","History","OT Narrative","Wisdom"]
console.log(topos.findBook("1 cor"), topos.findBook("Hezekiah"));
// 46 null
const gospels = topos.findGenre("gospels")!;
console.log(gospels.name, gospels.bookIds.join(", "));
// Gospels 40, 41, 42, 43
```

## Completing

`complete(text, cursor, unit, style, limit)` completes the reference that ends at `cursor`
(`limit` 0 means no limit). Accept a `Completion { label, kind, start, end, text }` by replacing
`start` to `end` with `text`; `kind` is a `CompletionKind` (`Book`, `Chapter`, `Verse`, or
`Translation`).

```ts
let typed = "Jn 3:16 and Rom";
for (const c of topos.complete(typed, typed.length, OffsetUnit.Utf16, BookStyle.Name, 3)) {
  console.log(Object.keys(CompletionKind)[c.kind], JSON.stringify(c.label), c.start, c.end, JSON.stringify(c.text));
}
// Book "Romans" 12 15 "Romans "
// Chapter "Romans 1" 12 15 "Romans 1"
// Chapter "Romans 2" 12 15 "Romans 2"

const completions = topos.complete("see Gen 1:", 10, OffsetUnit.Utf16, BookStyle.Name, 0);
console.log(completions.length, completions[0].text, completions[0].kind === CompletionKind.Verse);
// 31 Genesis 1:1 true
```

## Queries

A `ToposQuery` chooses which references `searchWith(text, unit, query)` keeps, like the CLI's
filter options. Each method returns a new query, so one can be reused and extended:

```ts
text = "Gen 1:1; John 3; John 3:16 ESV; Rom 8:28; Rom 8:1-4; 1 Cor 13:4; Jude 25; Ps 23";
const found = (query: ToposQuery) => topos.searchWith(text, OffsetUnit.Utf16, query).map((m) => m.passage.reference).join(", ");

const every = ToposQuery.create();
console.log(found(every));
// Genesis 1:1, John 3, John 3:16, Romans 8:28, Romans 8:1-4, 1 Corinthians 13:4, Jude 1:25, Psalm 23
```

Testaments (`testament(Testament.Old)`, `oldTestament()`, `newTestament()`) narrow the rest;
genres and books add up; exclusions always win:

```ts
console.log(found(every.oldTestament()), "|", found(every.testament(Testament.Old)));
// Genesis 1:1, Psalm 23 | Genesis 1:1, Psalm 23
console.log(found(every.newTestament().excludeTestament(Testament.Old).genre("Pauline Epistles")));
// Romans 8:28, Romans 8:1-4, 1 Corinthians 13:4
console.log(found(every.book("Jn").book("Ps")), "|", found(every.excludeGenre("pauline").excludeBook("Genesis")));
// John 3, John 3:16, Psalm 23 | John 3, John 3:16, Jude 1:25, Psalm 23
```

Passages: `inside` keeps references entirely inside one, `anyOverlap` those sharing a verse,
`explicitOverlap` those naming one of its verses (whole chapters don't count), and `exactOverlap`
those that are exactly it, however written. These are joined with OR; `excludeOverlap` then
drops references that share a verse:

```ts
console.log(found(every.inside("Romans 8")), "|", found(every.anyOverlap("John 3:16")));
// Romans 8:28, Romans 8:1-4 | John 3, John 3:16
console.log(found(every.explicitOverlap("John 3")), "|", found(every.exactOverlap("Romans 8:1,2,3,4")));
// John 3:16 | Romans 8:1-4
console.log(found(every.excludeOverlap("Romans 8").excludeOverlap("John")));
// Genesis 1:1, 1 Corinthians 13:4, Jude 1:25, Psalm 23
```

`verses(singleChapterBooks)` keeps references that name a verse, as the CLI does without
`--chapters` (`false` also drops single-chapter books' references like `Jude 25`), and
`translation(name)` keeps references followed by a translation's name:

```ts
console.log(found(every.verses(true)));
// Genesis 1:1, John 3:16, Romans 8:28, Romans 8:1-4, 1 Corinthians 13:4, Jude 1:25
console.log(found(every.verses(false)), "|", found(every.translation("ESV")));
// Genesis 1:1, John 3:16, Romans 8:28, Romans 8:1-4, 1 Corinthians 13:4 | John 3:16
```

`contradiction(query)` says why nothing can match, if the query's filters contradict each other:

```ts
console.log(topos.contradiction(every.oldTestament().genre("Pauline Epistles")), "|", topos.contradiction(every));
// no included genre or book is in the included testaments | null
```

`booksIn(query)` lists the books its testaments, genres, and books keep (its passage filters
aside), by id: every book for an empty query, and none when they contradict each other. An
unknown genre or book throws `InvalidQuery`:

```ts
console.log(topos.booksIn(every.genre("Gospels").excludeBook("John")).join(", "), "|", topos.booksIn(every.oldTestament().genre("Pauline Epistles")).length);
// 40, 41, 42 | 0
```

## Formats

A `ToposFormat` says how references are written, like the CLI's `--psg-fmt` and `--fmt-*`
options; `formatPassage(passage, format)` writes one. Chain the methods, or start from the CLI's
JSON with `withJson(json)`:

```ts
passage = topos.parse("John 3:16,17,18; 4", BookStyle.Name)!;
const plain = ToposFormat.create();
console.log(topos.formatPassage(passage, plain));
// John 3:16,17,18; 4
const short = plain.book(BookStyle.Abbreviation).joinAdjacent(true).range("–");
console.log(topos.formatPassage(passage, short));
// Jn 3:16–18; 4
const dotted = plain.bookSeparator(".").chapterVerse(".").verseSeparator(", ").chapterSeparator(" & ");
console.log(topos.formatPassage(passage, dotted));
// John.3.16, 17, 18 & 4
console.log(topos.formatPassage(passage, plain.withJson('{"join_adjacent": true, "chapter_verse": "."}')));
// John 3.16-18; 4

console.log(topos.formatPassage(topos.parse("John 1:1-2:3", BookStyle.Name)!, plain.omitFirstVerseOfChapterRange(true)));
// John 1-2:3
console.log(topos.formatPassage(topos.parse("Jude 5", BookStyle.Name)!, plain.chapterInSingleChapterBooks(false)));
// Jude 5
```

| Method | CLI | Default |
|---|---|---|
| `book(style)` | `-f` | `BookStyle.Name` |
| `bookSeparator(text)` | `--fmt-book-separator` | `" "` |
| `chapterVerse(text)` | `--fmt-chapter-verse` | `":"` |
| `range(text)` | `--fmt-range` | `"-"` |
| `verseSeparator(text)` | `--fmt-verse-separator` | `","` |
| `chapterSeparator(text)` | `--fmt-chapter-separator` | `"; "` |
| `joinAdjacent(bool)` | `--fmt-join-adjacent` | `false` |
| `omitFirstVerseOfChapterRange(bool)` | `--fmt-omit-first-verse-of-chapter-range` | `false` |
| `chapterInSingleChapterBooks(bool)` | `--fmt-chapter-in-single-chapter-books` | `true` |

`completeWith(text, cursor, unit, format, limit)` completes like `complete`, written with a
format:

```ts
typed = "see Jn 3:1";
console.log(JSON.stringify(topos.completeWith(typed, typed.length, OffsetUnit.Utf16, short, 3).map((c) => c.text)));
// ["Jn 3:1","Jn 3:10","Jn 3:11"]
```

## Options

A `ToposOptions` sets up an instance, like the CLI's data and context options. Chain the methods,
then `build()`. `mergeData(json)` adds names, books, genres, or chapter counts, and
`removeData(json)` removes values, or a book or genre listed alone:

```ts
const custom = ToposOptions.create()
  .mergeData('{"books":[{"book":"John","abbreviations":["jhn"]}]}')
  .removeData('{"books":[{"book":"Jude"}]}')
  .build();
console.log(custom.search("Jhn 3:16 and Jude 5", OffsetUnit.Utf16).map((m) => m.passage.reference).join(", "));
// John 3:16
```

`contextBook(name)` gives bare references like `3:16` a book, and `contextHeading(pattern)` lets
headings set it for what follows:

```ts
const inJohn = ToposOptions.create().contextBook("John").build();
console.log(inJohn.search("see 3:16 and 4:1", OffsetUnit.Utf16).map((m) => m.passage.reference).join(", "));
// John 3:16, John 4:1
const headings = ToposOptions.create().contextHeading("^#+ {book}$").build();
console.log(headings.search("# Romans\nsee 8:28\n# John\nand 3:16", OffsetUnit.Utf16).map((m) => m.passage.reference).join(", "));
// Romans 8:28, John 3:16
```

`format(format)` sets how `search` and `searchWith` write references (`parse` and `complete` use
it too, with the book style they're given):

```ts
const abbreviated = ToposOptions.create().format(short).build();
console.log(abbreviated.search("John 3:16,17,18", OffsetUnit.Utf16).map((m) => m.passage.reference).join(", "));
// Jn 3:16–18
```

`booksAlone(true)` makes searches find books named alone too, each as the whole book. It's off
by default, since in running text `Mark`, `Job`, and `Acts` are also words; turn it on for text
meant to be references, like a search box:

```ts
const alone = ToposOptions.create().booksAlone(true).build();
console.log(alone.search("Romans and Jude, Titus 2:11", OffsetUnit.Utf16).map((m) => m.passage.reference).join(", "));
// Romans 1-16, Jude 1:1-25, Titus 2:11
```

`data(json)` replaces the default `books`, `genres`, or `chapter_verses` (each part the JSON
has; the same JSON as the CLI's `--data`). `Topos.withConfig(json)` is the older way to do it,
and returns `null` on bad input instead of throwing:

```ts
const twoBooks = JSON.stringify({
  books: [
    { id: 1, book: "Genesis", abbreviation: "Gn", osis: "Gen", abbreviations: ["gen"] },
    { id: 2, book: "Exodus", abbreviation: "Ex", osis: "Exod", abbreviations: ["exod"] },
  ],
  chapter_verses: { Genesis: [31, 25], Exodus: [22] },
});
const small = ToposOptions.create().data(twoBooks).build();
console.log(small.books().map((b) => b.name).join(", "), "|", small.search("Gen 2:4, Exod 1:1, Exod 2:1, John 3:16", OffsetUnit.Utf16).map((m) => m.passage.reference).join(", "));
// Genesis, Exodus | Genesis 2:4, Exodus 1:1
console.log(Topos.withConfig(twoBooks)?.books().length, Topos.withConfig("{"));
// 2 null
```

## Translations

A `ToposTranslations` holds the translations a reference can be followed by (`John 3:16 KJV`,
`(KJV)`, `, KJV`), like the Bibles an app has: `add(name, aliases)` adds one, and
`chapterVerses(name, bookId, counts)` gives a book its own verse counts in it. Names and aliases
match ignoring case; `names()` lists them and `get(name)` finds the name an alias belongs to.

```ts
const translations = ToposTranslations.create()
  .add("ESV", [])
  .add("KJV", ["AV"])
  .add("LXX", [])
  .chapterVerses("ESV", 64, new Uint8Array([15])); // 3 John has 15 verses in the ESV
console.log(translations.names().join(", "), "|", translations.get("av"), translations.get("NIV"));
// ESV, KJV, LXX | KJV null
```

`parseWithTranslation(reference, unit, translations, style)` is like `parse`, and returns a
`TranslatedPassage { passage, translation }` with the `Translation { name, start, end }` named
after it. A reference naming a translation with its own verse counts is checked against them, so
3 John 15 exists in the ESV but not in the KJV (which uses the default counts):

```ts
const named = topos.parseWithTranslation("3 John 15 (esv)", OffsetUnit.Utf16, translations, BookStyle.Name);
console.log(named?.passage.reference, JSON.stringify(named?.translation));
// 3 John 1:15 {"name":"ESV","start":11,"end":14}
console.log(topos.parseWithTranslation("3 John 15 (av)", OffsetUnit.Utf16, translations, BookStyle.Name));
// null
```

`searchWithTranslations(text, unit, query, translations)` returns each reference as a
`TranslatedMatch { reference, translation }`. A name ends a reference, so `Ps 22, LXX` is Psalm
22 in the LXX (not Psalms 22 and 70). With it, a query's `translation(name)` also matches aliases:

```ts
text = "Ps 22, LXX; John 3:16 AV";
for (const m of topos.searchWithTranslations(text, OffsetUnit.Utf16, ToposQuery.create(), translations)) {
  console.log(m.reference.passage.reference, m.translation?.name, text.slice(m.translation!.start, m.translation!.end));
}
// Psalm 22 LXX LXX
// John 3:16 KJV AV
const kjv = topos.searchWithTranslations(text, OffsetUnit.Utf16, ToposQuery.create().translation("KJV"), translations);
console.log(kjv.map((m) => m.reference.passage.reference).join(", "));
// John 3:16
```

`completeWithTranslations(text, cursor, unit, format, limit, translations)` is like
`completeWith`, and offers translations first after a reference:

```ts
typed = "John 3:16 K";
for (const c of topos.completeWithTranslations(typed, typed.length, OffsetUnit.Utf16, ToposFormat.create(), 1, translations)) {
  console.log(Object.keys(CompletionKind)[c.kind], c.text, c.start, c.end);
}
// Translation KJV 10 11
```

## Errors

Methods that can fail throw `ToposErrorException`, with the error in `value`: a `ToposError`
whose `tag` is `InvalidConfig` (data, options, or a format's JSON that can't be used),
`InvalidQuery` (an unknown book, genre, or passage, or a bad path pattern), `InvalidIndex` (an
index file that's damaged or from another version), or `Files` (searching files, which
WebAssembly can't), and whose `message` says why. The exception's own `message` is that
message too.

```ts
try {
  ToposOptions.create().mergeData('{"books":[{"book":"John","abbreviations":["gen"]}]}').build();
} catch (error) {
  if (error instanceof ToposErrorException) console.log(error.value.tag, error.value.message);
}
// InvalidConfig invalid book data: `gen` would mean both Genesis and John

try {
  topos.searchWith("John 3:16", OffsetUnit.Utf16, ToposQuery.create().book("Hezekiah"));
} catch (error) {
  // `message` is the error's own, so code that only logs errors says why too
  if (error instanceof ToposErrorException) console.log(error.value.tag, "|", error.message);
}
// InvalidQuery | unknown book "Hezekiah"

try {
  ToposFormat.create().withJson('{"bold": true}');
} catch (error) {
  if (error instanceof ToposErrorException) console.log(error.value.tag, error.value.message.split(",")[0]);
}
// InvalidConfig unknown field `bold`
```

## Files and folders

The native packages (Python, Swift, Kotlin) search files and folders like the CLI, with
`searchFiles(paths, files, query, unit)` and `listFiles(paths, files)`. WebAssembly has no file
access, so in this package both throw a `Files` error: read files yourself and use `search`, or
keep their references in an index (below). `ToposFiles`, which holds the CLI's walking options,
is here so code can be shared with the native packages; the
[Python guide](https://pypi.org/project/topos-bible/) shows what each option does.

```ts
const files = ToposFiles.create()
  .hidden(true).noIgnore(false).noIgnoreVcs(false).noIgnoreParent(false).noRequireGit(false)
  .ignoreFile(".myignore").followLinks(true).maxDepth(3).maxFilesize(1_000_000n).oneFileSystem(true)
  .glob("!Archive").iglob("*.MD").pathRegex("sermons/").extension("md").excludeExtension("pdf")
  .binary(false).cfiAssertions(true).cache(true).cacheDir("/tmp/topos-cache");
for (const call of [
  () => topos.searchFiles(["notes"], files, ToposQuery.create(), OffsetUnit.Utf16),
  () => topos.listFiles(["notes"], files),
]) {
  try {
    call();
  } catch (error) {
    if (error instanceof ToposErrorException) console.log(error.value.tag, error.value.message);
  }
}
// Files WebAssembly has no file access: read files yourself and use search or ToposIndex
// Files WebAssembly has no file access: read files yourself and use search or ToposIndex
```

## An index of many files

A `ToposIndex` keeps the references of many files between runs, compactly (about 15 bytes each),
so only files that changed are searched again, and devices that sync the same files share it.
The app reads and writes the files; the index takes their text or bytes, sizes, and modification
times in milliseconds. These examples use Node's `node:fs` and `node:path`
(`readFileSync`, `writeFileSync`, `statSync`, `mkdirSync`, `readdirSync`, `join`).

`ToposIndex.create(device, engine, unit)` makes an empty index for a device (`device` names its
packs; `engine`, a version, is written in them). `check(path, size, mtime)` says whether its entry
for a file is current (`IndexStatus.Fresh`), and `indexText(topos, path, size, mtime, text,
written)` searches a text file and keeps its references:

```ts
// A file's size and modification time in milliseconds, as the index takes them
const stamp = (path: string): [number, number] => {
  const stat = statSync(path);
  return [stat.size, Math.floor(stat.mtimeMs)];
};

const index = ToposIndex.create("laptop", "1.0", OffsetUnit.Utf16);
for (const path of ["notes/sermon.md", "notes/old/draft.txt"]) {
  const [size, mtime] = stamp(path);
  if (index.check(path, size, mtime) !== IndexStatus.Fresh) {
    console.log(path, index.indexText(topos, path, size, mtime, readFileSync(path, "utf8"), Date.now()));
  }
}
// notes/sermon.md 2
// notes/old/draft.txt 1
console.log(Object.keys(IndexStatus)[index.check("notes/sermon.md", ...stamp("notes/sermon.md"))]);
// Fresh
```

`Topos.indexEntry(path, size, mtime, text, written, unit)` searches a text file into bytes
that `insert(bytes, path, size, mtime)` adds, so a Web Worker can search while the index stays
where it's queried. `indexEpub(path, size, mtime, bytes, written)` does the same for an EPUB's
bytes (its references get CFIs and the text around them), and `indexPdf` for a PDF's, only in
native builds with MuPDF. EPUB and PDF entries count offsets in UTF-16, so an index that holds
them is created with `OffsetUnit.Utf16`.

```ts
let [size, mtime] = stamp("notes/talk.srt");
let entry = topos.indexEntry("notes/talk.srt", size, mtime, readFileSync("notes/talk.srt", "utf8"), Date.now(), OffsetUnit.Utf16);
index.insert(entry, "notes/talk.srt", size, mtime);

[size, mtime] = stamp("notes/book.epub");
entry = topos.indexEpub("notes/book.epub", size, mtime, readFileSync("notes/book.epub"), Date.now());
index.insert(entry, "notes/book.epub", size, mtime);

try {
  topos.indexPdf("notes/paper.pdf", 4, 0, new Uint8Array([37, 80, 68, 70]), Date.now());
} catch (error) {
  if (error instanceof ToposErrorException) console.log(error.value.tag, error.value.message);
}
// InvalidIndex PDFs need topos built with the pdf feature (MuPDF), natively
```

`setEpub(path, size, mtime, written, references)` keeps an EPUB's references found elsewhere
(by a reader that has the book open), each an `EpubReference`. `paths()`, `fileCount()`, and
`referenceCount()` say what the index has:

```ts
const reference: EpubReference = {
  passage: topos.parse("John 1:1", BookStyle.Name)!,
  start: 0, end: 8, line: 1, column: 1, spineIndex: 0, chapter: "Prologue",
  cfi: "epubcfi(/6/2!/4/2,/1:0,/1:8)", lineText: "John 1:1 says",
};
index.setEpub("notes/other.epub", 2048, 0, Date.now(), [reference]);

console.log(index.paths().join(", "), "|", index.fileCount(), index.referenceCount());
// notes/sermon.md, notes/old/draft.txt, notes/talk.srt, notes/book.epub, notes/other.epub | 5 7
```

`query(topos, query, scope, path, order)` keeps the references a `ToposQuery` keeps, in every
file (`IndexScope.All`), one file (`File`), or a folder (`Folder`), sorted by file
(`IndexOrder.File`) or by where passages start in the Bible (`Bible`). Its `IndexResults` has
`len()`, `isEmpty()`, and `fileCount()`, and `page(topos, results, offset, limit)` reads them as
`IndexHit { path, passage, start, end, line, column, lineText, epub, pdf }`; EPUBs' hits have an
`IndexEpub { spineIndex, cfi, chapter }`, and PDFs' an `IndexPdf { page, locator }`.

```ts
const results = index.query(topos, ToposQuery.create().newTestament(), IndexScope.All, "", IndexOrder.Bible);
console.log(results.len(), results.isEmpty(), results.fileCount());
// 5 false 4
for (const hit of index.page(topos, results, 0, 3)) {
  console.log(hit.path, hit.passage.reference, hit.start, hit.end, hit.line, hit.column, JSON.stringify(hit.lineText), JSON.stringify(hit.epub), hit.pdf);
}
// notes/other.epub John 1:1 0 8 1 1 "John 1:1 says" {"spineIndex":0,"cfi":"epubcfi(/6/2!/4/2,/1:0,/1:8)","chapter":"Prologue"} null
// notes/book.epub John 3:16 4 11 1 5 "See Jn 3:16." {"spineIndex":0,"cfi":"epubcfi(/6/2!/4/2,/1:4,/1:11)","chapter":null} null
// notes/talk.srt John 3:16 40 49 3 9 "" null null
```

`queryIn(topos, query, scope, path, extensions, order)` keeps only files with those extensions,
`extensionCounts(results)` counts references and files by extension, and `fileHits(topos, path)`
gives one file's references:

```ts
const references = (results: IndexResults) => index.page(topos, results, 0, 10).map((hit) => hit.passage.reference).join(", ");
const inFolder = index.query(topos, ToposQuery.create(), IndexScope.Folder, "notes/old", IndexOrder.File);
const inFile = index.query(topos, ToposQuery.create(), IndexScope.File, "notes/sermon.md", IndexOrder.File);
console.log(references(inFolder), "|", references(inFile));
// Genesis 1:1 | Romans 8:28, Psalm 23:1
console.log(references(index.queryIn(topos, ToposQuery.create(), IndexScope.All, "", ["md", "txt"], IndexOrder.File)));
// Genesis 1:1, Romans 8:28, Psalm 23:1
console.log(JSON.stringify(index.extensionCounts(results)));
// [{"extension":"epub","references":3,"files":2},{"extension":"md","references":1,"files":1},{"extension":"srt","references":1,"files":1}]
console.log(index.fileHits(topos, "notes/sermon.md").map((hit) => hit.passage.reference).join(", "));
// Romans 8:28, Psalm 23:1
```

`queryPaths(topos, query, paths, order)` keeps the files a `ToposPaths` keeps: files and folders
(`path`), globs like ripgrep's (`glob`, and `iglob` ignoring case), regular expressions matched
anywhere in the path (`regex`), and extensions (`extension`, any case). A file is kept when it's
one of the paths (if any are given), matches an included glob or regular expression (if any
are), matches no excluded one (`!` excludes), and has one of the extensions (if any are).
`problems()` says why a pattern can't be read; queries throw `InvalidQuery` for it.

```ts
const inPaths = (paths: ToposPaths) => index.page(topos, index.queryPaths(topos, ToposQuery.create(), paths, IndexOrder.File), 0, 10).map((hit) => hit.path).join(", ");
console.log(inPaths(ToposPaths.create().path("notes/old").path("notes/sermon.md").glob("!*.txt")));
// notes/sermon.md, notes/sermon.md
console.log(inPaths(ToposPaths.create().iglob("*.EPUB").regex("talk")));
// notes/book.epub, notes/book.epub, notes/other.epub, notes/talk.srt
console.log(inPaths(ToposPaths.create().extension("TXT")), "|", ToposPaths.create().glob("[").regex("(").problems()[0]);
// notes/old/draft.txt | bad glob "[": unclosed character class; missing ']'
```

The index doesn't store translations, so its queries can't take `translation(name)`:

```ts
try {
  index.query(topos, ToposQuery.create().translation("ESV"), IndexScope.All, "", IndexOrder.File);
} catch (error) {
  if (error instanceof ToposErrorException) console.log(error.value.message);
}
// the index doesn't store translations, so it can't filter by them
```

**Saving.** `isDirty()` says whether this device has packs to write; `takeDirtyPacks()` and
`takeUnsavedDetails()` give them as `IndexFile { name, bytes }` (a pack is
`<device>-<kind>-<number>.bin`, an EPUB's or PDF's details `<name>.bin`). Each kind of file
(`IndexKind.Text`, `Epub`, `Pdf`) has packs of its own: `packKind(name)` says which a pack is
for (`null` for another file), and `kindOf(path)` which a file is. `detailNames()` lists the
details entries use, so others can be deleted. `isCurrent(results)` says whether the index is
unchanged since a query.

```ts
console.log(index.isCurrent(results), index.isDirty());
// true true
mkdirSync("index");
for (const file of [...index.takeDirtyPacks(), ...index.takeUnsavedDetails()]) {
  writeFileSync(join("index", file.name), file.bytes);
}
console.log(readdirSync("index").filter((name) => name.startsWith("laptop-")).sort().join(", "), "|", index.isDirty());
// laptop-epub-10.bin, laptop-epub-29.bin, laptop-text-14.bin, laptop-text-16.bin, laptop-text-18.bin | false
const kind = index.packKind("laptop-epub-10.bin");
console.log(kind === IndexKind.Epub, index.packKind("notes.bin"), Object.keys(IndexKind)[index.kindOf("notes/talk.srt")], index.detailNames().length);
// true null Text 2
```

**Loading.** `loadPack(bytes)` adds a pack, this device's from an earlier run or another
device's; load only the kinds the app searches. A loaded entry is used once `check` finds it
current. After syncing, a file's time can differ: `check` then says `Unconfirmed` (the entry is
used meanwhile), and `confirm(path, size, mtime, text)` checks it by the file's text (or, with
`null`, by its size alone, for files too big to read again).

```ts
const phone = ToposIndex.create("phone", "1.0", OffsetUnit.Utf16);
for (const name of readdirSync("index").sort()) {
  if (phone.packKind(name) !== null) phone.loadPack(readFileSync(join("index", name)));
}
for (const path of ["notes/sermon.md", "notes/old/draft.txt", "notes/talk.srt", "notes/book.epub"]) {
  phone.check(path, ...stamp(path));
}
console.log(phone.paths().sort().join(", "), "|", phone.referenceCount());
// notes/book.epub, notes/old/draft.txt, notes/sermon.md, notes/talk.srt | 6

[size, mtime] = stamp("notes/sermon.md");
console.log(Object.keys(IndexStatus)[phone.check("notes/sermon.md", size, mtime + 5000)]);
// Unconfirmed
console.log(phone.confirm("notes/sermon.md", size, mtime + 5000, readFileSync("notes/sermon.md", "utf8")));
// true
```

EPUBs' and PDFs' details (CFIs, locators, and the text around each reference) are read on
demand: `missingDetails(results, offset, limit)` names those a page needs that aren't loaded,
`missingFileDetail(path)` those a file needs, and `loadDetail(name, bytes)` loads one.

```ts
const everything = phone.query(topos, ToposQuery.create(), IndexScope.All, "", IndexOrder.File);
const missing = phone.missingDetails(everything, 0, 10);
console.log(missing.length, missing[0] === phone.missingFileDetail("notes/book.epub"));
// 1 true
for (const name of missing) phone.loadDetail(name, readFileSync(join("index", `${name}.bin`)));
console.log(JSON.stringify(phone.fileHits(topos, "notes/book.epub").map((hit) => hit.lineText)), phone.missingFileDetail("notes/book.epub"));
// ["See Jn 3:16.","And Romans 8:28"] null
```

`rename(old, new)`, `remove(path)` (a file deleted, or no longer searched), and `retain(paths)`
(every file not in `paths` taken out) follow the files:

```ts
phone.rename("notes/old/draft.txt", "notes/draft.txt");
console.log(phone.isCurrent(everything), phone.paths().sort().join(", "));
// false notes/book.epub, notes/draft.txt, notes/sermon.md, notes/talk.srt
phone.remove("notes/talk.srt");
phone.retain(["notes/sermon.md", "notes/draft.txt"]);
console.log(phone.paths().sort().join(", "));
// notes/draft.txt, notes/sermon.md
```

Packs that are damaged, from another version, or counted in another unit throw `InvalidIndex`:

```ts
try {
  phone.loadPack(new TextEncoder().encode("not a pack"));
} catch (error) {
  if (error instanceof ToposErrorException) console.log(error.value.tag, error.value.message);
}
// InvalidIndex not a topos index file
```

## Freeing objects

Objects (`Topos`, `ToposIndex`, `IndexResults`, the builders) hold Rust memory, freed when
they're garbage collected. `dispose()` frees it now:

```ts
for (const object of [results, everything, index, phone, translations, custom, inJohn, headings, abbreviated, small, topos]) {
  object.dispose();
}
```

Source, the CLI, and other languages: [github.com/MasterTemple/topos-bible](https://github.com/MasterTemple/topos-bible).
Released under [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/).
