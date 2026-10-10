//! Reading an EPUB's container: the package document `META-INF/container.xml` names, that
//! document's manifest, and any file by its path

use std::{
    fs::File,
    io::{BufReader, Read, Seek},
    path::Path,
};

/// The largest file read, uncompressed: 256 MiB
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// The most read from one container in all, uncompressed, from opening on: 1 GiB
pub const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;

/// The most entries an archive may list: 200,000
pub const MAX_ENTRIES: usize = 200_000;

/// The container file's path, from the archive's root
const CONTAINER_PATH: &str = "META-INF/container.xml";

/// The media type of a package document, as a rootfile names it
const PACKAGE_MEDIA_TYPE: &str = "application/oebps-package+xml";

/// The most memory set aside before an entry's bytes arrive, whatever size it declares
const MAX_FIRST_ALLOCATION: u64 = 1024 * 1024;

/// Why an EPUB's container can't be opened, or a file in it read
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContainerError {
    /// The input can't be read
    Io(String),
    /// It isn't a ZIP archive, or an entry can't be decompressed
    Zip(String),
    /// There is no `META-INF/container.xml`
    NoContainer,
    /// `META-INF/container.xml` isn't well-formed XML
    Container(String),
    /// `META-INF/container.xml` names no package document
    NoRootfile,
    /// The package document it names isn't in the archive
    MissingPackage { path: String },
    /// The package document isn't well-formed XML
    Package(String),
    /// A file is larger than [`MAX_FILE_BYTES`] uncompressed
    TooLarge { path: String },
    /// Reading this file would take what's been read from the container past
    /// [`MAX_TOTAL_BYTES`]
    TooMuchData { path: String },
    /// The archive lists more than [`MAX_ENTRIES`] entries
    TooManyEntries { count: usize },
}

impl std::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContainerError::Io(message) => write!(f, "the input can't be read: {message}"),
            ContainerError::Zip(message) => {
                write!(f, "the ZIP archive can't be read: {message}")
            }
            ContainerError::NoContainer => write!(f, "no {CONTAINER_PATH}"),
            ContainerError::Container(message) => {
                write!(f, "{CONTAINER_PATH} isn't well-formed XML: {message}")
            }
            ContainerError::NoRootfile => write!(f, "{CONTAINER_PATH} names no package document"),
            ContainerError::MissingPackage { path } => {
                write!(f, "the package document {path} is missing")
            }
            ContainerError::Package(message) => {
                write!(f, "the package document isn't well-formed XML: {message}")
            }
            ContainerError::TooLarge { path } => {
                write!(f, "{path} is larger than {MAX_FILE_BYTES} bytes")
            }
            ContainerError::TooMuchData { path } => write!(
                f,
                "reading {path} would take more than {MAX_TOTAL_BYTES} bytes from this archive in all"
            ),
            ContainerError::TooManyEntries { count } => {
                write!(
                    f,
                    "the archive has {count} entries, more than {MAX_ENTRIES}"
                )
            }
        }
    }
}

impl std::error::Error for ContainerError {}

/// An item of the package document's manifest
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestItem {
    /// Its `id`
    pub id: String,
    /// Its `href` as written
    pub href: String,
    /// The file it names (`OEBPS/text/ch1.xhtml`), or none when it names something outside the
    /// container (a URL with a scheme)
    pub path: Option<String>,
    /// Its `media-type`, as written (whitespace trimmed); empty when missing
    pub media_type: String,
    /// Its `properties` (EPUB 3), in order
    pub properties: Vec<String>,
}

impl ManifestItem {
    /// Whether its `properties` include `property` (`nav`, `cover-image`, …)
    pub fn has_property(&self, property: &str) -> bool {
        self.properties.iter().any(|own| own == property)
    }
}

