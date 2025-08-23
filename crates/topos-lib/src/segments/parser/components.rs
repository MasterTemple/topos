use std::u8;

use chumsky::prelude::*;

pub struct Decimal;
impl Decimal {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, u8> {
        any()
            .filter(|c: &char| c.is_numeric())
            .repeated()
            .at_least(1)
            .at_most(3)
            .to_slice()
            .from_str()
            .try_map(|v, span| v.map_err(|_| EmptyErr::default()))
    }
}

pub struct Subverse;
impl Subverse {
    const SUBVERSE: &str = r"abcd";
    pub fn parser<'a>() -> impl Parser<'a, &'a str, char> {
        one_of(Self::SUBVERSE)
    }
    pub fn optional_parser<'a>() -> impl Parser<'a, &'a str, Option<char>> {
        Self::parser().or_not()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Delimeter {
    Segment,
    Chapter,
    Range,
}

impl Delimeter {
    /// `,` or `;`
    const SEGMENT_DELIMETER: &'static str = r",;";
    pub fn segment_parser<'a>() -> impl Parser<'a, &'a str, char> {
        one_of(Self::SEGMENT_DELIMETER)
    }

    /// `.` or `:`
    const CHAPTER_DELIMETER: &'static str = r".:";
    pub fn chapter_parser<'a>() -> impl Parser<'a, &'a str, char> {
        one_of(Self::CHAPTER_DELIMETER)
    }

    /// Various dashes
    const RANGE_DELIMETER: &str = r"-–——⸺";
    pub fn range_parser<'a>() -> impl Parser<'a, &'a str, char> {
        one_of(Self::RANGE_DELIMETER)
    }
}
