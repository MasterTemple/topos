//! A persistent index of the references in many files (topos-bible-index): only changed files
//! are searched again, entries sync between devices, and queries filter, sort, and count in Rust

use std::{collections::HashSet, sync::Mutex};

use boltffi::*;
use topos_bible_index::{
    Detail, EntryMessage, EpubEntryBuilder, EpubRef, FileEntry, FileKind, Hit, Index, Order, Pack,
    PathFilter, Results, Scope, Stamp, Status, Unit, detail_name, text_entry,
};
use topos_lib::segments::Passage as CorePassage;

use crate::{OffsetUnit, Passage, Topos, ToposError, ToposQuery};

/// What kind of file an entry is for (by its extension): each kind's packs are apart, so an app
/// reads only the kinds it searches ([`ToposIndex::pack_kind`])
#[data]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexKind {
    /// Anything read as text (Markdown, subtitles, plain text)
    Text,
    Epub,
    Pdf,
}

impl From<FileKind> for IndexKind {
    fn from(kind: FileKind) -> Self {
        match kind {
            FileKind::Text => Self::Text,
            FileKind::Epub => Self::Epub,
            FileKind::Pdf => Self::Pdf,
        }
    }
}

/// Whether the index has a file's references (see [`ToposIndex::check`])
#[data]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexStatus {
    /// Its entry matches the file's size and modification time
    Fresh,
    /// An entry matches its size but not its time (likely synced from another device); it's used
    /// meanwhile, and `confirm` should check it
    Unconfirmed,
    /// The file needs to be searched
    Missing,
}

/// Which files a query looks at
#[data]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexScope {
    All,
    /// The file at the query's path
    File,
    /// Files in the folder at the query's path (and its subfolders)
    Folder,
}

/// How query results are ordered
#[data]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexOrder {
    /// By path, then position in the file
    File,
    /// By where the passage starts in the Bible, then by path and position
    Bible,
}

/// How many of a query's references are in files with an extension
#[data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionCount {
    /// Lowercase, without the dot ("" for files without one)
    pub extension: String,
    pub references: u32,
    pub files: u32,
}

/**
Which files an index query looks at, by path ([`ToposIndex::query_paths`]). Build one by chaining,
like [`ToposQuery`]; patterns are checked when it's used (or with [`ToposPaths::problems`])

```ts
const paths = ToposPaths.create().path("Sermons").path("Notes/John.md").glob("!drafts").extension("md");
index.queryPaths(topos, ToposQuery.create(), paths, IndexOrder.File);
```

A file is kept when it's one of the files or in one of the folders (if any are given), matches an
included glob or regular expression (if any are), matches no excluded one (`!` before a pattern
excludes; `\!` is a literal `!`), and has one of the extensions (if any are)
*/
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToposPaths {
    /// Files and folders (relative paths with `/`)
    pub paths: Vec<String>,
    /// Globs like ripgrep's `--glob`: `*.md` matches at any depth, and a glob matching a folder
    /// matches everything in it
    pub globs: Vec<String>,
    /// Globs ignoring case, like `--iglob`
    pub iglobs: Vec<String>,
    /// Regular expressions matched anywhere in the path (`(?i)` ignores case)
    pub regexes: Vec<String>,
    /// Extensions without the dot, any case
    pub extensions: Vec<String>,
}

impl ToposPaths {
    fn filter(&self) -> Result<PathFilter, ToposError> {
        let invalid = |e: topos_bible_index::PathError| ToposError::InvalidQuery {
            message: e.to_string(),
        };
        let mut filter = PathFilter::new();
        for path in &self.paths {
            filter = filter.path(path);
        }
        for glob in &self.globs {
            filter = filter.glob(glob, false).map_err(invalid)?;
        }
        for glob in &self.iglobs {
            filter = filter.glob(glob, true).map_err(invalid)?;
        }
        for regex in &self.regexes {
            filter = filter.regex(regex).map_err(invalid)?;
        }
        for extension in &self.extensions {
            filter = filter.extension(extension);
        }
        Ok(filter)
    }
}

