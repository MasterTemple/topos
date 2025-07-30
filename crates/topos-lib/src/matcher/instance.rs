use line_col::LineColLookup;
use regex::Match;

use crate::{
    data::{books::BookId, data::BibleData},
    segments::{
        parse::SegmentInput,
        segments::{BookSegments, Segments},
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
*/
#[derive(Clone, Debug)]
pub struct BibleMatch {
    location: Location,
    /// I want this to be of type [`BookSegments`] so that way I can use the
    /// [`BookSegments::overlaps_with`] function
    book_segments: BookSegments,
}

impl BibleMatch {
    pub fn new(location: Location, book_id: BookId, segments: Segments) -> Self {
        Self {
            location,
            book_segments: segments.with_book(book_id),
        }
    }

    pub fn try_match<'a>(
        lookup: &LineColLookup,
        data: &'a BibleData<'a>,
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

        let segment_input = SegmentInput::try_extract(segment_window)?;

        let start = cur.start();
        let end = cur.end() + segment_input.len();
        let location = Location::new(&lookup, start, end);

        let segments = Segments::parse(segment_input).ok()?;

        Some(BibleMatch::new(location, book_id, segments))
    }
}
