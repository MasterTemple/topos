// use once_cell::sync::Lazy;
// use regex::{Captures, Regex};
//
// use crate::segments::{
//     autocomplete::input::{CHAPTER_DELIMETER, RANGE_DELIMETER, WS},
//     segment::Segment,
// };
//
// pub enum IncompleteSegment {
//     // - Example Segment: ""
//     // - Suggests: chapters
//     Chapter {
//         start_chapter: Option<u8>,
//     },
//     // - Example Segment: "1-"
//     // - Suggests: chapters or verses
//     ChapterTo {
//         start_chapter: u8,
//         end: Option<u8>,
//     },
//     // - Example Segment: "1:"
//     // - Suggests: verses
//     ChapterVerse {
//         start_chapter: u8,
//         start_verse: Option<u8>,
//     },
//     // - Example Segment: "1:1-"
//     // - Suggests: chapters or verses
//     ChapterVerseTo {
//         start_chapter: u8,
//         start_verse: u8,
//         end: Option<u8>,
//     },
//     // - Example Segment: "1-2:"
//     // - Suggests: verses
//     ChapterRangeTo {
//         start_chapter: u8,
//         end_chapter: u8,
//         end_verse: Option<u8>,
//     },
//     // - Example Segment: "1:1-2:"
//     // - Suggests: verses
//     ChapterVerseRangeTo {
//         start_chapter: u8,
//         start_verse: u8,
//         end_chapter: u8,
//         end_verse: Option<u8>,
//     },
// }
//
// use constcat::concat;
//
// const OPT_DIGITS: &str = r"\d*";
// /// This is basically `(?<sc>\d*)(:(?<sv>\d*))?(-(?<ec>\d*)(:(?<ev>\d*))?)?`
// /// - `sc`: start chapter
// /// - `sv`: start verse
// /// - `ec`: end chapter
// /// - `ev`: end verse
// ///
// /// These groups do match empty when they have the symbol but no digits following it
// #[rustfmt::skip]
// const INCOMPLETE_SEGMENT_STR: &'static str = concat!(
//     // `(?<sc>\d*)`
//     "(?<sc>", WS, OPT_DIGITS, WS, ")",
//     // `(:(?<sv>\d*))?`
//     "(", CHAPTER_DELIMETER, "(?<sv>", WS, OPT_DIGITS, WS, "))?",
//     // `(-(?<ec>\d*)(:(?<ev>\d*))?)?`
//     "(",
//         RANGE_DELIMETER, "(?<ec>", WS, OPT_DIGITS, WS, ")",
//         "(", CHAPTER_DELIMETER, "(?<ev>", WS, OPT_DIGITS, WS, "))?",
//     ")?"
// );
//
// static INCOMPLETE_SEGMENT: Lazy<Regex> =
//     Lazy::new(|| Regex::new(&format!("^({INCOMPLETE_SEGMENT_STR})$")).unwrap());
//
// impl IncompleteSegment {
//     /// This expects to only receive only the unparsed incomplete segment
//     pub fn new(input: &str) -> Option<Self> {
//         let seg = INCOMPLETE_SEGMENT.captures(input)?;
//         Some(Self::from_captures(seg))
//     }
//
//     /// The groups
//     fn from_captures<'a>(cap: Captures<'a>) -> Self {
//         let parse_cap = |group: &str| -> Option<u8> {
//             cap.name(group).map(|c| c.as_str().parse().ok()).flatten()
//         };
//         let start_chapter = parse_cap("sc");
//         let start_verse = parse_cap("sv");
//         let end_chapter = parse_cap("ec");
//         let end_verse = parse_cap("ev");
//
//         match (start_chapter, start_verse, end_chapter, end_verse) {
//             // Segment: "" (chapter)
//             (start_chapter, None, None, None) => Self::Chapter { start_chapter },
//             // Segment: "1-" (chapter or verse)
//             (Some(start_chapter), None, end, None) => Self::ChapterTo { start_chapter, end },
//             // Segment: "1:" (verse)
//             (Some(start_chapter), start_verse, None, None) => Self::ChapterVerse {
//                 start_chapter,
//                 start_verse,
//             },
//             // Segment: "1:1-" (chapter or verse)
//             (Some(start_chapter), Some(start_verse), end, None) => Self::ChapterVerseTo {
//                 start_chapter,
//                 start_verse,
//                 end,
//             },
//             // Segment: "1-2:" (verse)
//             (Some(start_chapter), None, Some(end_chapter), end_verse) => Self::ChapterRangeTo {
//                 start_chapter,
//                 end_chapter,
//                 end_verse,
//             },
//             // Segment: "1:1-2:" (verse)
//             (Some(start_chapter), Some(start_verse), Some(end_chapter), end_verse) => {
//                 Self::ChapterVerseRangeTo {
//                     start_chapter,
//                     start_verse,
//                     end_chapter,
//                     end_verse,
//                 }
//             }
//             _ => unreachable!(),
//         }
//     }
// }