fn adding(mut list: Vec<String>, value: String) -> Vec<String> {
    list.push(value);
    list
}

#[export]
impl ToposPaths {
    /// Every file
    pub fn create() -> Self {
        Self::default()
    }

    /// A file, or a folder with everything in it
    pub fn path(&self, path: String) -> Self {
        Self {
            paths: adding(self.paths.clone(), path),
            ..self.clone()
        }
    }

    /// A glob (`!` excludes)
    pub fn glob(&self, glob: String) -> Self {
        Self {
            globs: adding(self.globs.clone(), glob),
            ..self.clone()
        }
    }

    /// A glob ignoring case (`!` excludes)
    pub fn iglob(&self, glob: String) -> Self {
        Self {
            iglobs: adding(self.iglobs.clone(), glob),
            ..self.clone()
        }
    }

    /// A regular expression matched anywhere in the path (`!` excludes)
    pub fn regex(&self, regex: String) -> Self {
        Self {
            regexes: adding(self.regexes.clone(), regex),
            ..self.clone()
        }
    }

    /// An extension, without the dot
    pub fn extension(&self, extension: String) -> Self {
        Self {
            extensions: adding(self.extensions.clone(), extension),
            ..self.clone()
        }
    }

    /// Why each pattern that can't be read can't be (empty when they all can)
    pub fn problems(&self) -> Vec<String> {
        let globs = self
            .globs
            .iter()
            .map(|g| PathFilter::new().glob(g, false).err());
        let iglobs = self
            .iglobs
            .iter()
            .map(|g| PathFilter::new().glob(g, true).err());
        let regexes = self
            .regexes
            .iter()
            .map(|r| PathFilter::new().regex(r).err());
        globs
            .chain(iglobs)
            .chain(regexes)
            .flatten()
            .map(|e| e.to_string())
            .collect()
    }
}

/// A file to write: a pack (`<device>-<kind>-<number>.bin`) or an EPUB's or PDF's details
/// (`<name>.bin`)
#[data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Where a reference in an EPUB is
#[data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexEpub {
    pub spine_index: u32,
    /// Its range CFI (empty until the book's details are loaded)
    pub cfi: String,
    pub chapter: Option<String>,
}

/// Where a reference in a PDF is
#[data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexPdf {
    /// 1-based
    pub page: u32,
    /// Where it is as PDF++ links write it (`page=3&rect=72,708,126,720`; empty until the
    /// document's details are loaded)
    pub locator: String,
}

/// A reference from the index
#[data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexHit {
    pub path: String,
    pub passage: Passage,
    /// Offsets in the index's unit (for EPUBs, UTF-16 offsets through the book's text)
    pub start: u32,
    pub end: u32,
    /// 1-based line and column of the start; for EPUBs, the column is in `line_text`
    pub line: u32,
    pub column: u32,
    /// For EPUBs (once their details are loaded): the text around the reference. Empty for
    /// other files, whose lines are in the files themselves
    pub line_text: String,
    pub epub: Option<IndexEpub>,
    pub pdf: Option<IndexPdf>,
}

/// A reference in an EPUB, for [`ToposIndex::set_epub`]
#[data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EpubReference {
    pub passage: Passage,
    /// Its UTF-16 range in the book's text
    pub start: u32,
    pub end: u32,
    /// 1-based line in the book's text, and UTF-16 column in it
    pub line: u32,
    pub column: u32,
    pub spine_index: u32,
    pub chapter: Option<String>,
    pub cfi: String,
    /// The line (paragraph) it starts in
    pub line_text: String,
}

fn unit(unit: OffsetUnit) -> Unit {
    match unit {
        OffsetUnit::Byte => Unit::Byte,
        OffsetUnit::Char => Unit::Char,
        OffsetUnit::Utf16 => Unit::Utf16,
    }
}

