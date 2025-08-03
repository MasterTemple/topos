use chumsky::prelude::*;

pub(crate) fn decimal<'a>() -> impl Parser<'a, &'a str, u8> {
    any()
        .filter(|c: &char| c.is_numeric())
        .repeated()
        .at_least(1)
        .at_most(3)
        .to_slice()
        .from_str()
        .unwrapped()
}

/// To support verses like `Matthew 28:18b-20`
const SUBVERSE: &str = r"abc";
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
