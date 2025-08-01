use constcat::concat;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    matcher::{
        instance::{BibleMatch, Location, Position},
        matcher::BibleMatcher,
    },
    segments::{
        autocomplete::{
            completer::SegmentAutoCompleter, incomplete::IncompleteSegment, joiner::SegmentJoiner,
        },
        segments::{Passage, Segments},
    },
};

/// TODO: parse Roman Numerals
/// - https://stackoverflow.com/questions/267399/how-do-you-match-only-valid-roman-numerals-with-a-regular-expression
///
/// Currently just parses a 3-character long set of digits
pub const DIGITS: &str = r"\d{1,3}";
/// Various dashes
pub const RANGE_DELIMETER: &str = r"[\-–——⸺]";
/// `.` or `:`
pub const CHAPTER_DELIMETER: &'static str = r"[\.:]";
/// `,` or `;`
pub const SEGMENT_DELIMETER: &'static str = r"[,;]";
/// Any whitespace
pub const WS: &str = r"\s*";

/// This will match any book up to the segments
const BOOK: &'static str = r"^\d?\D+";

/// This is basically `\d+(:\d+)?(-\d+(:\d+)?)?`
const ANY_VALID_SEGMENT_STR: &'static str = concat!(
    WS,
    DIGITS,
    WS,
    "(",
    CHAPTER_DELIMETER,
    WS,
    DIGITS,
    WS,
    ")?",
    "(",
    RANGE_DELIMETER,
    WS,
    DIGITS,
    WS,
    "(",
    CHAPTER_DELIMETER,
    WS,
    DIGITS,
    WS,
    ")?",
    ")?"
);

const VALID_SEGMENTS: &'static str =
    concat!("((?:)", ANY_VALID_SEGMENT_STR, SEGMENT_DELIMETER, ")*",);

static ANY_VALID_SEGMENT: Lazy<Regex> =
    // Lazy::new(|| Regex::new(r#"\d+(:\d+)?(-\d+(:\d+)?)?"#).unwrap());
    Lazy::new(|| Regex::new(ANY_VALID_SEGMENT_STR).unwrap());

const OPT_DIGITS: &str = r"\d*";
/// BUG: these DIGITS dont work
///
/// This is basically `(?<sc>\d*)(:(?<sv>\d*))?(-(?<ec>\d*)(:(?<ev>\d*))?)?`
#[rustfmt::skip]
const INCOMPLETE_SEGMENT: &'static str = concat!(
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

pub struct GroupedMatchStr<'a> {
    book: &'a str,
    valid: &'a str,
    incomplete: &'a str,
}

impl<'a> GroupedMatchStr<'a> {
    pub fn new(input: &'a str) -> Option<Self> {
        let cap = GROUP_MATCH.captures_iter(input).next()?;
        let book = cap.name("book")?.as_str();
        let valid = cap.name("valid")?.as_str();
        let incomplete = cap.name("incomplete")?.as_str();
        Some(Self {
            book,
            valid,
            incomplete,
        })
    }
}

/// This will group a match into a book, valid segments, and incomplete segments (which leaves off
/// final set of digits)
static GROUP_MATCH: Lazy<Regex> = Lazy::new(|| {
    // "\d+(:\d+)?(-)?(\d+(:\d+)?)";
    //
    // Regex::new(
    //     format!(
    //         "({})({})({}){}",
    //         BOOK, VALID_SEGMENTS, INCOMPLETE_SEGMENT, DIGITS
    //     )
    //     .as_str(),
    // )
    // .unwrap()
    Regex::new(
        // FIX: I shouldn't even need to match the incomplete part, i should just match book and
        // valid segments and then from there, try matching an incomplete segment
        // but how do i get rid of the last digits? i can just remove them from the end of the
        // input, and then the rest should parse exactly into an incomplete segment
        // if they are beyond it like `John 3:16, is a verse that ..' then I don't need to be auto
        // completeting
        format!(
            r#"^(?<book>{})(?<valid>{})(?<incomplete>.*)\d*"#,
            BOOK, VALID_SEGMENTS
        )
        .as_str(),
    )
    .unwrap()
});

static TRAILING_DIGITS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\d*$").unwrap());

pub struct InputAutoCompleter<'a> {
    matcher: &'a BibleMatcher,
    completer: &'a SegmentAutoCompleter,
}

impl<'a> InputAutoCompleter<'a> {
    pub fn new(matcher: &'a BibleMatcher, completer: &'a SegmentAutoCompleter) -> Self {
        Self { matcher, completer }
    }