/// Times and sizes are numbers, as JavaScript has them
fn whole(n: f64) -> u64 {
    if n.is_finite() && n > 0.0 {
        n as u64
    } else {
        0
    }
}

fn invalid(e: impl std::fmt::Display) -> ToposError {
    ToposError::InvalidIndex {
        message: e.to_string(),
    }
}

/**
The references in many files, kept between runs and shared between devices

```ts
const index = ToposIndex.create("laptop", "1.0", OffsetUnit.Utf16);
for (const bytes of packs) index.loadPack(bytes);
if (index.check(path, size, mtime) !== IndexStatus.Fresh) index.indexText(topos, path, size, mtime, text, Date.now());
const results = index.query(topos, ToposQuery.create().anyOverlap("Romans 8"), IndexScope.All, "", IndexOrder.Bible);
index.page(topos, results, 0, 100);
// Only Markdown files, and how many references each kind of file has
index.queryIn(topos, ToposQuery.create(), IndexScope.All, "", ["md"], IndexOrder.File);
index.extensionCounts(results);
for (const file of index.takeDirtyPacks()) write(file.name, file.bytes);
```
*/
pub struct ToposIndex {
    inner: Mutex<Index>,
}

/// What a query kept, read a page at a time with [`ToposIndex::page`]
pub struct IndexResults {
    inner: Results,
}

#[export]
impl IndexResults {
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// How many files the references are in
    pub fn file_count(&self) -> u32 {
        self.inner.file_count() as u32
    }
}

impl ToposIndex {
    fn lock(&self) -> std::sync::MutexGuard<'_, Index> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn hit(&self, topos: &Topos, hit: &Hit) -> Option<IndexHit> {
        let passage = topos.passage(&hit.reference.passage, &topos.format)?;
        let r = hit.reference;
        let locator = || hit.detail.map(|d| d.cfi.clone()).unwrap_or_default();
        let kind = FileKind::of(hit.path);
        let epub = (r.section)
            .filter(|_| kind == FileKind::Epub)
            .map(|section| IndexEpub {
                spine_index: section.spine,
                cfi: locator(),
                chapter: hit.chapter().map(str::to_string),
            });
        // A PDF's page index is where an EPUB's spine item is
        let pdf = (r.section)
            .filter(|_| kind == FileKind::Pdf)
            .map(|section| IndexPdf {
                page: section.spine + 1,
                locator: locator(),
            });
        let (line_text, column) = match hit.detail {
            Some(detail) => (detail.context.clone(), detail.at + 1),
            None if r.section.is_some() => (String::new(), 1),
            None => (String::new(), r.column),
        };
        Some(IndexHit {
            path: hit.path.to_string(),
            passage,
            start: r.start,
            end: r.end(),
            line: r.line,
            column,
            line_text,
            epub,
            pdf,
        })
    }
}

#[export]
impl ToposIndex {
    /// An empty index for this device (`device` names its packs); `engine` (a version) is written
    /// in them
    pub fn create(device: String, engine: String, offsets: OffsetUnit) -> Self {
        Self {
            inner: Mutex::new(Index::new(&device, unit(offsets), &engine)),
        }
    }

    /// Adds a pack (this device's from an earlier run, or another device's)
    pub fn load_pack(&self, bytes: Vec<u8>) -> Result<(), ToposError> {
        let pack = Pack::decode(&bytes).map_err(invalid)?;
        self.lock().load_pack(pack).map_err(invalid)
    }

    /// Whether the index has the references of a file of this size and modification time (in
    /// milliseconds), and makes that entry the one in use
    pub fn check(&self, path: String, size: f64, mtime: f64) -> IndexStatus {
        match self.lock().check(&path, whole(size), whole(mtime)) {
            Status::Fresh => IndexStatus::Fresh,
            Status::Unconfirmed => IndexStatus::Unconfirmed,
            Status::Missing => IndexStatus::Missing,
        }
    }