/// The path a URL string names, relative to `base` (a folder ending in `/`, or empty for the
/// root), or none when it names something outside the container: the rules the manifest's
/// `href`s are read by
pub fn resolve(base: &str, url: &str) -> Option<String> {
    // The URL Standard removes leading and trailing spaces and C0 controls, and tabs and
    // newlines anywhere, before reading a URL string.
    let cleaned: String = url
        .trim_matches(|c: char| c <= ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();

    if has_scheme(&cleaned) || cleaned.starts_with("//") {
        return None;
    }

    // The fragment goes first, then the query.
    let mut rest = cleaned.as_str();
    if let Some(at) = rest.find('#') {
        rest = &rest[..at];
    }
    if let Some(at) = rest.find('?') {
        rest = &rest[..at];
    }

    let rest = rest.replace('\\', "/");
    // Backslashes read as slashes can make a network path too.
    if rest.starts_with("//") {
        return None;
    }

    let joined = if let Some(from_root) = rest.strip_prefix('/') {
        from_root.to_string()
    } else if base.is_empty() {
        rest
    } else {
        let base = base.replace('\\', "/");
        if base.ends_with('/') {
            format!("{base}{rest}")
        } else {
            format!("{base}/{rest}")
        }
    };

    let raw: Vec<&str> = joined.split('/').collect();
    // As in RFC 3986's removing of dot segments, a path ending in `/`, `.`, or `..` is a
    // folder.
    let folder = matches!(raw.last(), Some(&"") | Some(&".") | Some(&".."));
    let mut segments: Vec<&str> = Vec::new();
    for segment in raw {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            name => segments.push(name),
        }
    }

    let mut path = segments
        .iter()
        .map(|segment| percent_decode(segment))
        .collect::<Vec<_>>()
        .join("/");
    if folder && !path.is_empty() {
        path.push('/');
    }
    Some(path)
}

/// Whether a URL string starts with a scheme: an ASCII letter, then letters, digits, `+`, `-`,
/// or `.`, then `:`
fn has_scheme(url: &str) -> bool {
    let mut chars = url.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    for c in chars {
        if c == ':' {
            return true;
        }
        if !(c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
            return false;
        }
    }
    false
}

/// One path segment with its percent-escapes decoded, or as it was when the decoded bytes
/// aren't UTF-8
fn percent_decode(segment: &str) -> String {
    let bytes = segment.as_bytes();
    if !bytes.contains(&b'%') {
        return segment.to_string();
    }
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        if byte == b'%'
            && let (Some(high), Some(low)) = (
                bytes.get(at + 1).and_then(|b| hex_value(*b)),
                bytes.get(at + 2).and_then(|b| hex_value(*b)),
            )
        {
            decoded.push(high * 16 + low);
            at += 3;
            continue;
        }
        decoded.push(byte);
        at += 1;
    }
    match String::from_utf8(decoded) {
        Ok(text) => text,
        Err(_) => segment.to_string(),
    }
}

/// The value of an ASCII hex digit
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Text from a file's bytes: UTF-8 or UTF-16 by its first bytes, invalid sequences replaced
fn decode_text(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(rest).into_owned()
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        decode_utf16(rest, false)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        decode_utf16(rest, true)
    } else if bytes.starts_with(&[0x3C, 0x00, 0x3F, 0x00]) {
        decode_utf16(bytes, false)
    } else if bytes.starts_with(&[0x00, 0x3C, 0x00, 0x3F]) {
        decode_utf16(bytes, true)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// UTF-16 text, invalid sequences and an odd final byte replaced
fn decode_utf16(bytes: &[u8], big_endian: bool) -> String {
    let pairs = bytes.chunks_exact(2);
    let odd = !pairs.remainder().is_empty();
    let units = pairs.map(|pair| {
        let pair = [pair.first().copied(), pair.get(1).copied()];
        let pair = [pair[0].unwrap_or(0), pair[1].unwrap_or(0)];
        if big_endian {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        }
    });
    let mut text: String = char::decode_utf16(units)
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect();
    if odd {
        text.push(char::REPLACEMENT_CHARACTER);
    }
    text
}

/// XML parsing that allows a document type declaration
fn parse_xml(text: &str) -> Result<roxmltree::Document<'_>, roxmltree::Error> {
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..roxmltree::ParsingOptions::default()
    };
    roxmltree::Document::parse_with_options(text, options)
}

/// An element's attribute with this name and no namespace (written without a prefix)
fn attribute<'a>(node: roxmltree::Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attributes()
        .find(|attribute| attribute.namespace().is_none() && attribute.name() == name)
        .map(|attribute| attribute.value())
}

