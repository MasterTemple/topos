# The EPUB container reader

What `topos_bible_formats::epub::container` does: it opens an EPUB's ZIP container, finds the
package document the container names, reads that document's manifest, and reads files by their
paths. Its interface is in [api.md](api.md). This file says how it behaves; where the standards
leave a choice, the choice is written here and marked **Choice**.

Standards cited:

- **EPUB 3.3**: W3C Recommendation, <https://www.w3.org/TR/epub-33/> (section numbers below are its)
- **OPF 2.0.1** and **OCF 2.0.1**: IDPF, the EPUB 2 package and container formats (EPUB 2 books
  are still common and must be read the same way)
- **ZIP**: PKWARE's APPNOTE.TXT
- **URL**: the WHATWG URL Standard, and RFC 3986 §5.2.4 (removing dot segments)

## 1. The ZIP archive

The input is a ZIP archive (EPUB 3.3 §4.3). Its entries are read with the `zip` crate.

- Stored and Deflate entries must be readable (§4.3.2). **Choice:** other methods the `zip` crate
  is built with may be read too; one it can't read makes reading that file an error.
- An input that isn't a ZIP archive is `ContainerError::Zip`.
- **Entry names.** File names are UTF-8 (§4.3.2) and case-sensitive (§4.2.3). **Choice:** a
  backslash (`\`) in an entry's name is read as `/` (some tools write Windows separators). Entries
  that are folders (names ending in `/`) are not files.
- The `mimetype` file is not checked (**Choice:** books with a missing, compressed, or misplaced
  `mimetype` are common and readable).
- **Size limit.** **Choice:** a file whose uncompressed size is more than `MAX_FILE_BYTES`
  (256 MiB) can't be read: `ContainerError::TooLarge`. The limit applies to the bytes actually
  produced while decompressing, not only to the size the archive declares, so a file that
  declares a small size but inflates further is cut off and is the same error.

## 2. Finding the package document

`META-INF/container.xml` (EPUB 3.3 §4.2.6.3.1; OCF 2.0.1 §3.5.1) names the package documents.

- If the archive has no `META-INF/container.xml` file, opening is `ContainerError::NoContainer`.
- It is parsed as XML. **Choice:** elements are matched by their local name only (`container`,
  `rootfiles`, `rootfile`), whatever their namespace or prefix, since books sometimes get the
  namespace wrong. A `container.xml` that isn't well-formed XML is `ContainerError::Container`.
- The package document is the `full-path` of the first `rootfile` (in document order, anywhere in
  the file) whose `media-type` is `application/oebps-package+xml` (§4.2.6.3.1.3). **Choice:**
  EPUB 3.3 doesn't say which of several renditions to use; the first is the default rendition in
  EPUB 3's Multiple-Rendition Publications spec and in OCF 2.0.1, so the first is used. If no
  `rootfile` has that media type, the first `rootfile` with a non-empty `full-path` is used.
  **Choice:** the media type is compared ignoring ASCII case and surrounding whitespace.
- No usable `rootfile` is `ContainerError::NoRootfile`.
- `full-path` is a path-relative-scheme-less URL string relative to the container's root
  (§4.2.6.3.1.3). It becomes a path (section 4) resolved against the root (an empty base).
- If the archive has no file at that path, opening is `ContainerError::MissingPackage` (with the
  path).

## 3. The package document

The package document is XML (EPUB 3.3 §5; OPF 2.0.1 §2) in UTF-8 or UTF-16 (§3.9).

- **Decoding** follows section 5 (text).
- It is parsed as XML 1.0, allowing a DTD (EPUB 2 package documents sometimes have a
  `<!DOCTYPE>`). One that isn't well-formed XML is `ContainerError::Package`.
- **Choice:** elements are matched by local name (`package`, `manifest`, `item`), whatever their
  namespace or prefix (EPUB 2 books sometimes write `<opf:item>`). The manifest is the first
  `manifest` element that is a child of the root element; if there is none, it has no items. Its
  items are its `item` children, in document order.
- Each item's attributes (EPUB 3.3 §5.6.2; OPF 2.0.1 §2.3):
  - `id` (required). **Choice:** an item without an `id`, or with an empty one, is left out.
  - `href` (required). **Choice:** an item without an `href` is left out. An empty `href` is kept
    and resolves to the package document's own folder, which is no file (its `path` is that
    folder's path with a trailing `/`, see section 4; reading it gives nothing).
  - `media-type` (required). **Choice:** missing is the empty string; kept as written (not
    lowercased), with surrounding whitespace removed.
  - `properties` (optional, EPUB 3): a list separated by ASCII whitespace; missing is empty.
  - Other attributes (`fallback`, `media-overlay`, …) are not read.
  - Attributes are matched by local name with no namespace (an unprefixed attribute).
- **Choice:** two items with the same `id` (not allowed, §5.3.3) are both kept in the list, in
  order; looking an item up by id finds the first.
- The package document's own text is kept, so callers can read the parts of it this reader
  doesn't (the spine, metadata) without reading it again.

## 4. Paths

A **path** here is the file path of a file in the container (§4.2.4): its names from the root,
joined by `/`, with no leading `/`. Paths are compared exactly (case-sensitively).

A URL string from the container (`full-path`) or the package document (`href`) becomes a path,
relative to a **base folder** (the empty string for the container's root; for the package
document, the folder its path is in: everything up to and including its last `/`, or empty).

1. **Not a path.** If the string has a scheme (it starts with an ASCII letter followed by any of
   ASCII letters, digits, `+`, `-`, `.`, then `:`), it doesn't name a file in the container
   (`http:`, `https:`, `data:`, `mailto:`, …): it has no path. **Choice:** a string starting with
   `//` (a network path) has no path either.
2. **Fragment and query.** Everything from the first `#` is removed, then everything from the
   first `?`.
3. **Separators.** **Choice:** each `\` is read as `/`.
4. **Joining.** If the string starts with `/`, it is relative to the root (**Choice:** disallowed
   by §4.2.5's note, but its meaning is clear) and the base is ignored; otherwise it is appended to
   the base folder.
5. **Dot segments.** Split on `/`. A segment `.` is removed. A segment `..` removes the segment
   before it, if any; at the root it is removed (§4.2.5: `..` beyond the root stays at the root).
   Empty segments (from `//`) are removed, except that a trailing `/` is kept as a trailing `/` (a
   folder).