    /// Checks an `Unconfirmed` file by its text (or, without text, by its size alone, for files
    /// too big to read again); returns whether an entry matched
    pub fn confirm(&self, path: String, size: f64, mtime: f64, text: Option<String>) -> bool {
        let hash = text.as_deref().map(topos_bible_index::text_hash);
        self.lock().confirm(&path, whole(size), whole(mtime), hash)
    }

    /// Searches a text file and keeps its references; returns how many there are
    pub fn index_text(
        &self,
        topos: &Topos,
        path: String,
        size: f64,
        mtime: f64,
        text: String,
        written: f64,
    ) -> u32 {
        let mut index = self.lock();
        let unit = index.unit();
        let entry = text_entry(
            &topos.matcher,
            &text,
            unit,
            whole(size),
            whole(mtime),
            whole(written),
        );
        let count = entry.refs.len() as u32;
        index.set(&path, entry);
        count
    }

    /**
    Adds an entry made elsewhere ([`Topos::index_entry`] on another thread, or `topos -m index`),
    for the file at `path` with this size and modification time
    */
    pub fn insert(
        &self,
        bytes: Vec<u8>,
        path: String,
        size: f64,
        mtime: f64,
    ) -> Result<(), ToposError> {
        let mut message = EntryMessage::decode(&bytes).map_err(invalid)?;
        message.path = path;
        message.entry.stamp.size = whole(size);
        message.entry.stamp.mtime = whole(mtime);
        self.lock().insert(message).map_err(invalid)?;
        Ok(())
    }

    /// Keeps an EPUB's references, found elsewhere
    pub fn set_epub(
        &self,
        path: String,
        size: f64,
        mtime: f64,
        written: f64,
        references: Vec<EpubReference>,
    ) {
        let mut builder = EpubEntryBuilder::default();
        for r in &references {
            builder.push(EpubRef {
                passage: CorePassage::from(&r.passage),
                start: r.start,
                end: r.end,
                line: r.line,
                column: r.column,
                spine: r.spine_index,
                chapter: r.chapter.as_deref(),
                cfi: r.cfi.clone(),
                line_text: &r.line_text,
            });
        }
        let stamp = Stamp {
            size: whole(size),
            mtime: whole(mtime),
            hash: None,
        };
        let (entry, detail) = builder.finish(stamp, whole(written), detail_name(&path, &stamp));
        let (entry, detail) = if entry.refs.is_empty() {
            (
                FileEntry {
                    detail: None,
                    ..entry
                },
                None,
            )
        } else {
            (entry, Some(detail))
        };
        let mut index = self.lock();
        let unit = index.unit();
        let message = EntryMessage {
            path,
            unit,
            entry,
            detail,
        };
        let _ = index.insert(message);
    }

    /// Takes a file out (deleted, or no longer searched)
    pub fn remove(&self, path: String) {
        self.lock().remove(&path);
    }

    pub fn rename(&self, old: String, new: String) {
        self.lock().rename(&old, &new);
    }

    /// Takes out every file not in `paths`
    pub fn retain(&self, paths: Vec<String>) {
        let paths: HashSet<&str> = paths.iter().map(String::as_str).collect();
        self.lock().retain(&paths);
    }

    /// Whether this device has packs to write
    pub fn is_dirty(&self) -> bool {
        self.lock().is_dirty()
    }

    /// This device's packs that changed, to write (each replaces the file of the same name,
    /// `<device>-<kind>-<number>.bin`)
    pub fn take_dirty_packs(&self) -> Vec<IndexFile> {
        self.lock()
            .take_dirty_packs()
            .into_iter()
            .map(|pack| IndexFile {
                name: pack.file_name(),
                bytes: pack.encode(),
            })
            .collect()
    }

