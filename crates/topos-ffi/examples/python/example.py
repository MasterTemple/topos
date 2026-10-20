"""Every feature of topos_bible, in the order of packaging/python/README.md, whose code blocks are
parts of this file. Run after `pip install dist/python/wheelhouse/*.whl` (see ../../README.md);
the comments right after each print are what it prints."""

import io
import os
import pathlib
import tempfile
import zipfile


def write_epub(path):
    """A small EPUB: two content documents, `See Jn 3:16.` and `And Romans 8:28`"""
    page = '<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>t</title></head><body><p id="p">{}</p></body></html>'
    files = {
        "mimetype": "application/epub+zip",
        "META-INF/container.xml": '<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>',
        "content.opf": '<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata/><manifest><item id="a" href="a.xhtml" media-type="application/xhtml+xml"/><item id="b" href="b.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="a"/><itemref idref="b"/></spine></package>',
        "a.xhtml": page.format("See Jn 3:16."),
        "b.xhtml": page.format("And Romans 8:28"),
    }
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as epub:
        for name, text in files.items():
            epub.writestr(name, text)
    pathlib.Path(path).write_bytes(buffer.getvalue())


# The files that the sections on files and the index search, in a temporary folder
workspace = tempfile.TemporaryDirectory()
os.chdir(workspace.name)
for name, text in {
    "notes/sermon.md": "Text: Rom 8:28\nAlso Ps 23:1",
    "notes/talk.srt": "1\n00:00:01,000 --> 00:00:04,000\nTurn to John 3:16\n",
    "notes/old/draft.txt": "Gen 1:1",
    "notes/.private.md": "John 1:1",
    "notes/scratch.md": "Acts 2:38",
    "notes/.toposignore": "scratch.md\n",
    "notes/run.log": "Mark 1:1",
    "notes/.gitignore": "*.log\n",
    "notes/huge.txt": "Isaiah 53:5 " * 100,
    ".ignore": "huge.txt\n",
    "more-ignores": "*.srt\n",
}.items():
    pathlib.Path(name).parent.mkdir(parents=True, exist_ok=True)
    pathlib.Path(name).write_text(text, encoding="utf-8")
pathlib.Path("notes/data.bin").write_bytes(b"\x00\x01John 3:16\x00")
write_epub("notes/book.epub")

# --- Usage

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

# --- Finding references

text = "Notes 📖: read Jn 3:16-18 and Rom 8:28."
for m in topos.search(text, OffsetUnit.CHAR):
    print(m.passage.reference, m.start, m.end, m.line, m.column, text[m.start : m.end])
# John 3:16-18 14 24 1 15 Jn 3:16-18
# Romans 8:28 29 37 1 30 Rom 8:28

for part in topos.parts("John 3:16; 4:5-7", OffsetUnit.CHAR):
    print(part.passage.reference, part.start, part.end, part.first.reference, part.last.reference, part.last_start)
# John 3:16 0 9 John 3:16 John 3:16 None
# John 4:5-7 11 16 John 4:5 John 4:7 15

# --- Parsing a reference

passage = topos.parse("John 3:16-18,20-4:2; 5", BookStyle.NAME)
print(passage.book_id, passage.book, "|", passage.reference, "|", passage.osis, "|", passage.usfm)
# 43 John | John 3:16-18,20-4:2; 5 | John.3.16-John.3.18 John.3.20-John.4.2 John.5 | JHN 3:16-18; JHN 3:20-4:2; JHN 5

for style in [BookStyle.NAME, BookStyle.ABBREVIATION, BookStyle.OSIS, BookStyle.USFM]:
    print(style.name, topos.parse("1 Cor 13:4", style).reference)
# NAME 1 Corinthians 13:4
# ABBREVIATION 1 Cor 13:4
# OSIS 1Cor 13:4
# USFM 1CO 13:4

print(topos.parse("John.3.16", BookStyle.NAME).reference, "|", topos.parse("JHN 3:16", BookStyle.NAME).reference)
# John 3:16 | John 3:16
print(topos.parse("Jude 5", BookStyle.NAME).reference, "|", topos.parse("John 3:99", BookStyle.NAME))
# Jude 1:5 | None
print(topos.parse("Gen", BookStyle.NAME).reference, "|", topos.parse("Jude", BookStyle.NAME).reference)
# Genesis 1-50 | Jude 1:1-25

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

