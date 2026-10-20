use std::ops::Range;

use regex::Match;

use crate::{
    data::{
        bible_data::BibleData,
        books::BookId,
        chapter_verses::ChapterVerses,
        translations::{Translation, Translations},
    },
    matcher::line_col::LineColLocation,
    segments::{
        grammar::{
            NumberKind, SegmentList,
            lexer::{SPACES, is_unit_symbol},
        },
        passage::{Passage, Segments},
        resolve::{Resolver, SegmentSpan},
    },
};

/**
- This is the minimal amount of data needed for a match in order to do complex filtering
- There will be a separate struct that will include file name, book name, book abbreviation, and so on

I think this is the ideal representation, because a matcher could have the capacity to get different locations
```ignore
pub trait Matcher<Location> {
    fn search(&self, input: PathOrContent) -> Option<Vec<BibleMatch<Location>>>;
}

impl Matcher<LineCol> for PlaintextMatcher {}
impl Matcher<TextFragment> for PlaintextMatcher {}

impl Matcher<Timestamp> for MediaMatcher {}

impl Matcher<Page> for PDFMatcher {}
impl Matcher<TextFragment> for PDFMatcher {}
```

but how will I parse, for example, timestamps from both SRT files and Whisper JSON files?
perhaps have additional args/params to the search method?
**no, make them part of the matcher struct**
create child structs if I need them: `WhisperJSONMediaMatcher` and `SRTMediaMatcher`

ideally, Location will be a big enum, so I don't have to deal with generics at the top level


*/
#[derive(Clone, Debug)]
pub struct BibleMatch<L = LineColLocation> {
    // TODO: make this into context, where Minimal<Location> is just the location, but
    // Verbose<Location> has the location and other things like the line, surrounding context
    // NOTE: I should make location (and maybe context type too) into enums
    pub location: L,
    /// I want this to be of type [`Passage`] so that way I can use the
    /// [`Passage::overlaps_with`] function
    pub psg: Passage,
}

impl<L> BibleMatch<L> {
    pub fn new(location: L, book_id: BookId, segments: Segments) -> Self {
        Self {
            location,
            psg: segments.with_book(book_id),
        }
    }
    pub fn map_loc<N>(self, f: impl FnOnce(L) -> N) -> BibleMatch<N> {
        BibleMatch {
            location: f(self.location),
            psg: self.psg,
        }
    }
}

/// A passage found in text, before its location is computed
#[derive(Clone, Debug)]
pub struct FoundPassage {
    /// Byte range of the book name and segments in the searched text
    pub bytes: Range<usize>,
    pub psg: Passage,
    /// Byte range of each segment, the first from the book name: `John 3:16` and `4:5-7` in
    /// `John 3:16; 4:5-7`; only when asked for (see [`BibleMatcher::search_with_parts`]), else empty
    ///
    /// [`BibleMatcher::search_with_parts`]: crate::matcher::BibleMatcher::search_with_parts
    pub parts: Vec<SegmentSpan>,
}

/// Whether `rest` starts with a number that has a unit or symbol (`5%`, `39pt`, `1 %`)
fn runs_into_quantity(rest: &str) -> bool {
    let rest = rest.trim_start();
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return false;
    }
    let after = &rest[digits..];
    let glued_letter = after.chars().next().is_some_and(char::is_alphabetic);
    let symbol = after
        .trim_start_matches(SPACES)
        .chars()
        .next()
        .is_some_and(is_unit_symbol);
    glued_letter || symbol
}

/// Text without whitespace this long is encoded data or code (like base64 in an SVG), not prose
const LONG_TOKEN: usize = 64;

/// Whether the word around `at` is at least [`LONG_TOKEN`] bytes without whitespace
fn in_long_token(text: &str, at: usize) -> bool {
    let is_space = |b: &u8| b.is_ascii_whitespace();
    let before = text.as_bytes()[..at]
        .iter()
        .rev()
        .take(LONG_TOKEN)
        .take_while(|b| !is_space(b))
        .count();
    let after = text.as_bytes()[at..]
        .iter()
        .take(LONG_TOKEN)
        .take_while(|b| !is_space(b))
        .count();
    before + after >= LONG_TOKEN
}

/// Book names are written in lowercase, Title Case, or CAPS, never like `mK` or `jE`
fn book_case_is_plausible(name: &str) -> bool {
    name.split_whitespace().all(|word| {
        let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
        !letters
            .windows(2)
            .any(|pair| pair[0].is_lowercase() && pair[1].is_uppercase())
    })
}

impl FoundPassage {
    /**
    Parses the passage that starts with the book name `cur`
    - `next_start` is where the next book name starts, which ends this passage's segments
      (that is how `John 1:1, 3 John 5` becomes `John 1:1` and `3 John 5`)
    */
    pub fn find(
        data: &BibleData,
        text: &str,
        cur: Match<'_>,
        next_start: Option<usize>,
    ) -> Option<Self> {
        Self::find_translated(data, text, cur, next_start, None, false).map(|(found, _)| found)
    }

