// Every feature of topos-bible, in the order of packaging/npm/README.md, whose code blocks are
// parts of this file. Run with `npm install && npm start` after `npm run pack:wasm` (see
// ../../README.md); the comments right after each console.log are what it prints.
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { crc32 } from "node:zlib";

// A small EPUB (a ZIP without compression): `See Jn 3:16.` and `And Romans 8:28`
function epub(): Uint8Array {
  const page = (body: string) =>
    `<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>t</title></head><body><p id="p">${body}</p></body></html>`;
  const files: [string, string][] = [
    ["mimetype", "application/epub+zip"],
    ["META-INF/container.xml", '<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>'],
    ["content.opf", '<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata/><manifest><item id="a" href="a.xhtml" media-type="application/xhtml+xml"/><item id="b" href="b.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="a"/><itemref idref="b"/></spine></package>'],
    ["a.xhtml", page("See Jn 3:16.")],
    ["b.xhtml", page("And Romans 8:28")],
  ];
  const parts: Buffer[] = [];
  const central: Buffer[] = [];
  let offset = 0;
  for (const [name, text] of files) {
    const data = Buffer.from(text);
    const nameBytes = Buffer.from(name);
    const header = Buffer.alloc(30);
    header.writeUInt32LE(0x04034b50, 0);
    header.writeUInt16LE(20, 4);
    header.writeUInt32LE(crc32(data), 14);
    header.writeUInt32LE(data.length, 18);
    header.writeUInt32LE(data.length, 22);
    header.writeUInt16LE(nameBytes.length, 26);
    const entry = Buffer.alloc(46);
    entry.writeUInt32LE(0x02014b50, 0);
    entry.writeUInt16LE(20, 4);
    entry.writeUInt16LE(20, 6);
    entry.writeUInt32LE(crc32(data), 16);
    entry.writeUInt32LE(data.length, 20);
    entry.writeUInt32LE(data.length, 24);
    entry.writeUInt16LE(nameBytes.length, 28);
    entry.writeUInt32LE(offset, 42);
    parts.push(header, nameBytes, data);
    central.push(entry, nameBytes);
    offset += header.length + nameBytes.length + data.length;
  }
  const directory = Buffer.concat(central);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(files.length, 8);
  end.writeUInt16LE(files.length, 10);
  end.writeUInt32LE(directory.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...parts, directory, end]);
}

// The files that the index section reads, in a temporary folder
const workspace = mkdtempSync(join(tmpdir(), "topos-example-"));
process.chdir(workspace);
for (const [name, text] of Object.entries({
  "notes/sermon.md": "Text: Rom 8:28\nAlso Ps 23:1",
  "notes/talk.srt": "1\n00:00:01,000 --> 00:00:04,000\nTurn to John 3:16\n",
  "notes/old/draft.txt": "Gen 1:1",
})) {
  mkdirSync(dirname(name), { recursive: true });
  writeFileSync(name, text);
}
writeFileSync("notes/book.epub", epub());

// --- Usage

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

// --- Finding references

let text = "Notes 📖: read Jn 3:16-18 and Rom 8:28.";
for (const m of topos.search(text, OffsetUnit.Utf16)) {
  console.log(m.passage.reference, m.start, m.end, m.line, m.column, text.slice(m.start, m.end));
}
// John 3:16-18 15 25 1 16 Jn 3:16-18
// Romans 8:28 30 38 1 31 Rom 8:28

for (const part of topos.parts("John 3:16; 4:5-7", OffsetUnit.Utf16)) {
  console.log(part.passage.reference, part.start, part.end, part.first.reference, part.last.reference, part.lastStart);
}
// John 3:16 0 9 John 3:16 John 3:16 null
// John 4:5-7 11 16 John 4:5 John 4:7 15

// --- Parsing a reference

let passage = topos.parse("John 3:16-18,20-4:2; 5", BookStyle.Name)!;
console.log(passage.bookId, passage.book, "|", passage.reference, "|", passage.osis, "|", passage.usfm);
// 43 John | John 3:16-18,20-4:2; 5 | John.3.16-John.3.18 John.3.20-John.4.2 John.5 | JHN 3:16-18; JHN 3:20-4:2; JHN 5

