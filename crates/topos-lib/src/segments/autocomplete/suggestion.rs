use regex::Captures;

use crate::segments::segment::Segment;

pub enum SuggestionType {
    Chapter,
    Verse,
    Both,
}

pub struct IncompleteSegment {
    ty: SuggestionType,
    prev: Option<Segment>,
    start_chapter: Option<u8>,
    start_verse: Option<u8>,
    end_chapter: Option<u8>,
    end_verse: Option<u8>,
}

pub enum ValueOrFocused {
    Value(u8),
    Focused,
}

impl ValueOrFocused {
    pub fn try_new<'a>(cap: Captures<'a>, group: &str) -> Option<Self> {
        cap.name(group).map(|c| match c.as_str().parse() {
            Ok(v) => ValueOrFocused::Value(v),
            Err(_) => ValueOrFocused::Focused,
        })
    }
}

impl IncompleteSegment {
    pub fn new<'a>(cap: Captures<'a>) -> Self {
        let parse_cap = |group: &str| -> Option<ValueOrFocused> {
            cap.name(group).map(|c| match c.as_str().parse() {
                Ok(v) => ValueOrFocused::Value(v),
                Err(_) => ValueOrFocused::Focused,
            })
        };
        let start_chapter = parse_cap("sc");
        let start_verse = parse_cap("sv");
        let end_chapter = parse_cap("ec");
        let end_verse = parse_cap("ev");

        use ValueOrFocused::*;

        match (start_chapter, start_verse, end_chapter, end_verse) {
            // Segment: ""
            (Some(Focused), None, None, None) => {
                //
            }
            // Segment: "1-"
            (Some(Value(start_chapter)), None, Some(Focused), None) => {
                //
            }
            // Segment: "1:"
            (Some(Value(start_chapter)), Some(Focused), None, None) => {
                //
            }
            // Segment: "1:1-"
            (Some(Value(start_chapter)), Some(Value(start_verse)), Some(Focused), None) => {
                //
            }
            // Segment: "1-2:"
            (Some(Value(start_chapter)), None, Some(Value(end_chapter)), Some(Focused)) => {
                //
            }
            // Segment: "1:1-2:"
            (
                Some(Value(start_chapter)),
                Some(Value(start_verse)),
                Some(Value(end_chapter)),
                Some(Focused),
            ) => {
                //
            }
            _ => unreachable!(),
        };

        todo!()
    }
}

/**
This is what is returned in response to a set of segments
*/
pub enum CompletionSegmentSuggestion {
    /// `John ?`
    Chapter(u8),
    /// `John 1:1,?`
    ChapterOrVerse(u8),
    /// `John 1:1,2:?`
    ChapterVerse { chapter: u8, verse: u8 },
    /// `John 1:1,2:1-?`
    ChapterVerseRange {
        chapter: u8,
        verse: u8,
        /// chapter or verse
        end: u8,
    },
    /// `John 1:1,2:1-3:?`
    ChapterRange {
        start_chapter: u8,
        start_verse: u8,
        end_chapter: u8,
        end_verse: u8,
    },
}