    /// The kind of files a pack (its file name, `laptop-epub-07.bin`) has entries for, so only
    /// the kinds an app searches are read; `None` for another name (a pack from an older index)
    pub fn pack_kind(&self, name: String) -> Option<IndexKind> {
        let stem = name.strip_suffix(".bin")?;
        let mut parts = stem.rsplitn(3, '-');
        let (_number, kind) = (parts.next()?, parts.next()?);
        parts.next()?;
        FileKind::from_name(kind).map(Into::into)
    }

    /// The kind of a file, by its extension
    pub fn kind_of(&self, path: String) -> IndexKind {
        FileKind::of(&path).into()
    }

    /// New EPUB details to write (`<name>.bin`)
    pub fn take_unsaved_details(&self) -> Vec<IndexFile> {
        self.lock()
            .take_unsaved_details()
            .into_iter()
            .map(|(name, detail)| IndexFile {
                name: format!("{name}.bin"),
                bytes: detail.encode(),
            })
            .collect()
    }

    /// The details any entry uses (by name, without `.bin`), so others can be deleted
    pub fn detail_names(&self) -> Vec<String> {
        self.lock().detail_names().into_iter().collect()
    }

    /// Adds an EPUB's details, read from `<name>.bin`
    pub fn load_detail(&self, name: String, bytes: Vec<u8>) -> Result<(), ToposError> {
        let detail = Detail::decode(&bytes).map_err(invalid)?;
        self.lock().load_detail(&name, detail);
        Ok(())
    }

    /// The details a file's references need that aren't loaded (a name), if any
    pub fn missing_file_detail(&self, path: String) -> Option<String> {
        let index = self.lock();
        let name = index.entry(&path)?.detail.clone()?;
        (!index.has_detail(&name)).then_some(name)
    }

    /// The references a query keeps, in `scope` (with `path` for a file or folder)
    pub fn query(
        &self,
        topos: &Topos,
        query: &ToposQuery,
        scope: IndexScope,
        path: String,
        order: IndexOrder,
    ) -> Result<IndexResults, ToposError> {
        self.query_in(topos, query, scope, path, vec![], order)
    }

    /// Like [`ToposIndex::query`], in files with one of `extensions` (without the dot, any case;
    /// empty for every file)
    pub fn query_in(
        &self,
        topos: &Topos,
        query: &ToposQuery,
        scope: IndexScope,
        path: String,
        extensions: Vec<String>,
        order: IndexOrder,
    ) -> Result<IndexResults, ToposError> {
        let scope = match scope {
            IndexScope::All => Scope::All,
            IndexScope::File => Scope::File(path),
            IndexScope::Folder => Scope::Folder(path),
        };
        let order = match order {
            IndexOrder::File => Order::File,
            IndexOrder::Bible => Order::Bible,
        };
        // Building a filter copies the Bible's data, so a query without filters skips it
        let results = if query.is_empty() {
            self.lock().query_in(None, None, &scope, &extensions, order)
        } else {
            let matcher = topos.filter(query)?.create_matcher();
            let books = matcher.possible_books();
            let keep = |p: &CorePassage| matcher.keeps(p);
            self.lock()
                .query_in(Some(&keep), books.as_ref(), &scope, &extensions, order)
        };
        Ok(IndexResults { inner: results })
    }

    /// The references a query keeps in the files `paths` keeps (an error names a pattern that
    /// can't be read)
    pub fn query_paths(
        &self,
        topos: &Topos,
        query: &ToposQuery,
        paths: &ToposPaths,
        order: IndexOrder,
    ) -> Result<IndexResults, ToposError> {
        let filter = paths.filter()?;
        let order = match order {
            IndexOrder::File => Order::File,
            IndexOrder::Bible => Order::Bible,
        };
        let results = if query.is_empty() {
            self.lock()
                .query_paths(None, None, &Scope::All, &filter, order)
        } else {
            let matcher = topos.filter(query)?.create_matcher();
            let books = matcher.possible_books();
            let keep = |p: &CorePassage| matcher.keeps(p);
            self.lock()
                .query_paths(Some(&keep), books.as_ref(), &Scope::All, &filter, order)
        };
        Ok(IndexResults { inner: results })
    }

