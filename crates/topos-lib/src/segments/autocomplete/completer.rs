// use crate::{
//     data::{
//         books::BookId,
//         chapter_verses::{BookChapterVerses, ChapterVerses},
//     },
//     matcher::{
//         instance::{BibleMatch, Location, Position},
//         matcher::BibleMatcher,
//     },
//     segments::{
//         autocomplete::joiner::SegmentJoiner,
//         segment::Segment,
//         segments::{Passage, Segments},
//         units::chapter_verse::ChapterVerse,
//         verse_bounds::VerseBounds,
//     },
// };
// use itertools::Itertools;
//
// pub struct SegmentAutoCompleter(pub BookChapterVerses);
//
// impl SegmentAutoCompleter {
//     pub fn suggest(
//         &self,
//         book: &BookId,
//         segments: &Segments,
//         joiner: Option<SegmentJoiner>,
//     ) -> Option<Vec<Segments>> {
//         let chapter_verses = self.0.get_chapter_verses(book)?;
//         let last_chapter = chapter_verses.get_chapter_count();
//
//         // extract last segment, but if there is not one, suggest every chapter
//         let Some(last) = segments.last() else {
//             let first_chapter = 1;
//             return Some(
//                 (1..=last_chapter)
//                     .map(|ch| Segment::full_chapter(ch).as_segments())
//                     .collect(),
//             );
//         };
//
//         let current_verse = last.ending_verse();
//         let current_chapter = last.ending_chapter();
//         let joiner = joiner?;
//
//         // preventing cases which look like `1-2-` or `1:1-2-` or `1-2:2-` or `1:1-2:2-`
//         if last.is_range() && joiner == SegmentJoiner::Range {
//             return None;
//         }
//
//         // preventing cases which look like `1:2:` or `1-2:2:` or `1:1-2:2:`
//         let second_to_last = segments.iter().rev().nth(1);
//         let has_previous_chapter_that_is_different =
//             second_to_last.is_some_and(|s| s.ending_chapter() != current_chapter);
//         // this means that it is `1:1` and not just `1`
//         let is_first_chapter_and_verse_provided =
//             second_to_last.is_none() && last.ending_verse().is_some();
//
//         let is_different_chapter =
//             has_previous_chapter_that_is_different || is_first_chapter_and_verse_provided;
//
//         if is_different_chapter && joiner == SegmentJoiner::Chapter {
//             return None;
//         }
//
//         let last_verse = chapter_verses.get_last_verse(current_chapter)?;
//
//         let remaining_verses = if let Some(current_verse) = current_verse {
//             (current_verse + 1..=last_verse)
//                 .map(|v| Segment::chapter_verse(current_chapter, v))
//                 .collect_vec()
//         } else {
//             if let SegmentJoiner::Chapter = joiner {
//                 (1..=last_verse)
//                     .map(|v| Segment::chapter_verse(current_chapter, v))
//                     .collect_vec()
//             } else {
//                 vec![]
//             }
//         };
//
//         Some(match joiner {
//             // if the joiner is a range, I will want to add
//             SegmentJoiner::Range => {
//                 // let before_range = &segments[0..segments.len() - 1];
//                 let mut before_range = segments.clone();
//                 before_range.pop();
//
//                 let mut results = vec![];
//                 for seg in remaining_verses {
//                     let mut prev = before_range.clone();
//                     let current_verse = current_verse.unwrap_or(1);
//                     // BUG: this is not right, i need to merge them
//                     let seg = if let Some(ending) = seg.ending_verse() {
//                         Segment::chapter_range(
//                             current_chapter,
//                             current_verse,
//                             seg.ending_chapter(),
//                             ending,
//                         )
//                     } else {
//                         Segment::chapter_verse_range(
//                             current_chapter,
//                             current_verse,
//                             seg.ending_chapter(),
//                         )
//                     };
//                     prev.push(seg);
//                     results.push(prev);
//                 }
//
//                 let remaining_chapters = if let Some(current_verse) = current_verse {
//                     (current_chapter + 1..=last_chapter)
//                         // BUG: I need some kind of way to indicate that the range is to the next
//                         // chapter, but where it does not have the starting verse; this should be a
//                         // separate segment type
//                         .map(|ch| Segment::chapter_range(current_chapter, current_verse, ch, 1))
//                         .collect_vec()
//                 } else {
//                     (current_chapter + 1..=last_chapter)
//                         .map(|ch| Segment::full_chapter_range(current_chapter, ch))
//                         .collect_vec()
//                 };
//
//                 for seg in remaining_chapters {
//                     let mut prev = before_range.clone();
//                     prev.push(seg);
//                     results.push(prev);
//                 }
//                 results
//             }
//             // if it is separate, just suggest things that are after this, both verses and
//             // chapters
//             SegmentJoiner::Separate => {
//                 let mut results = vec![];
//                 for seg in remaining_verses {
//                     let mut prev = segments.clone();
//                     prev.push(seg);
//                     results.push(prev);
//                 }
//                 let remaining_chapters = (current_chapter + 1..=last_chapter)
//                     .map(|ch| Segment::full_chapter(ch))
//                     .collect_vec();
//
//                 for seg in remaining_chapters {
//                     let mut prev = segments.clone();
//                     prev.push(seg);
//                     results.push(prev);
//                 }
//                 results
//             }
//             // I will only suggest verses after the user has given a `:`
//             SegmentJoiner::Chapter => {
//                 let mut results = vec![];
//                 let mut before_range = segments.clone();
//                 before_range.pop();
//                 for seg in remaining_verses {
//                     let mut prev = before_range.clone();
//                     prev.push(seg);
//                     results.push(prev);
//                 }
//                 results
//             }
//         })
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use crate::segments::autocomplete::input::InputAutoCompleter;
//
//     use super::*;
//     #[test]
//     fn complete() {
//         let engine = SegmentAutoCompleter(BookChapterVerses::default());
//
//         use SegmentJoiner::*;
//
//         // genesis has 50 chapters
//         // chapter 1 has 31 verses
//         // chapter 2 has 25 verses
//         let genesis = |input: &str, joiner: SegmentJoiner| {
//             engine
//                 .suggest(
//                     &BookId(1),
//                     &Segments::parse_str(input).unwrap(),
//                     Some(joiner),
//                 )
//                 .unwrap()
//                 .len()
//         };
//
//         let doesnt_parse = |input: &str, joiner: SegmentJoiner| -> bool {
//             engine
//                 .suggest(
//                     &BookId(1),
//                     &Segments::parse_str(input).unwrap(),
//                     Some(joiner),
//                 )
//                 .is_none()
//         };
//
//         assert_eq!(
//             engine
//                 .suggest(&BookId(1), &Segments::new(), None)
//                 .unwrap()
//                 .len(),
//             50 // chapters
//         );
//
//         // remaining chapters
//         assert_eq!(genesis("1", Range), 49);
//         assert_eq!(genesis("1", Separate), 49);
//         assert_eq!(genesis("1", Chapter), 31);
//
//         // ---
//
//         // remaining chapters + verses
//         assert_eq!(genesis("1:1", Range), 49 + 30);
//         assert_eq!(genesis("1:1", Separate), 49 + 30);
//
//         // ---
//
//         assert_eq!(genesis("1:2", Range), 49 + 29);
//         assert_eq!(genesis("1:2", Separate), 49 + 29);
//
//         // ---
//
//         assert_eq!(genesis("2", Range), 48);
//         assert_eq!(genesis("2", Separate), 48);
//         assert_eq!(genesis("2", Chapter), 25);
//
//         // ---
//
//         assert_eq!(genesis("2:1", Range), 48 + 24);
//         assert_eq!(genesis("2:1", Separate), 48 + 24);
//
//         // ---
//
//         assert_eq!(genesis("2:2", Range), 48 + 23);
//         assert_eq!(genesis("2:2", Separate), 48 + 23);
//
//         // ---
//
//         assert!(doesnt_parse("1-2", Range));
//         assert!(doesnt_parse("2-3", Range));
//
//         assert!(doesnt_parse("1:1-2", Range));
//         assert!(doesnt_parse("2:2-3", Range));
//
//         assert!(doesnt_parse("1-2:2", Range));
//         assert!(doesnt_parse("2-3:3", Range));
//
//         assert!(doesnt_parse("1:1-2:2", Range));
//         assert!(doesnt_parse("2:2-3:3", Range));
//
//         // ---
//
//         assert!(doesnt_parse("1:2", Chapter));
//         assert!(doesnt_parse("1-2:2", Chapter));
//         assert!(doesnt_parse("1:1-2:2", Chapter));
//
//         // ---
//
//         assert_eq!(genesis("1:1-2", Separate), 49 + 29);
//         assert_eq!(genesis("1:2-3", Separate), 49 + 28);
//
//         assert_eq!(genesis("1:1-2, 5", Separate), 49 + 26);
//         assert_eq!(genesis("1:2-3, 5", Separate), 49 + 26);
//         assert_eq!(genesis("1:1-2, 5", Range), 49 + 26);
//         assert_eq!(genesis("1:2-3, 5", Range), 49 + 26);
//         assert_eq!(genesis("1:1-2, 5", Chapter), 26);
//         assert_eq!(genesis("1:2-3, 5", Chapter), 26);
//
//         assert_eq!(genesis("1:1-2, 5-7", Separate), 49 + 24);
//         assert_eq!(genesis("1:2-3, 5-8", Separate), 49 + 23);
//
//         // ---
//     }
//
//     #[test]
//     fn suggest() {
//         let completer = SegmentAutoCompleter(BookChapterVerses::default());
//         let matcher = BibleMatcher::default();
//         let completer = InputAutoCompleter::new(&matcher, &completer);
//
//         for (idx, suggestion) in completer
//             // .suggest("Genesis") // bad
//             // .suggest("Genesis 1") // bad
//             // .suggest("Genesis 1:") // good
//             // .suggest("Genesis 1-") // good
//             .suggest("Genesis 1:1-") // good
//             // .suggest("Genesis 1:1-2,")
//             .unwrap()
//             .iter()
//             .enumerate()
//         {
//             // dbg!(&suggestion);
//             println!("{}. '{}'", idx, suggestion);
//         }
//     }
// }
