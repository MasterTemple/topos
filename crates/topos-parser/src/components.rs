use std::u8;

use chumsky::prelude::*;

/// TODO: What is the most graceful way to fail when a 3-digit number given than 256 is given?
pub(crate) fn decimal<'a>() -> impl Parser<'a, &'a str, u8> {
    any()
        .filter(|c: &char| c.is_numeric())
        .repeated()
        .at_least(1)
        .at_most(3)
        .to_slice()
        .from_str()
        // .map_err(|e| EmptyErr::default())
        .map(|v| v.unwrap_or(u8::MAX))
    // .unwrapped()
}

#[derive(Clone, Copy, Debug)]
pub enum Delimeter {
    Segment,
    Chapter,
    Range,
}

/// To support verses like `Matthew 28:18b-20`
const SUBVERSE: &str = r"abcd";
pub(crate) fn optional_subverse<'a>() -> impl Parser<'a, &'a str, Option<char>> {
    one_of(SUBVERSE).or_not()
}

/// `,` or `;`
const SEGMENT_DELIMETER: &'static str = r",;";
pub(crate) fn delim_segment<'a>() -> impl Parser<'a, &'a str, char> {
    one_of(SEGMENT_DELIMETER)
}

/// `.` or `:`
const CHAPTER_DELIMETER: &'static str = r".:";
pub(crate) fn delim_chapter<'a>() -> impl Parser<'a, &'a str, char> {
    one_of(CHAPTER_DELIMETER)
}

/// Various dashes
const RANGE_DELIMETER: &str = r"-–——⸺";
pub(crate) fn delim_range<'a>() -> impl Parser<'a, &'a str, char> {
    one_of(RANGE_DELIMETER)
}
