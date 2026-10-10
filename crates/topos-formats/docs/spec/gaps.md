# Gaps: questions the specification left open, and how they were decided

Written by the implementer. Each question is one `epub-container.md` doesn't settle; each decision
is justified from the standards (EPUB 3.3, OCF 2.0.1, XML 1.0, the WHATWG URL Standard, RFC 3986)
or from the specification's own reasoning, never from another EPUB reader.

## Finding the package document

1. **A `rootfile` of the right media type whose `full-path` is missing, empty, or has a scheme.**
   Section 2 says the first `rootfile` with the package media type is used, and later speaks of
   a "usable" `rootfile`. *Decision:* a `rootfile` is usable only when its `full-path` is
   present, non-empty, and names a path in the container (section 4: no scheme, not a network
   path). Unusable ones are passed over, both in the search for the package media type and in the
   fallback. EPUB 3.3 §4.2.6.3.1.3 requires `full-path` and requires it to be a
   path-relative-scheme-less URL string, so one that isn't can't name the package document.
2. **A `full-path` that resolves to a folder** (`OEBPS/`, or `OEBPS/..`). *Decision:* it's
   usable, and opening fails with `MissingPackage` naming that path: a folder names no file
   (section 5).
3. **How `META-INF/container.xml` itself is found and read.** *Decision:* by the same rules as any
   file (section 5: `\` read as `/`, then a single ASCII-case-insensitive match), decoded as
   section 5 says (so UTF-16 works; XML 1.0 Appendix F and EPUB 3.3 §3.9 allow it), and parsed
   allowing a DTD, as the package document is.
4. **Attributes in `container.xml`.** Section 3 says package-document attributes are matched by
   local name with no namespace; section 2 is silent. *Decision:* the same: `full-path` and
   `media-type` are the unprefixed attributes (in XML Namespaces, an unprefixed attribute has no
   namespace, and OCF defines these attributes that way).

## Paths

5. **Whitespace around a URL string.** *Decision:* leading and trailing C0 controls and spaces are
   removed, and ASCII tab, LF, and CR anywhere, before anything else, as the WHATWG URL
   Standard's basic URL parser does. Spaces inside are kept (and are part of the file name).
6. **Backslashes that make a network path** (`\\server\a.css`, `/\server`). *Decision:* after
   step 3 turns `\` into `/`, a string starting with `//` has no path, the same as one written
   with `//`. The URL Standard reads `\` as `/` in such URLs, so these are network paths too.
7. **A final `.` or `..` segment.** Step 5 keeps a trailing `/` but doesn't say what `a/.` or
   `a/..` end in. *Decision:* a folder (a trailing `/`), as RFC 3986 §5.2.4 gives (`a/b/..`
   becomes `a/`). When nothing is left (the root, as from `..` with the empty base), the path is
   the empty string, with no `/`.
8. **A base without its final `/`.** `resolve`'s documentation says the base is a folder ending
   in `/`. *Decision:* a non-empty base not ending in `/` is still read as a folder (a `/` is
   added), not as a file whose last segment is replaced, since the parameter is documented as a
   folder. `\` in a base is read as `/`.

## Files

9. **Two entries with the same name (in the same case).** ZIP allows it; section 5 says a path
   names *the* entry with that name. *Decision:* the first in the archive's order is read, and
   for the case-insensitive fallback, entries with exactly the same name count as one name (only
   different names make the match ambiguous). Folders never take part in the fallback.
10. **An entry with an empty name.** *Decision:* it's not a file (it has no file path, EPUB 3.3
    §4.2.4), so `files()` leaves it out; the empty path already names no file.
11. **ZIP errors while opening.** *Decision:* every error the `zip` crate gives while reading the
    archive's directory is `Zip` (an input that isn't a readable ZIP archive, including a
    truncated one); `Io` is for an input that can't be read at all (`Container::open` failing to
    open the file). Some entries the `zip` crate refuses already while reading the directory (an
    AES-encrypted entry's compression method, 99), which makes opening fail with `Zip`; other
    unreadable methods (as bzip2 in this build) fail only when that file is read.
12. **Messages.** Section 7 gives three. *Decision:* the others follow the same style:
    `the input can't be read: …`, `the ZIP archive can't be read: …` (an entry's failure names
    its path first: `OEBPS/a.xhtml: …`), `META-INF/container.xml isn't well-formed XML: …`,
    `META-INF/container.xml names no package document`, and
    `the package document isn't well-formed XML: …`.
13. **The size limit when a declared size is already over it.** *Decision:* `TooLarge` at once,
    before decompressing (section 1: the limit is on the uncompressed size; the declared size is
    one statement of it). The bytes produced are counted as well, whatever the declared size.