for (const style of [BookStyle.Name, BookStyle.Abbreviation, BookStyle.Osis, BookStyle.Usfm]) {
  console.log(Object.keys(BookStyle)[style], topos.parse("1 Cor 13:4", style)?.reference);
}
// Name 1 Corinthians 13:4
// Abbreviation 1 Cor 13:4
// Osis 1Cor 13:4
// Usfm 1CO 13:4

console.log(topos.parse("John.3.16", BookStyle.Name)?.reference, "|", topos.parse("JHN 3:16", BookStyle.Name)?.reference);
// John 3:16 | John 3:16
console.log(topos.parse("Jude 5", BookStyle.Name)?.reference, "|", topos.parse("John 3:99", BookStyle.Name));
// Jude 1:5 | null
console.log(topos.parse("Gen", BookStyle.Name)?.reference, "|", topos.parse("Jude", BookStyle.Name)?.reference);
// Genesis | Jude

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

// --- Verses

const john = topos.parse("John 3; 4:1-3", BookStyle.Name)!;
console.log(JSON.stringify(topos.verseRanges(john)));
// [{"start":{"chapter":3,"verse":1},"end":{"chapter":3,"verse":36}},{"start":{"chapter":4,"verse":1},"end":{"chapter":4,"verse":3}}]
console.log(topos.verses(john).length, JSON.stringify(topos.verses(topos.parse("John 3:16-18", BookStyle.Name)!)));
// 39 [{"chapter":3,"verse":16},{"chapter":3,"verse":17},{"chapter":3,"verse":18}]
console.log(topos.verseCount(43, 3), topos.verseCount(43, 22));
// 36 null

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

// --- Books and genres

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

// --- Completing

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

// --- Queries

text = "Gen 1:1; John 3; John 3:16 ESV; Rom 8:28; Rom 8:1-4; 1 Cor 13:4; Jude 25; Ps 23";
const found = (query: ToposQuery) => topos.searchWith(text, OffsetUnit.Utf16, query).map((m) => m.passage.reference).join(", ");

const every = ToposQuery.create();
console.log(found(every));
// Genesis 1:1, John 3, John 3:16, Romans 8:28, Romans 8:1-4, 1 Corinthians 13:4, Jude 1:25, Psalm 23

console.log(found(every.oldTestament()), "|", found(every.testament(Testament.Old)));
// Genesis 1:1, Psalm 23 | Genesis 1:1, Psalm 23
console.log(found(every.newTestament().excludeTestament(Testament.Old).genre("Pauline Epistles")));
// Romans 8:28, Romans 8:1-4, 1 Corinthians 13:4
console.log(found(every.book("Jn").book("Ps")), "|", found(every.excludeGenre("pauline").excludeBook("Genesis")));
// John 3, John 3:16, Psalm 23 | John 3, John 3:16, Jude 1:25, Psalm 23

console.log(found(every.inside("Romans 8")), "|", found(every.anyOverlap("John 3:16")));
// Romans 8:28, Romans 8:1-4 | John 3, John 3:16
console.log(found(every.explicitOverlap("John 3")), "|", found(every.exactOverlap("Romans 8:1,2,3,4")));
// John 3:16 | Romans 8:1-4
console.log(found(every.excludeOverlap("Romans 8").excludeOverlap("John")));
// Genesis 1:1, 1 Corinthians 13:4, Jude 1:25, Psalm 23

console.log(found(every.verses(true)));
// Genesis 1:1, John 3:16, Romans 8:28, Romans 8:1-4, 1 Corinthians 13:4, Jude 1:25
console.log(found(every.verses(false)), "|", found(every.translation("ESV")));
// Genesis 1:1, John 3:16, Romans 8:28, Romans 8:1-4, 1 Corinthians 13:4 | John 3:16