6. **Percent-decoding.** Each segment is percent-decoded: every `%` followed by two hex digits
   becomes that byte; any other `%` stays as it is. The bytes are read as UTF-8; **Choice:** if
   they aren't valid UTF-8, the segment is kept undecoded. Decoding happens after step 5, so `%2E%2E`
   is a file named `..`, not a step up. A decoded `%2F` stays inside its segment (the result can
   contain a `/` that wasn't a separator; that file almost certainly doesn't exist, which is fine).
7. The path is the segments joined by `/`.

Examples (base folder `OEBPS/`):

| String | Path |
|---|---|
| `text/ch1.xhtml` | `OEBPS/text/ch1.xhtml` |
| `text/ch1.xhtml#p3` | `OEBPS/text/ch1.xhtml` |
| `../Images/cover.jpg` | `Images/cover.jpg` |
| `../../../x.css` | `x.css` |
| `./a/./b/../c.xhtml` | `OEBPS/a/c.xhtml` |
| `Chapter%201.xhtml` | `OEBPS/Chapter 1.xhtml` |
| `caf%C3%A9.xhtml` | `OEBPS/café.xhtml` |
| `100%.xhtml` | `OEBPS/100%.xhtml` |
| `text\ch1.xhtml` | `OEBPS/text/ch1.xhtml` |
| `/OEBPS/toc.ncx` | `OEBPS/toc.ncx` |
| `https://example.org/a.mp3` | none |
| `data:image/png;base64,AAAA` | none |
| `` (empty) | `OEBPS/` |

With the empty base, `content.opf` is `content.opf` and `OEBPS/content.opf` is itself.

## 5. Reading files and text

- **Finding a file.** A path names the archive entry whose name (with `\` read as `/`, section 1)
  is exactly the path. **Choice:** if no entry's name is exactly the path, an entry whose name
  equals it ignoring ASCII case is used, if there is exactly one such entry (books made on
  case-insensitive file systems often get case wrong in their hrefs); otherwise there is no such
  file. A path ending in `/`, or the empty path, names no file.
- **Bytes.** Reading a file gives its uncompressed bytes. No such file is `Ok(None)`. A file that
  can't be decompressed, or is too large (section 1), is an error.
- **Text** (`read_text`, and the package document): the bytes are decoded as XML says
  (XML 1.0 Appendix F, restricted to what EPUB allows, §3.9):
  - starting with the UTF-8 byte order mark `EF BB BF`: UTF-8, without the mark;
  - starting with `FF FE`: UTF-16 little-endian, without the mark;
  - starting with `FE FF`: UTF-16 big-endian, without the mark;
  - **Choice:** with no mark, starting with `3C 00 3F 00` (`<?` in UTF-16LE): UTF-16LE; with
    `00 3C 00 3F`: UTF-16BE;
  - otherwise UTF-8.

  **Choice:** invalid sequences become U+FFFD (lossy), so a file with a stray bad byte is still
  read. An odd final byte in UTF-16 becomes U+FFFD. The declared `encoding` in the XML declaration
  is not consulted (EPUB allows only UTF-8 and UTF-16, which the bytes show).

## 6. Opening

Opening reads the archive's list of entries, `META-INF/container.xml`, and the package document
(sections 2 and 3), in that order, and fails with the first error. After opening, the package
document's path, text, and manifest are available without further reading; other files are read
on request.

## 7. Errors

`ContainerError` (api.md) has one variant per failure above. Its messages (`Display`) are
lowercase sentences without a final period, naming the path when there is one, for example
`no META-INF/container.xml`, `the package document OEBPS/content.opf is missing`,
`OEBPS/big.xhtml is larger than 268435456 bytes`. Errors from the `zip` crate and the XML parser
are carried as text.
