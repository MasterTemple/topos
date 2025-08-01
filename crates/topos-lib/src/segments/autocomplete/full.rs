use constcat::concat;
use once_cell::sync::Lazy;
use regex::{Match, Regex};

use crate::segments::segments::Segments;

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
const ANY_FULL_SEGMENT_STR: &'static str = concat!(
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

const FULL_SEGMENTS_STR: &'static str =
    concat!("((?:)", ANY_FULL_SEGMENT_STR, SEGMENT_DELIMETER, ")*",);

static FULL_SEGMENTS: Lazy<Regex> = Lazy::new(|| Regex::new(FULL_SEGMENTS_STR).unwrap());

pub(super) fn parse_full_segments<'a>(input: &'a str) -> Option<(Match<'a>, Segments)> {
    let mat = FULL_SEGMENTS.captures(input)?.get(0)?;

    let full_segments = if mat.as_str().len() == 0 {
        Segments::new()
    } else {
        Segments::parse_str(mat.as_str())?
    };

    Some((mat, full_segments))
}