console.log(topos.contradiction(every.oldTestament().genre("Pauline Epistles")), "|", topos.contradiction(every));
// no included genre or book is in the included testaments | null
console.log(topos.booksIn(every.genre("Gospels").excludeBook("John")).join(", "), "|", topos.booksIn(every.oldTestament().genre("Pauline Epistles")).length);
// 40, 41, 42 | 0

// --- Formats

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

typed = "see Jn 3:1";
console.log(JSON.stringify(topos.completeWith(typed, typed.length, OffsetUnit.Utf16, short, 3).map((c) => c.text)));
// ["Jn 3:1","Jn 3:10","Jn 3:11"]

// --- Options

const custom = ToposOptions.create()
  .mergeData('{"books":[{"book":"John","abbreviations":["jhn"]}]}')
  .removeData('{"books":[{"book":"Jude"}]}')
  .build();
console.log(custom.search("Jhn 3:16 and Jude 5", OffsetUnit.Utf16).map((m) => m.passage.reference).join(", "));
// John 3:16

const inJohn = ToposOptions.create().contextBook("John").build();
console.log(inJohn.search("see 3:16 and 4:1", OffsetUnit.Utf16).map((m) => m.passage.reference).join(", "));
// John 3:16, John 4:1
const headings = ToposOptions.create().contextHeading("^#+ {book}$").build();
console.log(headings.search("# Romans\nsee 8:28\n# John\nand 3:16", OffsetUnit.Utf16).map((m) => m.passage.reference).join(", "));
// Romans 8:28, John 3:16

const abbreviated = ToposOptions.create().format(short).build();
console.log(abbreviated.search("John 3:16,17,18", OffsetUnit.Utf16).map((m) => m.passage.reference).join(", "));
// Jn 3:16–18
const alone = ToposOptions.create().booksAlone(true).build();
console.log(alone.search("Romans and Jude, Titus 2:11", OffsetUnit.Utf16).map((m) => m.passage.reference).join(", "));
// Romans, Jude, Titus 2:11

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

// --- Translations

const translations = ToposTranslations.create()
  .add("ESV", [])
  .add("KJV", ["AV"])
  .add("LXX", [])
  .chapterVerses("ESV", 64, new Uint8Array([15])); // 3 John has 15 verses in the ESV
console.log(translations.names().join(", "), "|", translations.get("av"), translations.get("NIV"));
// ESV, KJV, LXX | KJV null

const named = topos.parseWithTranslation("3 John 15 (esv)", OffsetUnit.Utf16, translations, BookStyle.Name);
console.log(named?.passage.reference, JSON.stringify(named?.translation));
// 3 John 1:15 {"name":"ESV","start":11,"end":14}
console.log(topos.parseWithTranslation("3 John 15 (av)", OffsetUnit.Utf16, translations, BookStyle.Name));
// null
console.log(topos.verseCount(64, 1), topos.verseCountIn(64, 1, translations, "esv"), topos.verseCountIn(64, 1, translations, "AV"));
// 14 15 14

text = "Ps 22, LXX; John 3:16 AV";
for (const m of topos.searchWithTranslations(text, OffsetUnit.Utf16, ToposQuery.create(), translations)) {
  console.log(m.reference.passage.reference, m.translation?.name, text.slice(m.translation!.start, m.translation!.end));
}
// Psalm 22 LXX LXX
// John 3:16 KJV AV
const kjv = topos.searchWithTranslations(text, OffsetUnit.Utf16, ToposQuery.create().translation("KJV"), translations);
console.log(kjv.map((m) => m.reference.passage.reference).join(", "));
// John 3:16

typed = "John 3:16 K";
for (const c of topos.completeWithTranslations(typed, typed.length, OffsetUnit.Utf16, ToposFormat.create(), 1, translations)) {
  console.log(Object.keys(CompletionKind)[c.kind], c.text, c.start, c.end);
}
// Translation KJV 10 11

// --- Errors

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

// --- Files and folders

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

// --- An index of many files

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