    /// How many of the results are in files with each extension, by extension
    pub fn extension_counts(&self, results: &IndexResults) -> Vec<ExtensionCount> {
        self.lock()
            .extension_counts(&results.inner)
            .into_iter()
            .map(|(extension, references, files)| ExtensionCount {
                extension,
                references: references as u32,
                files: files as u32,
            })
            .collect()
    }

    /// Results `offset` to `offset + limit`
    pub fn page(
        &self,
        topos: &Topos,
        results: &IndexResults,
        offset: u32,
        limit: u32,
    ) -> Vec<IndexHit> {
        let index = self.lock();
        index
            .page(&results.inner, offset as usize, limit as usize)
            .iter()
            .filter_map(|hit| self.hit(topos, hit))
            .collect()
    }

    /// The details (names) a page needs that aren't loaded
    pub fn missing_details(&self, results: &IndexResults, offset: u32, limit: u32) -> Vec<String> {
        self.lock()
            .missing_details(&results.inner, offset as usize, limit as usize)
    }

    /// Whether the index changed since the query
    pub fn is_current(&self, results: &IndexResults) -> bool {
        self.lock().is_current(&results.inner)
    }

    /// A file's references
    pub fn file_hits(&self, topos: &Topos, path: String) -> Vec<IndexHit> {
        let index = self.lock();
        index
            .file_hits(&path)
            .iter()
            .filter_map(|hit| self.hit(topos, hit))
            .collect()
    }

    /// Paths of the files with references
    pub fn paths(&self) -> Vec<String> {
        self.lock().paths().map(str::to_string).collect()
    }

    pub fn file_count(&self) -> u32 {
        self.lock().file_count() as u32
    }

    pub fn reference_count(&self) -> u32 {
        self.lock().reference_count() as u32
    }
}

#[export]
impl Topos {
    /**
    Searches an EPUB (its bytes) for [`ToposIndex::insert`]: its references (UTF-16 offsets
    through the book's text) with their CFIs, written as the EPUB++ reader writes them, and the
    text around each. Errs when the book can't be read (not a ZIP, or no package document)
    */
    pub fn index_epub(
        &self,
        path: String,
        size: f64,
        mtime: f64,
        bytes: Vec<u8>,
        written: f64,
    ) -> Result<Vec<u8>, ToposError> {
        let stamp = Stamp {
            size: whole(size),
            mtime: whole(mtime),
            hash: None,
        };
        let (entry, detail) =
            topos_bible_index::epub_file_entry(&self.matcher, &path, &bytes, stamp, whole(written))
                .map_err(invalid)?;
        Ok(EntryMessage {
            path,
            unit: Unit::Utf16,
            entry,
            detail,
        }
        .encode())
    }

    /**
    Searches a PDF (its bytes) for [`ToposIndex::insert`], like [`Topos::index_epub`]: UTF-16
    offsets through the document's text, and each reference's page and PDF++ locator. Native only,
    with the `pdf` feature (MuPDF): elsewhere it errs
    */
    pub fn index_pdf(
        &self,
        path: String,
        size: f64,
        mtime: f64,
        bytes: Vec<u8>,
        written: f64,
    ) -> Result<Vec<u8>, ToposError> {
        #[cfg(feature = "pdf")]
        {
            let stamp = Stamp {
                size: whole(size),
                mtime: whole(mtime),
                hash: None,
            };
            let (entry, detail) = topos_bible_index::pdf_file_entry(
                &self.matcher,
                &path,
                &bytes,
                stamp,
                whole(written),
            )
            .map_err(invalid)?;
            Ok(EntryMessage {
                path,
                unit: Unit::Utf16,
                entry,
                detail,
            }
            .encode())
        }
        #[cfg(not(feature = "pdf"))]
        {
            let _ = (path, size, mtime, bytes, written);
            Err(invalid(
                "PDFs need topos built with the pdf feature (MuPDF), natively",
            ))
        }
    }