# --- Verses

john = topos.parse("John 3; 4:1-3", BookStyle.NAME)
print([(r.start.chapter, r.start.verse, r.end.chapter, r.end.verse) for r in topos.verse_ranges(john)])
# [(3, 1, 3, 36), (4, 1, 4, 3)]
print(len(topos.verses(john)), [(v.chapter, v.verse) for v in topos.verses(topos.parse("John 3:16-18", BookStyle.NAME))])
# 39 [(3, 16), (3, 17), (3, 18)]

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

# --- Books and genres

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

# --- Completing

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

# --- Queries

from topos_bible import Testament, ToposQuery

text = "Gen 1:1; John 3; John 3:16 ESV; Rom 8:28; Rom 8:1-4; 1 Cor 13:4; Jude 25; Ps 23"


def found(query):
    return [m.passage.reference for m in topos.search_with(text, OffsetUnit.CHAR, query)]


every = ToposQuery.create()
print(found(every))
# ['Genesis 1:1', 'John 3', 'John 3:16', 'Romans 8:28', 'Romans 8:1-4', '1 Corinthians 13:4', 'Jude 1:25', 'Psalm 23']

print(found(every.old_testament()), found(every.testament(Testament.OLD)))
# ['Genesis 1:1', 'Psalm 23'] ['Genesis 1:1', 'Psalm 23']
print(found(every.new_testament().exclude_testament(Testament.OLD).genre("Pauline Epistles")))
# ['Romans 8:28', 'Romans 8:1-4', '1 Corinthians 13:4']
print(found(every.book("Jn").book("Ps")), found(every.exclude_genre("pauline").exclude_book("Genesis")))
# ['John 3', 'John 3:16', 'Psalm 23'] ['John 3', 'John 3:16', 'Jude 1:25', 'Psalm 23']

print(found(every.inside("Romans 8")), found(every.any_overlap("John 3:16")))
# ['Romans 8:28', 'Romans 8:1-4'] ['John 3', 'John 3:16']
print(found(every.explicit_overlap("John 3")), found(every.exact_overlap("Romans 8:1,2,3,4")))
# ['John 3:16'] ['Romans 8:1-4']
print(found(every.exclude_overlap("Romans 8").exclude_overlap("John")))
# ['Genesis 1:1', '1 Corinthians 13:4', 'Jude 1:25', 'Psalm 23']

print(found(every.verses(True)))
# ['Genesis 1:1', 'John 3:16', 'Romans 8:28', 'Romans 8:1-4', '1 Corinthians 13:4', 'Jude 1:25']
print(found(every.verses(False)), found(every.translation("ESV")))
# ['Genesis 1:1', 'John 3:16', 'Romans 8:28', 'Romans 8:1-4', '1 Corinthians 13:4'] ['John 3:16']

print(topos.contradiction(every.old_testament().genre("Pauline Epistles")), topos.contradiction(every))
# no included genre or book is in the included testaments None
print(list(topos.books_in(every.genre("Gospels").exclude_book("John"))), len(topos.books_in(every.old_testament().genre("Pauline Epistles"))))
# [40, 41, 42] 0

# --- Formats

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

typed = "see Jn 3:1"
print([c.text for c in topos.complete_with(typed, len(typed), OffsetUnit.CHAR, short, 3)])
# ['Jn 3:1', 'Jn 3:10', 'Jn 3:11']

# --- Options

from topos_bible import ToposOptions

custom = (
    ToposOptions.create()
    .merge_data('{"books":[{"book":"John","abbreviations":["jhn"]}]}')
    .remove_data('{"books":[{"book":"Jude"}]}')
    .build()
)
print([m.passage.reference for m in custom.search("Jhn 3:16 and Jude 5", OffsetUnit.CHAR)])
# ['John 3:16']

in_john = ToposOptions.create().context_book("John").build()
print([m.passage.reference for m in in_john.search("see 3:16 and 4:1", OffsetUnit.CHAR)])
# ['John 3:16', 'John 4:1']
headings = ToposOptions.create().context_heading("^#+ {book}$").build()
print([m.passage.reference for m in headings.search("# Romans\nsee 8:28\n# John\nand 3:16", OffsetUnit.CHAR)])
# ['Romans 8:28', 'John 3:16']

