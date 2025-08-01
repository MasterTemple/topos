use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};

use crate::{
    data::chapter_verses::ChapterVerses,
    segments::{
        autocomplete::full::{CHAPTER_DELIMETER, RANGE_DELIMETER, WS},
        segment::Segment,
        verse_bounds::VerseBounds,
    },
};

#[derive(Clone, Debug)]
pub enum IncompleteSegment {
    /// - Example Segment: ""
    /// - Suggests: chapters or verses
    ChapterOrVerse { start: Option<u8> },
    // ChapterOrVerse,
    /// - Example Segment: "1-"
    /// - Suggests: chapters or verses
    ChapterOrVerseTo { start: u8, end: Option<u8> },
    // ChapterTo { start_chapter: u8 },
    /// - Example Segment: "1:"
    /// - Suggests: verses
    ChapterVerse {
        start_chapter: u8,
        start_verse: Option<u8>,
    },
    /// - Example Segment: "1:1-"
    /// - Suggests: chapters or verses
    ChapterVerseTo {
        start_chapter: u8,
        start_verse: u8,
        end: Option<u8>,
    },
    /// - Example Segment: "1-2:"
    /// - Suggests: verses
    ChapterRangeTo {
        start_chapter: u8,
        end_chapter: u8,
        end_verse: Option<u8>,
    },
    /// - Example Segment: "1:1-2:"
    /// - Suggests: verses
    ChapterVerseRangeTo {
        start_chapter: u8,
        start_verse: u8,
        end_chapter: u8,
        end_verse: Option<u8>,
    },
}

use constcat::concat;

const OPT_DIGITS: &str = r"\d*";

/// This is basically `(?<sc>\d*)(:(?<sv>\d*))?(-(?<ec>\d*)(:(?<ev>\d*))?)?`
/// - `sc`: start chapter
/// - `sv`: start verse
/// - `ec`: end chapter
/// - `ev`: end verse
/// 
/// These groups do match empty when they have the symbol but no digits following it
#[rustfmt::skip]
const INCOMPLETE_SEGMENT_STR: &'static str = concat!(
    // `(?<sc>\d*)`
    "(?<sc>", WS, OPT_DIGITS, WS, ")",
    // `(:(?<sv>\d*))?`
    "(", CHAPTER_DELIMETER, "(?<sv>", WS, OPT_DIGITS, WS, "))?",
    // `(-(?<ec>\d*)(:(?<ev>\d*))?)?`
    "(",
        RANGE_DELIMETER, "(?<ec>", WS, OPT_DIGITS, WS, ")",
        "(", CHAPTER_DELIMETER, "(?<ev>", WS, OPT_DIGITS, WS, "))?",
    ")?"
);

static INCOMPLETE_SEGMENT: Lazy<Regex> =
    Lazy::new(|| Regex::new(&format!("^({INCOMPLETE_SEGMENT_STR})$")).unwrap());

impl IncompleteSegment {
    /// This expects to only receive only the unparsed incomplete segment
    pub fn new(input: &str) -> Option<Self> {
        let seg = INCOMPLETE_SEGMENT.captures(input)?;
        Some(Self::from_captures(seg))
        // dbg!(Some(Self::from_captures(seg)))
    }

    /// The groups
    // fn from_captures<'a>(cap: Captures<'a>) -> Self {
    //     let parse_cap = |group: &str| -> Option<u8> {
    //         cap.name(group)
    //             .map(|c| c.as_str().trim().parse().ok())
    //             .flatten()
    //     };
    //     let start_chapter = parse_cap("sc");
    //     let start_verse = parse_cap("sv");
    //     let end_chapter = parse_cap("ec");
    //     let end_verse = parse_cap("ev");
    //
    //     println!("{}", "-".repeat(80));
    //     println!("{}", cap.get(0).unwrap().as_str());
    //     dbg!(start_chapter, start_verse, end_chapter, end_verse);
    //     println!("{}", "-".repeat(80));
    //     dbg!(match (start_chapter, start_verse, end_chapter, end_verse) {
    //         // Segment: "1:1-2:" (verse)
    //         (Some(start_chapter), Some(start_verse), Some(end_chapter), end_verse) => {
    //             Self::ChapterVerseRangeTo {
    //                 start_chapter,
    //                 start_verse,
    //                 end_chapter,
    //                 end_verse,
    //             }
    //         }
    //         // Segment: "1-2:" (verse)
    //         (Some(start_chapter), None, Some(end_chapter), end_verse) => Self::ChapterRangeTo {
    //             start_chapter,
    //             end_chapter,
    //             end_verse,
    //         },
    //         // Segment: "1:1-" (chapter or verse)
    //         (Some(start_chapter), Some(start_verse), end, None) => Self::ChapterVerseTo {
    //             start_chapter,
    //             start_verse,
    //             end,
    //         },
    //         // Segment: "1:" (verse)
    //         (Some(start_chapter), start_verse, None, None) => Self::ChapterVerse {
    //             start_chapter,
    //             start_verse,
    //         },
    //         // Segment: "1-" (chapter or verse)
    //         (Some(start_chapter), None, end, None) => Self::ChapterTo { start_chapter, end },
    //         // Segment: "" (chapter)
    //         (start, None, None, None) => Self::ChapterOrVerse { start },
    //         _ => unreachable!(),
    //     })
    // }

