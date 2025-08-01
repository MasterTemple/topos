use regex::Captures;

use crate::segments::segment::Segment;

pub enum IncompleteSegment {
    // - Example Segment: ""
    // - Suggests: chapters
    Chapter {
        start_chapter: Option<u8>,
    },
    // - Example Segment: "1-"
    // - Suggests: chapters or verses
    ChapterTo {
        start_chapter: u8,
        end: Option<u8>,
    },
    // - Example Segment: "1:"
    // - Suggests: verses
    ChapterVerse {
        start_chapter: u8,
        start_verse: Option<u8>,
    },
    // - Example Segment: "1:1-"
    // - Suggests: chapters or verses
    ChapterVerseTo {
        start_chapter: u8,
        start_verse: u8,
        end: Option<u8>,
    },
    // - Example Segment: "1-2:"
    // - Suggests: verses
    ChapterRangeTo {
        start_chapter: u8,
        end_chapter: u8,
        end_verse: Option<u8>,
    },
    // - Example Segment: "1:1-2:"
    // - Suggests: verses
    ChapterVerseRangeTo {
        start_chapter: u8,
        start_verse: u8,
        end_chapter: u8,
        end_verse: Option<u8>,
    },
}
//
impl IncompleteSegment {
    pub fn new<'a>(cap: Captures<'a>) -> Self {
        let parse_cap = |group: &str| -> Option<u8> {
            cap.name(group).map(|c| c.as_str().parse().ok()).flatten()
        };
        let start_chapter = parse_cap("sc");
        let start_verse = parse_cap("sv");
        let end_chapter = parse_cap("ec");
        let end_verse = parse_cap("ev");

        match (start_chapter, start_verse, end_chapter, end_verse) {
            // Segment: "" (chapter)
            (start_chapter, None, None, None) => Self::Chapter { start_chapter },
            // Segment: "1-" (chapter or verse)
            (Some(start_chapter), None, end, None) => Self::ChapterTo { start_chapter, end },
            // Segment: "1:" (verse)
            (Some(start_chapter), start_verse, None, None) => Self::ChapterVerse {
                start_chapter,
                start_verse,
            },
            // Segment: "1:1-" (chapter or verse)
            (Some(start_chapter), Some(start_verse), end, None) => Self::ChapterVerseTo {
                start_chapter,
                start_verse,
                end,
            },
            // Segment: "1-2:" (verse)
            (Some(start_chapter), None, Some(end_chapter), end_verse) => Self::ChapterRangeTo {
                start_chapter,
                end_chapter,
                end_verse,
            },
            // Segment: "1:1-2:" (verse)
            (Some(start_chapter), Some(start_verse), Some(end_chapter), end_verse) => {
                Self::ChapterVerseRangeTo {
                    start_chapter,
                    start_verse,
                    end_chapter,
                    end_verse,
                }
            }
            _ => unreachable!(),
        }
    }
}