    /**
    Searches a text file for [`ToposIndex::insert`], in the index's format: lets another thread
    (a Web Worker) search while the index stays where it's queried
    */
    pub fn index_entry(
        &self,
        path: String,
        size: f64,
        mtime: f64,
        text: String,
        written: f64,
        offsets: OffsetUnit,
    ) -> Vec<u8> {
        let unit = unit(offsets);
        let entry = text_entry(
            &self.matcher,
            &text,
            unit,
            whole(size),
            whole(mtime),
            whole(written),
        );
        EntryMessage {
            path,
            unit,
            entry,
            detail: None,
        }
        .encode()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_by_path() -> Result<(), ToposError> {
        let topos = Topos::new();
        let index = ToposIndex::create("laptop".into(), "test".into(), OffsetUnit::Utf16);
        for (path, text) in [
            ("Notes/a.md", "John 3:16"),
            ("Notes/drafts/b.md", "Rom 8:28"),
            ("Sermons/c.srt", "Ps 23"),
            ("d.txt", "Gen 1:1"),
        ] {
            index.index_text(&topos, path.into(), 1.0, 1.0, text.into(), 1.0);
        }
        let found = |paths: ToposPaths| -> Result<Vec<String>, ToposError> {
            let results =
                index.query_paths(&topos, &ToposQuery::create(), &paths, IndexOrder::File)?;
            Ok(index
                .page(&topos, &results, 0, 10)
                .into_iter()
                .map(|h| h.path)
                .collect())
        };
        let notes = ToposPaths::create()
            .path("Notes".into())
            .path("d.txt".into());
        assert_eq!(
            found(notes.glob("!drafts".into()))?,
            ["d.txt", "Notes/a.md"]
        );
        assert_eq!(
            found(ToposPaths::create().iglob("SERMONS".into()))?,
            ["Sermons/c.srt"]
        );
        assert_eq!(
            found(
                ToposPaths::create()
                    .regex(r"\.(md|txt)$".into())
                    .extension("TXT".into())
            )?,
            ["d.txt"]
        );
        let bad = ToposPaths::create()
            .glob("a[".into())
            .regex("(".into())
            .glob("*.md".into());
        assert_eq!(bad.problems().len(), 2);
        assert!(matches!(found(bad), Err(ToposError::InvalidQuery { .. })));
        Ok(())
    }

    #[test]
    fn packs_are_named_by_kind() {
        let topos = Topos::new();
        let index = ToposIndex::create("my-laptop".into(), "test".into(), OffsetUnit::Utf16);
        index.index_text(&topos, "a.md".into(), 1.0, 1.0, "John 3:16".into(), 1.0);
        let names: Vec<String> = index
            .take_dirty_packs()
            .into_iter()
            .map(|f| f.name)
            .collect();
        assert_eq!(names.len(), 1);
        assert!(names[0].starts_with("my-laptop-text-"), "{names:?}");
        assert_eq!(index.pack_kind(names[0].clone()), Some(IndexKind::Text));
        assert_eq!(
            index.pack_kind("my-laptop-pdf-03.bin".into()),
            Some(IndexKind::Pdf)
        );
        // Packs from before kinds (`<device>-<number>.bin`) have none
        assert_eq!(index.pack_kind("my-laptop-03.bin".into()), None);
        assert_eq!(index.pack_kind("laptop-03.bin".into()), None);
        assert_eq!(index.kind_of("Books/A.EPUB".into()), IndexKind::Epub);
    }

    #[cfg(not(feature = "pdf"))]
    #[test]
    fn pdfs_need_the_feature() {
        let topos = Topos::new();
        assert!(
            topos
                .index_pdf("a.pdf".into(), 1.0, 1.0, vec![], 1.0)
                .is_err()
        );
    }
}
