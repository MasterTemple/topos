use constcat::concat;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    matcher::{
        instance::{BibleMatch, Location, Position},
        matcher::BibleMatcher,
    },
    segments::{
        autocomplete::{completer::SegmentAutoCompleter, joiner::SegmentJoiner},
        segments::{Passage, Segments},
    },
};

// with surrounding spaces
const DIGITS: &str = r" *\d{1,3} *";
const RANGE_DELIMETER: &str = r"[\-–——⸺]";
const CHAPTER_DELIMETER: &'static str = r"[\.:]";
const SEGMENT_DELIMETER: &'static str = r"[,;]";

/// `\d+`
const CHAPTER: &'static str = concat!(DIGITS);

/// `\d+:\d+`
const CHAPTER_VERSE: &'static str = concat!(DIGITS, CHAPTER_DELIMETER, DIGITS);

/// `\d+:\d+-\d+`
const CHAPTER_VERSE_RANGE: &'static str =
    concat!(DIGITS, CHAPTER_DELIMETER, DIGITS, RANGE_DELIMETER, DIGITS);

/// `\d+-\d+:\d+`
const CHAPTER_VERSE_RANGE_ALT: &'static str =
    concat!(DIGITS, RANGE_DELIMETER, DIGITS, CHAPTER_DELIMETER, DIGITS);

/// `\d+:\d+-\d+:\d+`
const CHAPTER_RANGE: &'static str = concat!(
    DIGITS,
    CHAPTER_VERSE,
    DIGITS,
    RANGE_DELIMETER,
    DIGITS,
    CHAPTER_VERSE,
    DIGITS
);

/// `\d+-\d+`
const FULL_CHAPTER_RANGE: &'static str = concat!(DIGITS, RANGE_DELIMETER, DIGITS);

/**
At runtime would be:
```ignore
let any_segment = format!("({})", [CHAPTER, CHAPTER_VERSE, CHAPTER_VERSE_RANGE, CHAPTER_VERSE_RANGE_ALT, CHAPTER_RANGE, FULL_CHAPTER_RANGE].join("|"));
```
*/
const ANY_SEGMENT: &'static str = concat!(
    "((?:)",
    CHAPTER,
    "|",
    CHAPTER_VERSE,
    "|",
    CHAPTER_VERSE_RANGE,
    "|",
    CHAPTER_VERSE_RANGE_ALT,
    "|",
    CHAPTER_RANGE,
    "|",
    FULL_CHAPTER_RANGE,
    ")",
);

const VALID_SEGMENTS: &'static str = concat!("((?:)", ANY_SEGMENT, SEGMENT_DELIMETER, ")*",);

// static VALID_SEGMENTS: Lazy<Regex> =
//     // Lazy::new(|| Regex::new(r"^ *\d+( *[\.,:;\-–——⸺] *\d+)*").unwrap());
//     // let valid_segments = format!("({}{})*", ANY_SEGMENT, SEGMENT_DELIMETER);
//     Lazy::new(|| {
//         let valid_segments = format!("({}{})*", ANY_SEGMENT, SEGMENT_DELIMETER);
//         Regex::new(r"^ *\d{1,3}( *[\.,:;\-–——⸺] *\d{1,3})*").unwrap()
//     });

/// This will match any book up to the segments
const BOOK: &'static str = r"^\d?\D+";

/// This is basically `\d+(:\d+)?(-\d+(:\d+)?)?`
const ANY_VALID_SEGMENT_STR: &'static str = concat!(
    DIGITS,
    "(",
    CHAPTER_DELIMETER,
    DIGITS,
    ")?",
    "(",
    RANGE_DELIMETER,
    DIGITS,
    "(",
    CHAPTER_DELIMETER,
    DIGITS,
    ")?",
    ")?"
);
static ANY_VALID_SEGMENT: Lazy<Regex> =
    // Lazy::new(|| Regex::new(r#"\d+(:\d+)?(-\d+(:\d+)?)?"#).unwrap());
    Lazy::new(|| Regex::new(ANY_VALID_SEGMENT_STR).unwrap());

/// This will group a match into a book, valid segments, and incomplete segments (which leaves off
/// final set of digits)
static GROUP_MATCH: Lazy<Regex> = Lazy::new(|| {
    // "\d+(:\d+)?(-)?(\d+(:\d+)?)";
    //
    Regex::new(format!("({})({})(.*){}", BOOK, VALID_SEGMENTS, DIGITS).as_str()).unwrap()
});

pub struct InputAutoCompleter<'a> {
    matcher: &'a BibleMatcher,
    completer: &'a SegmentAutoCompleter,
}

impl<'a> InputAutoCompleter<'a> {
    pub fn new(matcher: &'a BibleMatcher, completer: &'a SegmentAutoCompleter) -> Self {
        Self { matcher, completer }
    }
    pub fn suggest(&self, input: &str) -> Option<Vec<Segments>> {
        // self.matcher.data().
        let mat = self.matcher.find(input)?;
        dbg!(&mat);
        let BibleMatch {
            location: Location { start, end },
            psg: Passage { book, segments },
        } = &mat;
        // let Passage { book, segments } = &psg;
        // let Position { line, column } = &mat.location.end;

        let line = input.lines().nth(start.line - 1)?;
        dbg!(&line);
        let match_to_end_of_line = &line[start.column - 1..];
        let result = GROUP_MATCH.captures_iter(match_to_end_of_line).next();
        dbg!(result);

        let line = input.lines().nth(end.line - 1)?;
        let remaining = &line[end.column - 1..];
        dbg!(&remaining);
        let joiner = match remaining.trim() {
            ":" => Some(SegmentJoiner::Chapter),
            "-" => Some(SegmentJoiner::Range),
            // d if ALL_DASHES.contains(d) => Some(SegmentJoiner::Range),
            "." | "," => Some(SegmentJoiner::Separate),
            "" => None,
            _ => None?,
        };

        self.completer.suggest(book, segments, joiner)
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use crate::{
        data::chapter_verses::BookChapterVerses,
        matcher::matcher::BibleMatcher,
        segments::{
            autocomplete::{completer::SegmentAutoCompleter, input::InputAutoCompleter},
            segment::Segment,
        },
    };

    #[test]
    fn group_match() {
        let completer = SegmentAutoCompleter(BookChapterVerses::default());
        let matcher = BibleMatcher::default();
        let completer = InputAutoCompleter::new(&matcher, &completer);
        completer.suggest("Genesis 1:1");
    }

    #[test]
    fn test_regex() {
        // let re = Regex::new(r#"(?<sc>\d*)(?<sv>:\d*)?((?:)(?<ec>-\d*)(?<ev>:\d*)?)?"#).unwrap();
        let re = Regex::new(r#"(?<sc>\d*)(:(?<sv>\d*))?(-(?<ec>\d*)(:(?<ev>\d*))?)?"#).unwrap();

        let values = vec!["", "1-", "1:", "1:1-", "1-2:", "1:1-2:"];
        for v in values {
            if let Some(cap) = re.captures_iter(v).next() {
                // println!("Segment: {:?}", cap.get(0));
                println!("Segment: {:?}", v);
                println!("Start Chapter: {:?}", cap.name("sc").map(|c| c.as_str()));
                println!("Start Verse: {:?}", cap.name("sv").map(|c| c.as_str()));
                println!("End Chapter: {:?}", cap.name("ec").map(|c| c.as_str()));
                println!("End Verse: {:?}", cap.name("ev").map(|c| c.as_str()));
            }
            println!("----------------------");
        }
    }
}