/// The package document's path from `container.xml`'s text
fn find_package_path(text: &str) -> Result<String, ContainerError> {
    let document = parse_xml(text).map_err(|e| ContainerError::Container(e.to_string()))?;
    let mut fallback = None;
    for node in document.descendants() {
        if !node.is_element() || node.tag_name().name() != "rootfile" {
            continue;
        }
        let Some(full_path) = attribute(node, "full-path").filter(|path| !path.is_empty()) else {
            continue;
        };
        let Some(path) = resolve("", full_path) else {
            continue;
        };
        let is_package = attribute(node, "media-type").is_some_and(|media_type| {
            media_type
                .trim_matches(|c: char| c.is_ascii_whitespace())
                .eq_ignore_ascii_case(PACKAGE_MEDIA_TYPE)
        });
        if is_package {
            return Ok(path);
        }
        if fallback.is_none() {
            fallback = Some(path);
        }
    }
    fallback.ok_or(ContainerError::NoRootfile)
}

/// The manifest's items from the package document's text
fn read_manifest(text: &str, folder: &str) -> Result<Vec<ManifestItem>, ContainerError> {
    let document = parse_xml(text).map_err(|e| ContainerError::Package(e.to_string()))?;
    let Some(manifest) = document
        .root_element()
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "manifest")
    else {
        return Ok(Vec::new());
    };
    let items = manifest
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "item")
        .filter_map(|node| {
            let id = attribute(node, "id").filter(|id| !id.is_empty())?;
            let href = attribute(node, "href")?;
            Some(ManifestItem {
                id: id.to_string(),
                href: href.to_string(),
                path: resolve(folder, href),
                media_type: attribute(node, "media-type")
                    .unwrap_or("")
                    .trim_matches(|c: char| c.is_ascii_whitespace())
                    .to_string(),
                properties: attribute(node, "properties")
                    .unwrap_or("")
                    .split_ascii_whitespace()
                    .map(str::to_string)
                    .collect(),
            })
        })
        .collect();
    Ok(items)
}

/// How much a container reads: the constants, or smaller ones in tests
#[derive(Debug, Clone, Copy)]
struct Limits {
    /// The largest file, uncompressed
    file: u64,
    /// The most read in all, uncompressed
    total: u64,
    /// The most entries the archive may list
    entries: usize,
}

impl Limits {
    const DEFAULT: Limits = Limits {
        file: MAX_FILE_BYTES,
        total: MAX_TOTAL_BYTES,
        entries: MAX_ENTRIES,
    };
}

/// What a container may still read, and how
struct Budget {
    limits: Limits,
    /// Bytes decompressed so far
    used: u64,
}

/// An archive entry: its name with `\` read as `/`, and its index in the archive
struct Entry {
    name: String,
    index: usize,
}

impl Entry {
    /// Whether it's a file: not a folder (a name ending in `/`), and with a name
    fn is_file(&self) -> bool {
        !self.name.is_empty() && !self.name.ends_with('/')
    }
}

/**
An EPUB's container, opened: its package document and manifest are read; other files are read
when asked for

```no_run
use topos_bible_formats::epub::container::Container;

let mut book = Container::open("book.epub".as_ref())?;
let nav = book.manifest().iter().find(|item| item.has_property("nav")).cloned();
if let Some(path) = nav.and_then(|item| item.path) {
    let text = book.read_text(&path)?;
}
# Ok::<(), topos_bible_formats::epub::container::ContainerError>(())
```
*/
pub struct Container<R> {
    archive: zip::ZipArchive<R>,
    entries: Vec<Entry>,
    package_path: String,
    package_text: String,
    package_folder: String,
    manifest: Vec<ManifestItem>,
    budget: Budget,
}

impl Container<BufReader<File>> {
    /// Opens an EPUB file
    pub fn open(path: &Path) -> Result<Self, ContainerError> {
        let file = File::open(path).map_err(|e| ContainerError::Io(e.to_string()))?;
        Self::from_reader(BufReader::new(file))
    }
}

impl<R: Read + Seek> Container<R> {
    /// Opens an EPUB from any reader (bytes in memory: `std::io::Cursor`)
    pub fn from_reader(reader: R) -> Result<Self, ContainerError> {
        Self::with_limits(reader, Limits::DEFAULT)
    }

