//! Reading an EPUB's container: the package document `META-INF/container.xml` names, that
//! document's manifest, and any file by its path

use std::{
    fs::File,
    io::{BufReader, Read, Seek},
    path::Path,
};

/// The largest file read, uncompressed: 256 MiB
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

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
        let mut archive = zip::ZipArchive::new(reader).map_err(zip_error)?;
        let entries = (0..archive.len())
            .filter_map(|index| {
                archive.name_for_index(index).map(|name| Entry {
                    name: name.replace('\\', "/"),
                    index,
                })
            })
            .collect::<Vec<_>>();

        let container_text = match find_entry(&entries, CONTAINER_PATH) {
            Some(index) => decode_text(&read_entry(&mut archive, index, CONTAINER_PATH)?),
            None => return Err(ContainerError::NoContainer),
        };
        let package_path = find_package_path(&container_text)?;
        let package_text = match find_entry(&entries, &package_path) {
            Some(index) => decode_text(&read_entry(&mut archive, index, &package_path)?),
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
            Some(index) => read_entry(&mut self.archive, index, path).map(Some),
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

/// An entry's uncompressed bytes, at most [`MAX_FILE_BYTES`] of them
fn read_entry<R: Read + Seek>(
    archive: &mut zip::ZipArchive<R>,
    index: usize,
    path: &str,
) -> Result<Vec<u8>, ContainerError> {
    let too_large = || ContainerError::TooLarge {
        path: path.to_string(),
    };
    let undecompressable = |e: std::io::Error| ContainerError::Zip(format!("{path}: {e}"));
    let mut file = archive
        .by_index(index)
        .map_err(|e| ContainerError::Zip(format!("{path}: {e}")))?;
    if file.size() > MAX_FILE_BYTES {
        return Err(too_large());
    }
    // The declared size is only a claim, so the bytes produced are counted, and no more than
    // a little memory is set aside on its word.
    let capacity = usize::try_from(file.size().min(MAX_FIRST_ALLOCATION)).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    (&mut file)
        .take(MAX_FILE_BYTES)
        .read_to_end(&mut bytes)
        .map_err(undecompressable)?;
    // Up to the limit, the file may still go on: one more byte makes it too large. (Reading
    // the limit and then one byte apart, rather than the limit and one at once, keeps the
    // buffer from doubling past the limit.)
    let mut probe = [0u8; 1];
    loop {
        match file.read(&mut probe) {
            Ok(0) => return Ok(bytes),
            Ok(_) => return Err(too_large()),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(undecompressable(e)),
        }
    }
}
