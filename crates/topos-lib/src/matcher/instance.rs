use line_col::LineColLookup;
use regex::Match;

use crate::{
    data::{books::BookId, data::BibleData},
    segments::{
        parser::minimal::MinimalSegments,
        segments::{Passage, Segments},
    },
};

#[derive(Copy, Clone, Debug)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
    pub fn new_pair((line, column): (usize, usize)) -> Self {
        Self::new(line, column)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Location {
    pub start: Position,
    pub end: Position,
}

impl Location {
    pub fn new(lookup: &LineColLookup, start: usize, end: usize) -> Self {
        let start = Position::new_pair(lookup.get(start));
        let end = Position::new_pair(lookup.get(end));
        Self { start, end }
    }
}

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
pub struct BibleMatch {
    // TODO: make this into context, where Minimal<Location> is just the location, but
    // Verbose<Location> has the location and other things like the line, surrounding context
    // NOTE: I should make location (and maybe context type too) into enums
    pub location: Location,
    /// I want this to be of type [`BookSegments`] so that way I can use the
    /// [`BookSegments::overlaps_with`] function
    pub psg: Passage,
}

impl BibleMatch {
    pub fn new(location: Location, book_id: BookId, segments: Segments) -> Self {
        Self {
            location,
            psg: segments.with_book(book_id),
        }
    }

    pub fn try_match<'a>(
        lookup: &LineColLookup,
        data: &'a BibleData,
        input: &str,
        cur: Match<'a>,
        next_start: Option<usize>,
    ) -> Option<Self> {
        let book_id = data.books().search(cur.as_str())?;

        let segment_window = if let Some(next_start) = next_start {
            &input[cur.end()..next_start]
        } else {
            &input[cur.end()..]
        };

        let segment_input = MinimalSegments::parse(segment_window)?;
        // eprintln!("{} vs {}", old_segment_input.len(), segment_input.len());

        let start = cur.start();
        let end = cur.end() + segment_input.len();
        let location = Location::new(&lookup, start, end);

        // let segments = Segments::parse(segment_input)?;
        let segments = Segments::from(segment_input);

        Some(BibleMatch::new(location, book_id, segments))
    }
}