const reference: EpubReference = {
  passage: topos.parse("John 1:1", BookStyle.Name)!,
  start: 0, end: 8, line: 1, column: 1, spineIndex: 0, chapter: "Prologue",
  cfi: "epubcfi(/6/2!/4/2,/1:0,/1:8)", lineText: "John 1:1 says",
};
index.setEpub("notes/other.epub", 2048, 0, Date.now(), [reference]);

console.log(index.paths().join(", "), "|", index.fileCount(), index.referenceCount());
// notes/sermon.md, notes/old/draft.txt, notes/talk.srt, notes/book.epub, notes/other.epub | 5 7

const results = index.query(topos, ToposQuery.create().newTestament(), IndexScope.All, "", IndexOrder.Bible);
console.log(results.len(), results.isEmpty(), results.fileCount());
// 5 false 4
for (const hit of index.page(topos, results, 0, 3)) {
  console.log(hit.path, hit.passage.reference, hit.start, hit.end, hit.line, hit.column, JSON.stringify(hit.lineText), JSON.stringify(hit.epub), hit.pdf);
}
// notes/other.epub John 1:1 0 8 1 1 "John 1:1 says" {"spineIndex":0,"cfi":"epubcfi(/6/2!/4/2,/1:0,/1:8)","chapter":"Prologue"} null
// notes/book.epub John 3:16 4 11 1 5 "See Jn 3:16." {"spineIndex":0,"cfi":"epubcfi(/6/2!/4/2,/1:4,/1:11)","chapter":null} null
// notes/talk.srt John 3:16 40 49 3 9 "" null null

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

const inPaths = (paths: ToposPaths) => index.page(topos, index.queryPaths(topos, ToposQuery.create(), paths, IndexOrder.File), 0, 10).map((hit) => hit.path).join(", ");
console.log(inPaths(ToposPaths.create().path("notes/old").path("notes/sermon.md").glob("!*.txt")));
// notes/sermon.md, notes/sermon.md
console.log(inPaths(ToposPaths.create().iglob("*.EPUB").regex("talk")));
// notes/book.epub, notes/book.epub, notes/other.epub, notes/talk.srt
console.log(inPaths(ToposPaths.create().extension("TXT")), "|", ToposPaths.create().glob("[").regex("(").problems()[0]);
// notes/old/draft.txt | bad glob "[": unclosed character class; missing ']'

try {
  index.query(topos, ToposQuery.create().translation("ESV"), IndexScope.All, "", IndexOrder.File);
} catch (error) {
  if (error instanceof ToposErrorException) console.log(error.value.message);
}
// the index doesn't store translations, so it can't filter by them

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

const everything = phone.query(topos, ToposQuery.create(), IndexScope.All, "", IndexOrder.File);
const missing = phone.missingDetails(everything, 0, 10);
console.log(missing.length, missing[0] === phone.missingFileDetail("notes/book.epub"));
// 1 true
for (const name of missing) phone.loadDetail(name, readFileSync(join("index", `${name}.bin`)));
console.log(JSON.stringify(phone.fileHits(topos, "notes/book.epub").map((hit) => hit.lineText)), phone.missingFileDetail("notes/book.epub"));
// ["See Jn 3:16.","And Romans 8:28"] null

phone.rename("notes/old/draft.txt", "notes/draft.txt");
console.log(phone.isCurrent(everything), phone.paths().sort().join(", "));
// false notes/book.epub, notes/draft.txt, notes/sermon.md, notes/talk.srt
phone.remove("notes/talk.srt");
phone.retain(["notes/sermon.md", "notes/draft.txt"]);
console.log(phone.paths().sort().join(", "));
// notes/draft.txt, notes/sermon.md

try {
  phone.loadPack(new TextEncoder().encode("not a pack"));
} catch (error) {
  if (error instanceof ToposErrorException) console.log(error.value.tag, error.value.message);
}
// InvalidIndex not a topos index file

// --- Browsers and bundlers

import { initialized } from "topos-bible";

await initialized; // before the first call

// --- Freeing objects

for (const object of [results, everything, index, phone, translations, custom, inJohn, headings, abbreviated, small, topos]) {
  object.dispose();
}

process.chdir(tmpdir());
rmSync(workspace, { recursive: true });