    /// Opens an EPUB, reading no more than `limits` allow
    fn with_limits(reader: R, limits: Limits) -> Result<Self, ContainerError> {
        let mut archive = zip::ZipArchive::new(reader).map_err(zip_error)?;
        if archive.len() > limits.entries {
            return Err(ContainerError::TooManyEntries {
                count: archive.len(),
            });
        }
        let mut budget = Budget { limits, used: 0 };
        let entries = (0..archive.len())
            .filter_map(|index| {
                archive.name_for_index(index).map(|name| Entry {
                    name: name.replace('\\', "/"),
                    index,
                })
            })
            .collect::<Vec<_>>();

        let container_text = match find_entry(&entries, CONTAINER_PATH) {
            Some(index) => decode_text(&read_entry(
                &mut archive,
                index,
                CONTAINER_PATH,
                &mut budget,
            )?),
            None => return Err(ContainerError::NoContainer),
        };
        let package_path = find_package_path(&container_text)?;
        let package_text = match find_entry(&entries, &package_path) {
            Some(index) => decode_text(&read_entry(
                &mut archive,
                index,
                &package_path,
                &mut budget,
            )?),
            None => return Err(ContainerError::MissingPackage { path: package_path }),
        };
        let package_folder = match package_path.rfind('/') {
            Some(at) => package_path[..=at].to_string(),
            None => String::new(),
        };
        let manifest = read_manifest(&package_text, &package_folder)?;

        Ok(Container {
            archive,
            entries,
            package_path,
            package_text,
            package_folder,
            manifest,
            budget,
        })
    }

    /// The package document's path (`OEBPS/content.opf`)
    pub fn package_path(&self) -> &str {
        &self.package_path
    }

    /// The package document's text
    pub fn package_text(&self) -> &str {
        &self.package_text
    }

    /// The folder the package document is in, which its `href`s are relative to (`OEBPS/`, or
    /// empty at the root)
    pub fn package_folder(&self) -> &str {
        &self.package_folder
    }

    /// The manifest's items, in order
    pub fn manifest(&self) -> &[ManifestItem] {
        &self.manifest
    }

    /// The first manifest item with this `id`
    pub fn item(&self, id: &str) -> Option<&ManifestItem> {
        self.manifest.iter().find(|item| item.id == id)
    }

    /// Every file's path in the archive, in the archive's order
    pub fn files(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|entry| entry.is_file())
            .map(|entry| entry.name.clone())
            .collect()
    }

    /// A file's bytes, or none when there's no such file
    pub fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, ContainerError> {
        match find_entry(&self.entries, path) {
            Some(index) => read_entry(&mut self.archive, index, path, &mut self.budget).map(Some),
            None => Ok(None),
        }
    }

    /// A file's text (UTF-8 or UTF-16, by its first bytes), or none when there's no such file
    pub fn read_text(&mut self, path: &str) -> Result<Option<String>, ContainerError> {
        Ok(self.read(path)?.map(|bytes| decode_text(&bytes)))
    }
}

/// A ZIP error as this reader's
fn zip_error(error: zip::result::ZipError) -> ContainerError {
    ContainerError::Zip(error.to_string())
}

/// The archive index of the file a path names: the entry with exactly that name, else the one
/// entry whose name matches it ignoring ASCII case
fn find_entry(entries: &[Entry], path: &str) -> Option<usize> {
    if path.is_empty() || path.ends_with('/') {
        return None;
    }
    if let Some(entry) = entries.iter().find(|entry| entry.name == path) {
        return Some(entry.index);
    }
    let mut found: Option<&Entry> = None;
    for entry in entries {
        if !entry.is_file() || !entry.name.eq_ignore_ascii_case(path) {
            continue;
        }
        match found {
            // The same name twice is still one name; the first entry is read, as for an
            // exact match.
            Some(first) if first.name == entry.name => {}
            Some(_) => return None,
            None => found = Some(entry),
        }
    }
    found.map(|entry| entry.index)
}