abbreviated = ToposOptions.create().format(short).build()
print([m.passage.reference for m in abbreviated.search("John 3:16,17,18", OffsetUnit.CHAR)])
# ['Jn 3:16–18']
alone = ToposOptions.create().books_alone(True).build()
print([m.passage.reference for m in alone.search("Romans and Jude, Titus 2:11", OffsetUnit.CHAR)])
# ['Romans 1-16', 'Jude 1:1-25', 'Titus 2:11']

two_books = '{"books":[{"id":1,"book":"Genesis","abbreviation":"Gn","osis":"Gen","abbreviations":["gen"]},{"id":2,"book":"Exodus","abbreviation":"Ex","osis":"Exod","abbreviations":["exod"]}],"chapter_verses":{"Genesis":[31,25],"Exodus":[22]}}'
small = ToposOptions.create().data(two_books).build()
print([b.name for b in small.books()], [m.passage.reference for m in small.search("Gen 2:4, Exod 1:1, Exod 2:1, John 3:16", OffsetUnit.CHAR)])
# ['Genesis', 'Exodus'] ['Genesis 2:4', 'Exodus 1:1']
print(len(Topos.with_config(two_books).books()))
# 2

# --- Translations

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

named = topos.parse_with_translation("3 John 15 (esv)", OffsetUnit.CHAR, translations, BookStyle.NAME)
print(named.passage.reference, named.translation)
# 3 John 1:15 Translation(name='ESV', start=11, end=14)
print(topos.parse_with_translation("3 John 15 (av)", OffsetUnit.CHAR, translations, BookStyle.NAME))
# None

text = "Ps 22, LXX; John 3:16 AV"
for m in topos.search_with_translations(text, OffsetUnit.CHAR, ToposQuery.create(), translations):
    print(m.reference.passage.reference, m.translation.name, text[m.translation.start : m.translation.end])
# Psalm 22 LXX LXX
# John 3:16 KJV AV
kjv = topos.search_with_translations(text, OffsetUnit.CHAR, ToposQuery.create().translation("KJV"), translations)
print([m.reference.passage.reference for m in kjv])
# ['John 3:16']

typed = "John 3:16 K"
for c in topos.complete_with_translations(typed, len(typed), OffsetUnit.CHAR, ToposFormat.create(), 1, translations):
    print(c.kind.name, c.text, c.start, c.end)
# TRANSLATION KJV 10 11

# --- Errors

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

# --- Files and folders

from topos_bible import ToposFiles

files = ToposFiles.create().cache_dir("cache")
for result in topos.search_files(["notes"], files, ToposQuery.create(), OffsetUnit.CHAR):
    print(result.path, result.error, [(m.passage.reference, m.line, m.column, m.label) for m in result.matches])
# notes/book.epub None [('John 3:16', 1, 5, 'epubcfi(/6/2!/4/2,/1:4,/1:11)'), ('Romans 8:28', 3, 5, 'epubcfi(/6/4!/4/2,/1:4,/1:15)')]
# notes/old/draft.txt None [('Genesis 1:1', 1, 1, None)]
# notes/run.log None [('Mark 1:1', 1, 1, None)]
# notes/sermon.md None [('Romans 8:28', 1, 7, None), ('Psalm 23:1', 2, 6, None)]
# notes/talk.srt None [('John 3:16', 3, 9, '00:00:01.000')]

sermon = topos.search_files(["notes/sermon.md"], files, ToposQuery.create().old_testament(), OffsetUnit.CHAR)[0]
match = sermon.matches[0]
print(match.passage.reference, match.start, match.end, match.line, match.column, match.label, match.epub)
# Psalm 23:1 20 27 2 6 None None
epub = topos.search_files(["notes/book.epub"], files, ToposQuery.create(), OffsetUnit.CHAR)[0].matches[0].epub
print(epub.spine_index, epub.cfi, epub.chapter, repr(epub.line_text))
# 0 epubcfi(/6/2!/4/2,/1:4,/1:11) None 'See Jn 3:16.'
print(topos.search_files(["missing.md"], files, ToposQuery.create(), OffsetUnit.CHAR))
# [FileMatches(path='missing.md', matches=[], error='No such file or directory (os error 2)')]

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