    fn from_captures<'a>(cap: Captures<'a>) -> Self {
        use ValueOrFocused::*;
        let parse_cap = |group: &str| -> Option<ValueOrFocused> {
            cap.name(group).map(|c| match c.as_str().trim().parse() {
                Ok(v) => Value(v),
                Err(_) => Focused,
            })
        };
        let start_chapter = parse_cap("sc");
        let start_verse = parse_cap("sv");
        let end_chapter = parse_cap("ec");
        let end_verse = parse_cap("ev");

        // println!("{}", "-".repeat(80));
        // println!("{}", cap.get(0).unwrap().as_str());
        // dbg!(start_chapter, start_verse, end_chapter, end_verse);
        // println!("{}", "-".repeat(80));
        match (start_chapter, start_verse, end_chapter, end_verse) {
            // Segment: "" (chapter)
            (Some(start), None, None, None) => Self::ChapterOrVerse {
                start: start.as_option(),
            },
            // Segment: "1-" (chapter or verse)
            (Some(Value(start_chapter)), None, Some(end), None) => Self::ChapterOrVerseTo {
                start: start_chapter,
                end: end.as_option(),
            },
            // Segment: "1:" (verse)
            (Some(Value(start_chapter)), Some(start_verse), None, None) => Self::ChapterVerse {
                start_chapter,
                start_verse: start_verse.as_option(),
            },
            // Segment: "1:1-" (chapter or verse)
            (Some(Value(start_chapter)), Some(Value(start_verse)), Some(end), None) => {
                Self::ChapterVerseTo {
                    start_chapter,
                    start_verse,
                    end: end.as_option(),
                }
            }
            // Segment: "1-2:" (verse)
            (Some(Value(start_chapter)), None, Some(Value(end_chapter)), Some(end_verse)) => {
                Self::ChapterRangeTo {
                    start_chapter,
                    end_chapter,
                    end_verse: end_verse.as_option(),
                }
            }
            // Segment: "1:1-2:" (verse)
            (
                Some(Value(start_chapter)),
                Some(Value(start_verse)),
                Some(Value(end_chapter)),
                Some(end_verse),
            ) => Self::ChapterVerseRangeTo {
                start_chapter,
                start_verse,
                end_chapter,
                end_verse: end_verse.as_option(),
            },
            _ => unreachable!(),
        }
    }

    /**
    This will suggest segments that can be added to the input segments
    These segments can be suggesting chapters or verses or both
    */
    pub fn suggest(
        &self,
        chapter_verses: &ChapterVerses,
        prev: Option<&Segment>,
    ) -> Option<Vec<Segment>> {
        let last_chapter = chapter_verses.get_chapter_count();

        Some(match self.clone() {
            // the first number of all segments is always and only a chapter
            Self::ChapterOrVerse { start } => {
                if let Some(prev) = prev {
                    let next_verse = match prev.ending_verse() {
                        Some(cur) => cur + 1,
                        None => 1,
                    };
                    let current_chapter = prev.ending_chapter();
                    let next_chapter = current_chapter + 1;
                    let verses = (next_verse..=chapter_verses.get_last_verse(current_chapter)?)
                        .map(|v| Segment::chapter_verse(current_chapter, v))
                        .collect_vec();
                    verses
                } else {
                    (1..=last_chapter)
                        .map(|ch| Segment::full_chapter(ch))
                        .collect()
                }
            }

            Self::ChapterOrVerseTo { start, end } => {
                // I can use context to determine if start is a chapter or a verse
                if let Some(prev) = prev {
                    let is_chapter = prev.ending_verse().is_some();
                    if is_chapter {
                        let chapters = (start + 1..=last_chapter)
                            .map(|c| Segment::full_chapter_range(start, c))
                            .collect_vec();
                        chapters
                    } else {
                        let next_verse = match prev.ending_verse() {
                            Some(cur) => cur + 1,
                            None => 1,
                        };
                        let current_chapter = prev.ending_chapter();
                        let verses = (next_verse
                            ..=chapter_verses.get_last_verse(current_chapter)?)
                            .map(|v| Segment::chapter_verse(current_chapter, v))
                            .collect_vec();
                        verses
                    }
                } else {
                    let chapters = (start + 1..=last_chapter)
                        .map(|c| Segment::full_chapter_range(start, c))
                        .collect_vec();
                    chapters
                }
            }

            Self::ChapterVerse {
                start_chapter,
                start_verse,
            } => {
                let verses = (1..=chapter_verses.get_last_verse(start_chapter)?)
                    .map(|v| Segment::chapter_verse(start_chapter, v))
                    .collect_vec();
                verses
            }

            Self::ChapterVerseTo {
                start_chapter,
                start_verse,
                end,
            } => {
                let verses = (start_verse + 1..=chapter_verses.get_last_verse(start_chapter)?)
                    .map(|v| Segment::chapter_verse_range(start_chapter, start_verse, v))
                    .collect_vec();
                let chapters = (start_chapter + 1..=last_chapter)
                    .map(|c| Segment::chapter_range(start_chapter, start_verse, c, 1))
                    .collect_vec();
                verses.into_iter().chain(chapters).collect()
            }

            Self::ChapterRangeTo {
                start_chapter,
                end_chapter,
                end_verse,
            } => {
                let verses = (1..=chapter_verses.get_last_verse(end_chapter)?)
                    .map(|v| Segment::chapter_range(start_chapter, 1, end_chapter, v))
                    .collect_vec();
                verses
            }

            Self::ChapterVerseRangeTo {
                start_chapter,
                start_verse,
                end_chapter,
                end_verse,
            } => {
                let verses = (1..=chapter_verses.get_last_verse(end_chapter)?)
                    .map(|v| Segment::chapter_range(start_chapter, start_verse, end_chapter, v))
                    .collect_vec();
                verses
            }
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ValueOrFocused {
    Value(u8),
    Focused,
}
impl ValueOrFocused {
    pub fn as_option(self) -> Option<u8> {
        match self {
            ValueOrFocused::Value(v) => Some(v),
            ValueOrFocused::Focused => None,
        }
    }
}