    /**
    [`FoundPassage::find`], and the translation named right after the passage, if any of
    `translations` is (the passage is checked against its chapter and verse counts), with each
    segment's place when `with_parts`
    */
    pub(crate) fn find_translated(
        data: &BibleData,
        text: &str,
        cur: Match<'_>,
        next_start: Option<usize>,
        translations: Option<&Translations>,
        with_parts: bool,
    ) -> Option<(Self, Option<Translation>)> {
        let book = data.books().search(cur.as_str())?;
        if in_long_token(text, cur.start()) || !book_case_is_plausible(cur.as_str()) {
            return None;
        }
        let window = &text[cur.end()..next_start.unwrap_or(text.len())];
        let (list, translation) = match translations {
            Some(translations) => {
                let (list, named) = translations.read_segments(&text[cur.end()..], window.len());
                let translation =
                    named.map(|(name, range)| Translation::at(name, range, cur.end()));
                (list, translation)
            }
            None => (SegmentList::parse(window), None),
        };
        let versification = (translation.as_ref())
            .zip(translations)
            .and_then(|(t, translations)| translations.chapter_verses(&t.name, book))
            .or_else(|| data.chapter_verses().get_chapter_verses(&book));
        let (found, verse) =
            Self::resolve(versification, book, cur.end(), window, &list, with_parts)?;
        let has_verse = verse.is_some();
        // Abbreviations that are also words need a `:` verse (`Is 1:1`), since `is 2.5` is a
        // decimal; a `.` verse only counts after an abbreviation with a period (`Is. 53.5`)
        let ambiguous_ok = match verse {
            Some(':') => true,
            Some(_) => cur.as_str().ends_with('.'),
            None => false,
        };
        if !ambiguous_ok && data.books().is_ambiguous(cur.as_str()) {
            return None;
        }
        // A book glued to its chapter (`Jn3:16`) needs a verse, and never a Roman numeral, since
        // codes and encoded data are full of things like `Gn5` and `GNi`
        let first = list.nodes.first()?;
        let glued = first.start.span.start == 0 && !cur.as_str().ends_with('.');
        if glued && (!has_verse || first.start.kind == NumberKind::Roman) {
            return None;
        }
        let mut found = Self {
            bytes: cur.start()..found.bytes.end,
            ..found
        };
        if let Some(first) = found.parts.first_mut() {
            first.written.start = cur.start();
        }
        Some((found, translation))
    }

    /**
    A book named alone (`name`, a match of [`Books::book_regex`]), as the whole book: not an
    abbreviation that is also a word (`is`), not followed by a number (that's a reference, or
    nothing), and never inside encoded data or oddly cased (`mK`); its `.` isn't part of it
    (`Romans.` ending a sentence)

    [`Books::book_regex`]: crate::data::books::Books::book_regex
    */
    pub(crate) fn book_alone(
        data: &BibleData,
        text: &str,
        name: Match<'_>,
        with_parts: bool,
    ) -> Option<Self> {
        let written = name.as_str().trim_end_matches('.');
        let books = data.books();
        let followed_by_number = text[name.end()..]
            .trim_start()
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_digit());
        if followed_by_number
            || books.is_ambiguous(written)
            || in_long_token(text, name.start())
            || !book_case_is_plausible(written)
        {
            return None;
        }
        let psg = data.whole_book(books.search(written)?)?;
        let bytes = name.start()..name.start() + written.len();
        let parts = match with_parts {
            true => vec![SegmentSpan {
                written: bytes.clone(),
                end: None,
            }],
            false => Vec::new(),
        };
        Some(Self { bytes, psg, parts })
    }

    /// A reference without a book name (`1:1-5`), which must start with `chapter:verse`
    pub fn find_bare(
        data: &BibleData,
        book: BookId,
        text: &str,
        window: Range<usize>,
    ) -> Option<Self> {
        Self::bare(data, book, text, window, false)
    }

    /// [`FoundPassage::find_bare`], with each segment's place when `with_parts`
    pub(crate) fn bare(
        data: &BibleData,
        book: BookId,
        text: &str,
        window: Range<usize>,
        with_parts: bool,
    ) -> Option<Self> {
        let list = SegmentList::parse(&text[window.clone()]);
        let first = list.nodes.first()?;
        if first.start_verse.is_none_or(|p| p.delimiter.actual != ':') {
            return None;
        }
        let versification = data.chapter_verses().get_chapter_verses(&book);
        Self::resolve(
            versification,
            book,
            window.start,
            &text[window],
            &list,
            with_parts,
        )
        .map(|(found, _)| found)
    }

    /**
    Resolves `list`, parsed from `window` (which starts at byte `start`), and returns the
    delimiter of its explicit verse, if any (`:` is preferred over `.`)
    */
    fn resolve(
        versification: Option<&ChapterVerses>,
        book: BookId,
        start: usize,
        window: &str,
        list: &SegmentList,
        with_parts: bool,
    ) -> Option<(Self, Option<char>)> {
        // `3:16%` or `2.5%`: the reference runs into a quantity, so it is not a reference
        if list.nodes.last().is_some_and(|n| !n.is_complete())
            && runs_into_quantity(&window[list.end()..])
        {
            return None;
        }
        let resolver = Resolver::for_book(versification);
        let (resolved, spans) = if with_parts {
            resolver.resolve_with_spans(&list.nodes)
        } else {
            (resolver.resolve(&list.nodes), Vec::new())
        };
        if resolved.used == 0 {
            return None;
        }
        let verses = list.nodes[..resolved.used]
            .iter()
            .filter_map(|node| node.start_verse.filter(|p| p.number.is_some()))
            .map(|p| p.delimiter.actual);
        let verse = verses.fold(None, |best, d| match best {
            Some(':') => best,
            _ => Some(d),
        });
        let found = Self {
            // Only the text that resolved is part of the match
            bytes: start..start + resolved.end,
            psg: resolved.segments.with_book(book),
            parts: (spans.into_iter())
                .map(|span| SegmentSpan {
                    written: start + span.written.start..start + span.written.end,
                    end: span.end.map(|end| start + end),
                })
                .collect(),
        };
        Some((found, verse))
    }
}
