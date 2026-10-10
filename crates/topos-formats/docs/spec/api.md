# `topos_bible_formats::epub::container`: the interface

The public items of the EPUB container reader: their signatures, derives, public fields, and doc
comments. The bodies, private fields, and private items are the implementer's. Behaviour is in
[epub-container.md](epub-container.md); where a doc comment and the specification differ, the
specification is right.

In the clean-room repository this module is the crate's root (`src/lib.rs`); in topos it becomes
`crates/topos-formats/src/epub/container.rs`, so it must use nothing but `std`, `zip`, and
`roxmltree`, and must not name the crate it is in. (Doc tests name it by path, so the example below names
`topos_bible_formats::epub::container`; in the clean-room repository it names that crate instead.)

```rust
//! Reading an EPUB's container: the package document `META-INF/container.xml` names, that
//! document's manifest, and any file by its path

use std::{
    fs::File,
    io::{BufReader, Read, Seek},
    path::Path,
};

/// The largest file read, uncompressed: 256 MiB
pub const MAX_FILE_BYTES: u64 = 256 * 1024 * 1024;

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

impl std::fmt::Display for ContainerError { /* … */ }
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
    pub fn has_property(&self, property: &str) -> bool;
}

/// The path a URL string names, relative to `base` (a folder ending in `/`, or empty for the
/// root), or none when it names something outside the container: the rules the manifest's
/// `href`s are read by
pub fn resolve(base: &str, url: &str) -> Option<String>;

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
pub struct Container<R> { /* private */ }

impl Container<BufReader<File>> {
    /// Opens an EPUB file
    pub fn open(path: &Path) -> Result<Self, ContainerError>;
}

impl<R: Read + Seek> Container<R> {
    /// Opens an EPUB from any reader (bytes in memory: `std::io::Cursor`)
    pub fn from_reader(reader: R) -> Result<Self, ContainerError>;

    /// The package document's path (`OEBPS/content.opf`)
    pub fn package_path(&self) -> &str;

    /// The package document's text
    pub fn package_text(&self) -> &str;

    /// The folder the package document is in, which its `href`s are relative to (`OEBPS/`, or
    /// empty at the root)
    pub fn package_folder(&self) -> &str;

    /// The manifest's items, in order
    pub fn manifest(&self) -> &[ManifestItem];

    /// The first manifest item with this `id`
    pub fn item(&self, id: &str) -> Option<&ManifestItem>;

    /// Every file's path in the archive, in the archive's order
    pub fn files(&self) -> Vec<String>;

    /// A file's bytes, or none when there's no such file
    pub fn read(&mut self, path: &str) -> Result<Option<Vec<u8>>, ContainerError>;

    /// A file's text (UTF-8 or UTF-16, by its first bytes), or none when there's no such file
    pub fn read_text(&mut self, path: &str) -> Result<Option<String>, ContainerError>;
}
```