    /// - This assumes your cursor is at the end of the input
    pub fn complete(&self, input: &str) -> Option<Vec<Segments>> {
        // cache this later
        let re = self.matcher.data().create_book_regex().unwrap();
        let cap = re.captures_iter(input).last()?;
        let book_match = cap.get(1).unwrap();
        // dbg!(&book_match.as_str());
        let book_id = self.matcher.data().books().search(book_match.as_str())?;
        let segments_input = &input[book_match.end()..];
        // dbg!(&segments_input);
        let valid_segments = Regex::new(&format!("^{VALID_SEGMENTS}"))
            .unwrap()
            .captures(segments_input)?
            .get(0)?;

        let mut incomplete_segments_input = &segments_input[valid_segments.end()..];

        let full_segments = if valid_segments.as_str().len() == 0 {
            Segments::new()
        } else {
            Segments::parse_str(valid_segments.as_str())?
        };
        let incomplete_segment = IncompleteSegment::new(incomplete_segments_input)?;

        None
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

        // let line = input.lines().nth(start.line - 1)?;
        // dbg!(&line);
        // let match_to_end_of_line = &line[start.column - 1..];
        // let result = GROUP_MATCH.captures_iter(match_to_end_of_line).next();
        // dbg!(result);

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
            autocomplete::{
                completer::SegmentAutoCompleter,
                input::{GROUP_MATCH, InputAutoCompleter},
            },
            segment::Segment,
        },
    };

    #[test]
    fn group_match() {
        let completer = SegmentAutoCompleter(BookChapterVerses::default());
        let matcher = BibleMatcher::default();
        let completer = InputAutoCompleter::new(&matcher, &completer);
        // completer.suggest("Genesis 1:1");
        let values = vec![
            "Genesis",
            "Genesis ",
            "Genesis 1",
            "Genesis 1:",
            "Genesis 1:1",
            "Genesis 1:1,",
            "Genesis 1:1-",
            "Genesis 1:1,2",
            "Genesis 1:1-2",
            "Genesis 1:1,2,",
            "Genesis 1:1-2:",
            "Genesis 1:1-2:3",
            "Genesis 1:1-2:3,",
        ];
        for val in values {
            let cap = GROUP_MATCH.captures_iter(val).next().unwrap();
            println!("Value: {:?}", val);
            println!("book: {:?}", cap.name("book").map(|c| c.as_str()));
            println!("valid: {:?}", cap.name("valid").map(|c| c.as_str()));
            println!(
                "incomplete: {:?}",
                cap.name("incomplete").map(|c| c.as_str())
            );
            // println!(
            //     "incomplete: {:?}",
            //     cap.name("incomplete")
            //         .map(|c| c.as_str().trim_end_matches(char::is_numeric))
            // );
            println!("----------------------");
        }
    }

    #[test]
    fn test_regex() {
        // let re = Regex::new(r#"(?<sc>\d*)(?<sv>:\d*)?((?:)(?<ec>-\d*)(?<ev>:\d*)?)?"#).unwrap();
        // let re = Regex::new(r#"(?<sc>\d*)(:(?<sv>\d*))?(-(?<ec>\d*)(:(?<ev>\d*))?)?"#).unwrap();
        let re = Regex::new(r#"^(?<sc>\d*)(:(?<sv>\d*))?(-(?<ec>\d*)(:(?<ev>\d*))?)?\d*"#).unwrap();

        let mut values = vec!["", "1-", "1:", "1:1-", "1-2:", "1:1-2:"];
        // let values = vec!["9", "1-9", "1:9", "1:1-9", "1-2:9", "1:1-2:9"];
        values.extend(["9", "1-9", "1:9", "1:1-9", "1-2:9", "1:1-2:9"]);
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

    #[test]
    fn test_complete() {
        let completer = SegmentAutoCompleter(BookChapterVerses::default());
        let matcher = BibleMatcher::default();
        let completer = InputAutoCompleter::new(&matcher, &completer);

        let mut values = vec!["", "1-", "1:", "1:1-", "1-2:", "1:1-2:"];
        values.extend(["9", "1-9", "1:9", "1:1-9", "1-2:9", "1:1-2:9"]);
        values.extend(["1:1-2:9,", "1:1-2:9,3", "1:1-2:9,3-", "1:1-2:9,3- hi"]);
        for v in values {
            completer.complete(&format!("Genesis {v}"));
        }
        // completer.complete(&format!("Genesis 1:1-2,3:"));
    }
}
