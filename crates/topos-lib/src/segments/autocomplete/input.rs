use constcat::concat;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    data::chapter_verses::BookChapterVerses,
    matcher::{
        instance::{BibleMatch, Location, Position},
        matcher::BibleMatcher,
    },
    segments::{
        autocomplete::{
            full::parse_full_segments, incomplete::IncompleteSegment, output::CompletionOutput,
        },
        segments::{Passage, Segments},
    },
};

pub struct InputAutoCompleter<'a> {
    matcher: &'a BibleMatcher,
    book_regex: Regex,
    // bcv: BookChapterVerses
}

impl<'a> InputAutoCompleter<'a> {
    pub fn new(matcher: &'a BibleMatcher) -> Self {
        let book_regex = matcher.data().create_book_regex().unwrap();
        Self {
            matcher,
            book_regex,
        }
    }

    /// - This assumes your cursor is at the end of the input
    pub fn suggest(&self, input: &str) -> Option<CompletionOutput> {
        let cap = self.book_regex.captures_iter(input).last()?;
        let book_match = cap.get(1).unwrap();
        let book_id = self.matcher.data().books().search(book_match.as_str())?;

        let segments_input = &input[book_match.end()..];
        let (mat, full_segments) = parse_full_segments(segments_input)?;

        let incomplete_segments_input = &segments_input[mat.end()..];
        let incomplete_segment = IncompleteSegment::new(incomplete_segments_input)?;

        let chapter_verses = self
            .matcher
            .data()
            .chapter_verses()
            .get_chapter_verses(&book_id)?;
        let suggestions = incomplete_segment.suggest(chapter_verses, full_segments.last())?;

        Some(CompletionOutput::new(book_id, full_segments, suggestions))
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use crate::{
        matcher::matcher::BibleMatcher, segments::autocomplete::input::InputAutoCompleter,
    };

    // use crate::{
    //     data::chapter_verses::BookChapterVerses,
    //     matcher::matcher::BibleMatcher,
    //     segments::{
    //         autocomplete::{
    //             completer::SegmentAutoCompleter,
    //             input::{GROUP_MATCH, InputAutoCompleter},
    //         },
    //         segment::Segment,
    //     },
    // };
    //
    // #[test]
    // fn group_match() {
    //     let completer = SegmentAutoCompleter(BookChapterVerses::default());
    //     let matcher = BibleMatcher::default();
    //     let completer = InputAutoCompleter::new(&matcher, &completer);
    //     // completer.suggest("Genesis 1:1");
    //     let values = vec![
    //         "Genesis",
    //         "Genesis ",
    //         "Genesis 1",
    //         "Genesis 1:",
    //         "Genesis 1:1",
    //         "Genesis 1:1,",
    //         "Genesis 1:1-",
    //         "Genesis 1:1,2",
    //         "Genesis 1:1-2",
    //         "Genesis 1:1,2,",
    //         "Genesis 1:1-2:",
    //         "Genesis 1:1-2:3",
    //         "Genesis 1:1-2:3,",
    //     ];
    //     for val in values {
    //         let cap = GROUP_MATCH.captures_iter(val).next().unwrap();
    //         println!("Value: {:?}", val);
    //         println!("book: {:?}", cap.name("book").map(|c| c.as_str()));
    //         println!("valid: {:?}", cap.name("valid").map(|c| c.as_str()));
    //         println!(
    //             "incomplete: {:?}",
    //             cap.name("incomplete").map(|c| c.as_str())
    //         );
    //         // println!(
    //         //     "incomplete: {:?}",
    //         //     cap.name("incomplete")
    //         //         .map(|c| c.as_str().trim_end_matches(char::is_numeric))
    //         // );
    //         println!("----------------------");
    //     }
    // }
    //
    // #[test]
    // fn test_regex() {
    //     // let re = Regex::new(r#"(?<sc>\d*)(?<sv>:\d*)?((?:)(?<ec>-\d*)(?<ev>:\d*)?)?"#).unwrap();
    //     // let re = Regex::new(r#"(?<sc>\d*)(:(?<sv>\d*))?(-(?<ec>\d*)(:(?<ev>\d*))?)?"#).unwrap();
    //     let re = Regex::new(r#"^(?<sc>\d*)(:(?<sv>\d*))?(-(?<ec>\d*)(:(?<ev>\d*))?)?\d*"#).unwrap();
    //
    //     let mut values = vec!["", "1-", "1:", "1:1-", "1-2:", "1:1-2:"];
    //     // let values = vec!["9", "1-9", "1:9", "1:1-9", "1-2:9", "1:1-2:9"];
    //     values.extend(["9", "1-9", "1:9", "1:1-9", "1-2:9", "1:1-2:9"]);
    //     for v in values {
    //         if let Some(cap) = re.captures_iter(v).next() {
    //             // println!("Segment: {:?}", cap.get(0));
    //             println!("Segment: {:?}", v);
    //             println!("Start Chapter: {:?}", cap.name("sc").map(|c| c.as_str()));
    //             println!("Start Verse: {:?}", cap.name("sv").map(|c| c.as_str()));
    //             println!("End Chapter: {:?}", cap.name("ec").map(|c| c.as_str()));
    //             println!("End Verse: {:?}", cap.name("ev").map(|c| c.as_str()));
    //         }
    //         println!("----------------------");
    //     }
    // }
    //
    // #[test]
    // fn test_complete() {
    //     let completer = SegmentAutoCompleter(BookChapterVerses::default());
    //     let matcher = BibleMatcher::default();
    //     let completer = InputAutoCompleter::new(&matcher, &completer);
    //
    //     let mut values = vec!["", "1-", "1:", "1:1-", "1-2:", "1:1-2:"];
    //     values.extend(["9", "1-9", "1:9", "1:1-9", "1-2:9", "1:1-2:9"]);
    //     values.extend(["1:1-2:9,", "1:1-2:9,3", "1:1-2:9,3-", "1:1-2:9,3- hi"]);
    //     for v in values {
    //         completer.complete(&format!("Genesis {v}"));
    //     }
    //     // completer.complete(&format!("Genesis 1:1-2,3:"));
    // }
    #[test]
    fn test_suggest() {
        let matcher = BibleMatcher::default();
        let completer = InputAutoCompleter::new(&matcher);

        let mut values = vec!["", "1-", "1:", "1:1-", "1-2:", "1:1-2:"];
        values.extend(["9", "1-9", "1:9", "1:1-9", "1-2:9", "1:1-2:9"]);
        values.extend(["1:1-2:9,", "1:1-2:9,3", "1:1-2:9,3-", "1:1-2:9,3- hi"]);
        let bk = "Genesis ";
        for v in values.into_iter().take(3) {
            let input = &format!("{bk}{v}");
            if let Some(result) = completer.suggest(input) {
                println!("{input}");
                let total = result.suggestions.len();
                for (idx, sug) in result.suggestions.into_iter().enumerate() {
                    let segs = result.segments.with_suggestion(sug);
                    println!("{}{}", " ".repeat(bk.len()), segs);
                    // if idx > 5 {
                    //     println!("Total: {total}");
                    //     break;
                    // }
                }
                println!("{}", "-".repeat(80));
            }
        }
        // completer.complete(&format!("Genesis 1:1-2,3:"));
    }
}
