use chumsky::span::SimpleSpan;
use chumsky::text::whitespace;
use chumsky::{prelude::*, text::inline_whitespace};

use crate::components::{delim_chapter, delim_range, delim_segment};
use crate::roman_numerals::{ROMAN_NUMERALS, parse_roman_numeral};
use crate::{
    components::{Delimeter, decimal, optional_subverse},
    roman_numerals::only_roman_numerals,
};

#[derive(Clone, Debug)]
pub struct Spanned<T> {
    pub value: T,
    pub span: SimpleSpan,
}
impl<T> Spanned<T> {
    pub fn new(value: T, span: SimpleSpan) -> Self {
        Self { value, span }
    }
}

#[derive(Clone, Debug)]
pub struct VerboseDecimal<'a> {
    pub actual: Spanned<&'a str>,
    pub parsed: u8,
}

impl<'a> VerboseDecimal<'a> {
    pub fn parser() -> impl Parser<'a, &'a str, Self> {
        any()
            .filter(|c: &char| c.is_numeric())
            .repeated()
            .at_least(1)
            .at_most(3)
            .to_slice()
            .try_map(|slice, span| {
                let actual = Spanned::<&str>::new(slice, span);
                let parsed = slice.parse().map_err(|_| EmptyErr::default())?;
                Ok(Self { actual, parsed })
            })
    }
}

#[derive(Clone, Debug)]
pub struct VerboseRomanNumeral<'a> {
    pub actual: Spanned<&'a str>,
    pub parsed: u8,
}

impl<'a> VerboseRomanNumeral<'a> {
    pub fn parser() -> impl Parser<'a, &'a str, Self> {
        any()
            .filter(|c: &char| ROMAN_NUMERALS.contains(c))
            .repeated()
            .at_least(1)
            .at_most(9) // Just to keep the parser from getting trolled
            .to_slice()
            .try_map(|slice, span| {
                let actual = Spanned::<&str>::new(slice, span);
                let parsed = parse_roman_numeral(slice); //.map_err(|_| EmptyErr::default())?;
                Ok(Self { actual, parsed })
            })
    }
}

#[derive(Clone, Debug)]
pub enum VerboseNumberKind<'a> {
    Decimal(VerboseDecimal<'a>),
    Roman(VerboseRomanNumeral<'a>),
}

impl<'a> VerboseNumberKind<'a> {
    pub fn parser() -> impl Parser<'a, &'a str, Self> {
        VerboseDecimal::parser()
            .map(Self::Decimal)
            .or(VerboseRomanNumeral::parser().map(Self::Roman))
    }
}

#[derive(Clone, Debug)]
pub struct VerboseNumber<'a> {
    pub number: VerboseNumberKind<'a>,
    pub subverse: Option<Spanned<char>>,
}

impl<'a> VerboseNumber<'a> {
    pub fn parser() -> impl Parser<'a, &'a str, Self> {
        VerboseNumberKind::parser()
            .then(optional_subverse())
            .map_with(|(number, subverse), e| Self {
                number,
                subverse: subverse.map(|c| Spanned::new(c, e.span())),
            })
    }
}

#[derive(Clone, Debug)]
pub struct VerboseSpace<'a> {
    pub actual: Spanned<&'a str>,
}

impl<'a> VerboseSpace<'a> {
    pub fn parser() -> impl Parser<'a, &'a str, Self> {
        whitespace().to_slice().map_with(|space, e| Self {
            actual: Spanned::new(space, e.span()),
        })
    }
}

#[derive(Clone, Debug)]
pub struct VerboseDelimeter {
    pub actual: Spanned<char>,
    pub parsed: Delimeter,
}

impl VerboseDelimeter {
    pub fn segment_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
        delim_segment().map_with(|ch, e| Self {
            actual: Spanned::new(ch, e.span()),
            parsed: Delimeter::Segment,
        })
    }

    pub fn chapter_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
        delim_chapter().map_with(|ch, e| Self {
            actual: Spanned::new(ch, e.span()),
            parsed: Delimeter::Chapter,
        })
    }

    pub fn range_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
        delim_range().map_with(|ch, e| Self {
            actual: Spanned::new(ch, e.span()),
            parsed: Delimeter::Range,
        })
    }
}

#[derive(Clone, Debug)]
pub struct DelimitedNumber<'a> {
    pub delimeter: VerboseDelimeter,
    pub padded_number: FrontPadded<'a, VerboseNumber<'a>>,
}

pub type FrontPaddedDelimetedNumber<'a> = FrontPadded<'a, DelimitedNumber<'a>>;

/// Each atomic unit should be front-padded
#[derive(Clone, Debug)]
pub struct FrontPadded<'a, T> {
    pub space: VerboseSpace<'a>,
    pub value: T,
}

/**
- The reason leading whitespace is included is that this is to be used on the segments that come *right after* a matched book name
- This is a full segment, which all segments but potentially the last one are (it may be incomplete, as the user is still typing it)
*/
#[derive(Clone, Debug)]
pub struct VerboseFullSegment<'a> {
    /// `\s*\d+(\s*:\d+)?(\s*-\d+(\s*:\d+)?)?`
    /// `\s*\d+`
    pub start: FrontPadded<'a, VerboseNumber<'a>>,
    /// `\s*\d+(\s*:\d+)?(\s*-\d+(\s*:\d+)?)?`
    /// ----->`(\s*:\d+)?`
    pub explicit_start_verse: Option<FrontPaddedDelimetedNumber<'a>>,
    /// `\s*\d+(\s*:\d+)?(\s*-\d+(\s*:\d+)?)?`
    /// --------------->`(\s*-\d+(\s*:\d+)?)?`
    pub end: Option<(
        FrontPaddedDelimetedNumber<'a>,
        Option<FrontPaddedDelimetedNumber<'a>>,
    )>,
}

// fn verbose_delimeter<'a>() -> impl Parser<'a, &'a str, VerboseSpace<'a>> {
//     whitespace()
//         .to_slice()
//         .map_with(|space, e| Spanned::new(space, e.span()))
// }
//
// fn verbose_number<'a>() -> impl Parser<'a, &'a str, VerboseNumber<'a>> {
//     whitespace()
//         .to_slice()
//         .map_with(|space, e| Spanned::new(space, e.span()))
//         .then(decimal())
//         .or(only_roman_numerals())
//         .then(optional_subverse().or_not())
// }