/// An entry's uncompressed bytes, at most the file limit of them, and no more than the budget
/// has left; what's decompressed is taken from the budget
fn read_entry<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    index: usize,
    path: &str,
    budget: &mut Budget,
) -> Result<Vec<u8>, ContainerError> {
    let too_large = || ContainerError::TooLarge {
        path: path.to_string(),
    };
    let too_much = || ContainerError::TooMuchData {
        path: path.to_string(),
    };
    let undecompressable = |e: std::io::Error| ContainerError::Zip(format!("{path}: {e}"));
    let mut file = archive
        .by_index(index)
        .map_err(|e| ContainerError::Zip(format!("{path}: {e}")))?;
    let left = budget.limits.total.saturating_sub(budget.used);
    if file.size() > budget.limits.file {
        return Err(too_large());
    }
    if file.size() > left {
        return Err(too_much());
    }
    // Whichever limit comes first stops the reading; at a tie, it's the file's own.
    let file_limit_first = budget.limits.file <= left;
    let cap = budget.limits.file.min(left);
    // The declared size is only a claim, so the bytes produced are counted, and no more than
    // a little memory is set aside on its word.
    let capacity = usize::try_from(file.size().min(MAX_FIRST_ALLOCATION)).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    let read = (&mut file).take(cap).read_to_end(&mut bytes);
    budget.used = budget.used.saturating_add(bytes.len() as u64);
    read.map_err(undecompressable)?;
    // Up to the cap, the file may still go on: one more byte is over a limit. (Reading the cap
    // and then one byte apart, rather than the cap and one at once, keeps the buffer from
    // doubling past it.)
    let mut probe = [0u8; 1];
    loop {
        match file.read(&mut probe) {
            Ok(0) => return Ok(bytes),
            Ok(_) => {
                budget.used = budget.used.saturating_add(1);
                return Err(if file_limit_first {
                    too_large()
                } else {
                    too_much()
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(undecompressable(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    /// An EPUB's bytes: its container and package document, then `files` (name, bytes), stored
    fn epub(files: &[(String, Vec<u8>)]) -> Vec<u8> {
        epub_with(files, zip::CompressionMethod::Stored)
    }

    /// An EPUB's bytes, every entry compressed with `method`
    fn epub_with(files: &[(String, Vec<u8>)], method: zip::CompressionMethod) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let stored = zip::write::SimpleFileOptions::default().compression_method(method);
        let mut add = |name: &str, bytes: &[u8]| {
            zip.start_file(name, stored).expect("an entry");
            zip.write_all(bytes).expect("its bytes");
        };
        add(
            CONTAINER_PATH,
            br#"<container><rootfiles><rootfile full-path="content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#,
        );
        add("content.opf", b"<package><manifest/></package>");
        for (name, bytes) in files {
            add(name, bytes);
        }
        zip.finish().expect("an archive").into_inner()
    }

    /// `count` files of `size` bytes each
    fn files(count: usize, size: usize) -> Vec<(String, Vec<u8>)> {
        (0..count)
            .map(|i| (format!("f{i}.xhtml"), vec![b'a'; size]))
            .collect()
    }

    /// What opening and reading take: the container and package document's bytes
    fn opening_bytes() -> u64 {
        let book = Container::from_reader(Cursor::new(epub(&[]))).expect("opens");
        book.budget.used
    }

    fn limits(file: u64, total: u64, entries: usize) -> Limits {
        Limits {
            file,
            total,
            entries,
        }
    }

    #[test]
    fn the_limits_are_as_specified() {
        assert_eq!(MAX_FILE_BYTES, 256 << 20);
        assert_eq!(MAX_TOTAL_BYTES, 1 << 30);
        assert_eq!(MAX_ENTRIES, 200_000);
    }

    #[test]
    fn too_many_entries_cant_be_opened() {
        // Two entries for the container and package document, and eight files
        let bytes = epub(&files(8, 1));
        assert!(Container::with_limits(Cursor::new(&bytes), limits(1000, 1000, 10)).is_ok());
        let error = Container::with_limits(Cursor::new(&bytes), limits(1000, 1000, 9))
            .err()
            .expect("too many entries");
        assert_eq!(error, ContainerError::TooManyEntries { count: 10 });
        assert_eq!(
            error.to_string(),
            "the archive has 10 entries, more than 200000"
        );
    }

    #[test]
    fn many_small_files_stop_at_the_total() {
        let opening = opening_bytes();
        // Ten files of 100 bytes, each under the file limit, with room for five in all
        let bytes = epub(&files(10, 100));
        let mut book =
            Container::with_limits(Cursor::new(&bytes), limits(1000, opening + 500, 1000))
                .expect("opens");
        for i in 0..5 {
            let read = book.read(&format!("f{i}.xhtml")).expect("in the budget");
            assert_eq!(read.map(|bytes| bytes.len()), Some(100));
        }
        let error = book.read("f5.xhtml").expect_err("over the total");
        assert_eq!(
            error,
            ContainerError::TooMuchData {
                path: "f5.xhtml".into()
            }
        );
        assert_eq!(
            error.to_string(),
            "reading f5.xhtml would take more than 1073741824 bytes from this archive in all"
        );
        // A file that isn't there is still none
        assert_eq!(book.read("missing.xhtml"), Ok(None));
    }

    #[test]
    fn reading_one_file_again_counts_again() {
        let opening = opening_bytes();
        let bytes = epub(&files(1, 100));
        let mut book =
            Container::with_limits(Cursor::new(&bytes), limits(1000, opening + 250, 1000))
                .expect("opens");
        assert!(book.read("f0.xhtml").is_ok());
        assert!(book.read_text("f0.xhtml").is_ok());
        assert_eq!(
            book.read("f0.xhtml"),
            Err(ContainerError::TooMuchData {
                path: "f0.xhtml".into()
            })
        );
    }

    #[test]
    fn an_empty_file_reads_with_nothing_left() {
        let opening = opening_bytes();
        let mut list = files(1, 100);
        list.push(("empty.xhtml".into(), Vec::new()));
        let bytes = epub(&list);
        let mut book =
            Container::with_limits(Cursor::new(&bytes), limits(1000, opening + 100, 1000))
                .expect("opens");
        assert!(book.read("f0.xhtml").is_ok());
        assert_eq!(book.read("empty.xhtml"), Ok(Some(Vec::new())));
    }

    #[test]
    fn the_file_limit_is_reported_over_both() {
        let bytes = epub(&files(1, 1000));
        let mut book =
            Container::with_limits(Cursor::new(&bytes), limits(500, opening_bytes() + 10, 1000))
                .expect("opens");
        assert_eq!(
            book.read("f0.xhtml"),
            Err(ContainerError::TooLarge {
                path: "f0.xhtml".into()
            })
        );
    }

    #[test]
    fn opening_counts_toward_the_total() {
        let bytes = epub(&[]);
        let error = Container::with_limits(Cursor::new(&bytes), limits(1000, 10, 1000))
            .err()
            .expect("container.xml is over the total");
        assert_eq!(
            error,
            ContainerError::TooMuchData {
                path: CONTAINER_PATH.into()
            }
        );
    }

    #[test]
    fn a_declared_size_over_whats_left_reads_nothing() {
        let opening = opening_bytes();
        let bytes = epub(&files(1, 100));
        let mut book =
            Container::with_limits(Cursor::new(&bytes), limits(1000, opening + 99, 1000))
                .expect("opens");
        assert!(matches!(
            book.read("f0.xhtml"),
            Err(ContainerError::TooMuchData { .. })
        ));
        assert_eq!(book.budget.used, opening);
    }

    /// A deflated entry that declares fewer bytes than it inflates to (its headers changed after
    /// writing) is stopped by what it produces, which is counted
    #[test]
    fn bytes_produced_are_counted_whatever_the_declared_size() {
        let opening = opening_bytes();
        let mut bytes = epub_with(
            &[("bomb.xhtml".into(), vec![b'a'; 1000])],
            zip::CompressionMethod::Deflated,
        );
        // Every header's uncompressed size of 1000 becomes 10: the local header's at offset 22,
        // the central directory's at offset 24
        let name = b"bomb.xhtml";
        let mut patched = 0;
        let mut at = 0;
        while let Some(found) = bytes[at..].windows(name.len()).position(|w| w == name) {
            let start = at + found;
            for (signature, offset, name_at) in [(*b"PK\x03\x04", 22, 30), (*b"PK\x01\x02", 24, 46)]
            {
                if start >= name_at && bytes[start - name_at..start - name_at + 4] == signature {
                    let size = start - name_at + offset;
                    assert_eq!(bytes[size..size + 4], 1000u32.to_le_bytes());
                    bytes[size..size + 4].copy_from_slice(&10u32.to_le_bytes());
                    patched += 1;
                }
            }
            at = start + name.len();
        }
        assert_eq!(patched, 2);
        let mut book =
            Container::with_limits(Cursor::new(&bytes), limits(1000, opening + 500, 1000))
                .expect("opens");
        assert_eq!(
            book.read("bomb.xhtml"),
            Err(ContainerError::TooMuchData {
                path: "bomb.xhtml".into()
            })
        );
        assert_eq!(book.budget.used, opening + 501);
    }
}