try:
    topos.list_files(["notes"], every.glob("["))
except ToposErrorException as error:
    print(isinstance(error.error, ToposErrorFiles), error.error.message)
# True --glob [: error parsing glob '[': unclosed character class; missing ']'

# --- An index of many files

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

john = topos.parse("John 1:1", BookStyle.NAME)
reference = EpubReference(passage=john, start=0, end=8, line=1, column=1, spine_index=0, chapter="Prologue", cfi="epubcfi(/6/2!/4/2,/1:0,/1:8)", line_text="John 1:1 says")
index.set_epub("notes/other.epub", 2048, 0, time.time() * 1000, [reference])

print(index.paths(), index.file_count(), index.reference_count())
# ['notes/sermon.md', 'notes/old/draft.txt', 'notes/talk.srt', 'notes/book.epub', 'notes/other.epub'] 5 7

results = index.query(topos, ToposQuery.create().new_testament(), IndexScope.ALL, "", IndexOrder.BIBLE)
print(results.len(), results.is_empty(), results.file_count())
# 5 False 4
for hit in index.page(topos, results, 0, 3):
    print(hit.path, hit.passage.reference, hit.start, hit.end, hit.line, hit.column, repr(hit.line_text), hit.epub, hit.pdf)
# notes/other.epub John 1:1 0 8 1 1 'John 1:1 says' IndexEpub(spine_index=0, cfi='epubcfi(/6/2!/4/2,/1:0,/1:8)', chapter='Prologue') None
# notes/book.epub John 3:16 4 11 1 5 'See Jn 3:16.' IndexEpub(spine_index=0, cfi='epubcfi(/6/2!/4/2,/1:4,/1:11)', chapter=None) None
# notes/talk.srt John 3:16 40 49 3 9 '' None None

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

try:
    index.query(topos, ToposQuery.create().translation("ESV"), IndexScope.ALL, "", IndexOrder.FILE)
except ToposErrorException as error:
    print(error.error.message)
# the index doesn't store translations, so it can't filter by them

print(index.is_current(results), index.is_dirty())
# True True
pathlib.Path("index").mkdir()
for file in index.take_dirty_packs() + index.take_unsaved_details():
    pathlib.Path("index", file.name).write_bytes(file.bytes)
print([name for name in sorted(os.listdir("index")) if name.startswith("laptop-")], index.is_dirty())
# ['laptop-epub-10.bin', 'laptop-epub-29.bin', 'laptop-text-14.bin', 'laptop-text-16.bin', 'laptop-text-18.bin'] False
print(index.pack_kind("laptop-epub-10.bin").name, index.pack_kind("notes.bin"), index.kind_of("notes/talk.srt").name, len(index.detail_names()))
# EPUB None TEXT 2

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

everything = phone.query(topos, ToposQuery.create(), IndexScope.ALL, "", IndexOrder.FILE)
print(phone.missing_details(everything, 0, 10) == [phone.missing_file_detail("notes/book.epub")])
# True
for name in phone.missing_details(everything, 0, 10):
    phone.load_detail(name, pathlib.Path("index", name + ".bin").read_bytes())
print([h.line_text for h in phone.file_hits(topos, "notes/book.epub")], phone.missing_file_detail("notes/book.epub"))
# ['See Jn 3:16.', 'And Romans 8:28'] None

phone.rename("notes/old/draft.txt", "notes/draft.txt")
print(phone.is_current(everything), sorted(phone.paths()))
# False ['notes/book.epub', 'notes/draft.txt', 'notes/sermon.md', 'notes/talk.srt']
phone.remove("notes/talk.srt")
phone.retain(["notes/sermon.md", "notes/draft.txt"])
print(sorted(phone.paths()))
# ['notes/draft.txt', 'notes/sermon.md']

try:
    phone.load_pack(b"not a pack")
except ToposErrorException as error:
    print(isinstance(error.error, ToposErrorInvalidIndex), error.error.message)
# True not a topos index file

os.chdir("/")
workspace.cleanup()
