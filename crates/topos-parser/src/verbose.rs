use chumsky::span::SimpleSpan;
use chumsky::text::whitespace;
use chumsky::{prelude::*, text::inline_whitespace};

use crate::components::{delim_chapter, delim_range, delim_segment};
use crate::roman_numerals::{ROMAN_NUMERALS, parse_roman_numeral};
use crate::{
    components::{Delimeter, decimal, optional_subverse},
    roman_numerals::only_roman_numerals,
};

pub trait FullSpan {
    fn full_span(&self) -> SimpleSpan;
    fn full_span_start(&self) -> usize {
        self.full_span().start
    }
    fn full_span_end(&self) -> usize {
        self.full_span().end
    }
}

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

impl<T> FullSpan for Spanned<T> {
    fn full_span(&self) -> SimpleSpan {
        self.span
    }
}

#[derive(Clone, Debug)]
pub struct VerboseDecimal<'a> {
    pub actual: Spanned<&'a str>,
    pub parsed: u8,
}

impl<'a> FullSpan for VerboseDecimal<'a> {
    fn full_span(&self) -> SimpleSpan {
        self.actual.full_span()
    }
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

impl<'a> FullSpan for VerboseRomanNumeral<'a> {
    fn full_span(&self) -> SimpleSpan {
        self.actual.full_span()
    }
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

impl<'a> FullSpan for VerboseNumberKind<'a> {
    fn full_span(&self) -> SimpleSpan {
        match self {
            VerboseNumberKind::Decimal(num) => num.full_span(),
            VerboseNumberKind::Roman(num) => num.full_span(),
        }
    }
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

impl<'a> FullSpan for VerboseNumber<'a> {
    fn full_span(&self) -> SimpleSpan {
        if let Some(subverse) = &self.subverse {
            let start = self.number.full_span_start();
            let end = subverse.full_span_end();
            SimpleSpan::from(start..end)
        } else {
            self.number.full_span()
        }
    }
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

impl<'a> FullSpan for VerboseSpace<'a> {
    fn full_span(&self) -> SimpleSpan {
        self.actual.full_span()
    }
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

impl FullSpan for VerboseDelimeter {
    fn full_span(&self) -> SimpleSpan {
        self.actual.full_span()
    }
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

impl<'a> FullSpan for DelimitedNumber<'a> {
    fn full_span(&self) -> SimpleSpan {
        let start = self.delimeter.full_span_start();
        let end = self.padded_number.full_span_end();
        SimpleSpan::from(start..end)
    }
}

pub type FrontPaddedDelimetedNumber<'a> = FrontPadded<'a, DelimitedNumber<'a>>;

/// Each atomic unit should be front-padded
#[derive(Clone, Debug)]
pub struct FrontPadded<'a, T: FullSpan> {
    pub space: VerboseSpace<'a>,
    pub value: T,
}

impl<'a, T: FullSpan> FullSpan for FrontPadded<'a, T> {
    fn full_span(&self) -> SimpleSpan {
        let start = self.space.full_span_start();
        let end = self.value.full_span_end();
        SimpleSpan::from(start..end)
    }
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

impl<'a> FullSpan for VerboseFullSegment<'a> {
    fn full_span(&self) -> SimpleSpan {
        let start = self.start.full_span_start();
        let end = if let Some(end) = &self.end {
            if let Some(end) = &end.1 {
                end.full_span_end()
            } else {
                end.0.full_span_end()
            }
        } else {
            if let Some(end) = &self.explicit_start_verse {
                end.full_span_end()
            } else {
                self.start.full_span_end()
            }
        };
        SimpleSpan::from(start..end)
    }
}
