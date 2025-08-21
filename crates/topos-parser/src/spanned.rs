use chumsky::{prelude::*, text::whitespace};
use from_nested_tuple::FromTuple;

use crate::{
    components::{Delimeter, SUBVERSE, delim_chapter, delim_range, delim_segment},
    roman_numerals::{ROMAN_NUMERALS, parse_roman_numeral},
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

impl FullSpan for SimpleSpan {
    fn full_span(&self) -> SimpleSpan {
        self.clone()
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

    pub fn parser<'a>(child: impl Parser<'a, &'a str, T>) -> impl Parser<'a, &'a str, Self> {
        child.map_with(|value, e| Self::new(value, e.span()))
    }
}

impl<T> FullSpan for Spanned<T> {
    fn full_span(&self) -> SimpleSpan {
        self.span
    }
}

#[derive(Clone, Debug)]
pub struct VerboseDecimal {
    pub span: SimpleSpan,
    pub parsed: u8,
}

impl VerboseDecimal {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        any()
            .filter(|c: &char| c.is_numeric())
            .repeated()
            .at_least(1)
            .at_most(3)
            .to_slice()
            .try_map(|slice: &str, span| {
                let parsed = slice.parse().map_err(|_| EmptyErr::default())?;
                Ok(Self { span, parsed })
            })
    }
}

impl FullSpan for VerboseDecimal {
    fn full_span(&self) -> SimpleSpan {
        self.span.full_span()
    }
}

#[derive(Clone, Debug)]
pub struct VerboseRomanNumeral {
    pub span: SimpleSpan,
    pub parsed: u8,
}

impl VerboseRomanNumeral {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        any()
            .filter(|c: &char| ROMAN_NUMERALS.contains(c))
            .repeated()
            .at_least(1)
            .at_most(9) // Just to keep the parser from getting trolled
            .to_slice()
            .try_map(|slice, span| {
                let parsed = parse_roman_numeral(slice); //.map_err(|_| EmptyErr::default())?;
                Ok(Self { span, parsed })
            })
    }
}

impl FullSpan for VerboseRomanNumeral {
    fn full_span(&self) -> SimpleSpan {
        self.span.full_span()
    }
}

#[derive(Clone, Debug)]
pub enum VerboseNumberKind {
    Decimal(VerboseDecimal),
    Roman(VerboseRomanNumeral),
}

impl VerboseNumberKind {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        VerboseDecimal::parser()
            .map(Self::Decimal)
            .or(VerboseRomanNumeral::parser().map(Self::Roman))
    }
}

impl FullSpan for VerboseNumberKind {
    fn full_span(&self) -> SimpleSpan {
        match self {
            VerboseNumberKind::Decimal(num) => num.full_span(),
            VerboseNumberKind::Roman(num) => num.full_span(),
        }
    }
}

#[derive(Clone, Debug, FromTuple)]
pub struct VerboseNumber {
    pub number: VerboseNumberKind,
    pub subverse: Option<char>,
}

impl VerboseNumber {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        VerboseNumberKind::parser()
            .then(one_of(SUBVERSE).or_not())
            .map(FromTuple::from_tuple)
    }
}

impl FullSpan for VerboseNumber {
    fn full_span(&self) -> SimpleSpan {
        if let Some(_) = &self.subverse {
            let start = self.number.full_span_start();
            let end = self.number.full_span_end() + 1;
            SimpleSpan::from(start..end)
        } else {
            self.number.full_span()
        }
    }
}

// TODO: I technically don't need to store the span of spaces, since I can calculate them between non-space characters
#[derive(Clone, Debug)]
pub struct VerboseSpace {
    pub span: SimpleSpan,
}

impl VerboseSpace {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        whitespace().to_span().map(|span| Self { span })
    }

    pub fn optional_parser<'a>() -> impl Parser<'a, &'a str, Option<Self>> {
        whitespace()
            .at_least(1)
            .to_span()
            .or_not()
            .map(|span| span.map(|span| Self { span }))
    }
}

impl FullSpan for VerboseSpace {
    fn full_span(&self) -> SimpleSpan {
        self.span.full_span()
    }
}

#[derive(Clone, Debug)]
pub struct VerboseDelimeter {
    pub actual: char,
    pub parsed: Delimeter,
}

impl VerboseDelimeter {
    pub fn segment_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
        delim_segment().map(|actual| Self {
            actual,
            parsed: Delimeter::Segment,
        })
    }

    pub fn chapter_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
        delim_chapter().map(|actual| Self {
            actual,
            parsed: Delimeter::Chapter,
        })
    }

    pub fn range_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
        delim_range().map(|actual| Self {
            actual,
            parsed: Delimeter::Range,
        })
    }
}

// #[derive(Clone, Debug)]
// pub struct VerboseDelimeter {
//     pub actual: SimpleSpan,
//     pub parsed: Delimeter,
// }
//
// impl VerboseDelimeter {
//     pub fn segment_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
//         delim_segment().to_span().map(|actual| Self {
//             actual,
//             parsed: Delimeter::Segment,
//         })
//     }
//
//     pub fn chapter_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
//         delim_chapter().to_span().map(|actual| Self {
//             actual,
//             parsed: Delimeter::Chapter,
//         })
//     }
//
//     pub fn range_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
//         delim_range().to_span().map(|actual| Self {
//             actual,
//             parsed: Delimeter::Range,
//         })
//     }
// }

// impl FullSpan for VerboseDelimeter {
//     fn full_span(&self) -> SimpleSpan {
//         self.actual.full_span()
//     }
// }

#[derive(Clone, Debug, FromTuple)]
pub struct DelimitedNumber {
    pub delimeter: VerboseDelimeter,
    pub padded_number: FrontPadded<VerboseNumber>,
}

impl DelimitedNumber {
    pub fn by_chapter<'a>() -> impl Parser<'a, &'a str, Self> {
        VerboseDelimeter::chapter_delimeter()
            .then(FrontPadded::parser(VerboseNumber::parser()))
            .map(FromTuple::from_tuple)
    }

    pub fn by_range<'a>() -> impl Parser<'a, &'a str, Self> {
        VerboseDelimeter::range_delimeter()
            .then(FrontPadded::parser(VerboseNumber::parser()))
            .map(FromTuple::from_tuple)
    }
}

impl FullSpan for DelimitedNumber {
    fn full_span(&self) -> SimpleSpan {
        // let start = self.delimeter.full_span_start();
        let start = self.padded_number.full_span_start() - 1;
        let end = self.padded_number.full_span_end();
        SimpleSpan::from(start..end)
    }
}

/// Each atomic unit should be front-padded
#[derive(Clone, Debug, FromTuple)]
pub struct FrontPadded<T> {
    // TODO: this should [`Option<T>`] to more clearly indicate if there is space or not
    // but if I do that, then I break [`FullSpan`] on [`VerboseSpace<'a>`]
    pub space: Option<VerboseSpace>,
    pub value: T,
}

impl<T> FrontPadded<T> {
    pub fn parser<'a>(child: impl Parser<'a, &'a str, T>) -> impl Parser<'a, &'a str, Self> {
        VerboseSpace::optional_parser()
            .then(child)
            .map(FromTuple::from_tuple)
    }
}

impl<T: FullSpan> FullSpan for FrontPadded<T> {
    fn full_span(&self) -> SimpleSpan {
        if let Some(ref space) = self.space {
            let start = space.full_span_start();
            let end = self.value.full_span_end();
            SimpleSpan::from(start..end)
        } else {
            self.value.full_span()
        }
    }
}

/**
- The reason leading whitespace is included is that this is to be used on the segments that come *right after* a matched book name
- This is a full segment, which all segments but potentially the last one are (it may be incomplete, as the user is still typing it)
*/
// PERF: If I just store spans (and not any str's), I can probably massively cut the size of this
// struct.
// However it is fine for now as:
// 1. It is written in Rust
// 2. This is only used for auto-completing 1 verse
// TODO: You can use enums instead of nesting
#[derive(Clone, Debug, FromTuple)]
pub struct VerboseFullSegment {
    /// `\s*\d+(\s*:\d+)?(\s*-\d+(\s*:\d+)?)?`
    /// `\s*\d+`
    pub start: FrontPadded<VerboseNumber>,
    /// `\s*\d+(\s*:\d+)?(\s*-\d+(\s*:\d+)?)?`
    /// ----->`(\s*:\d+)?`
    pub explicit_start_verse: Option<FrontPadded<DelimitedNumber>>,
    /// `\s*\d+(\s*:\d+)?(\s*-\d+(\s*:\d+)?)?`
    /// --------------->`(\s*-\d+(\s*:\d+)?)?`
    pub end: Option<(
        FrontPadded<DelimitedNumber>,
        Option<FrontPadded<DelimitedNumber>>,
    )>,
    /// TODO: I don't know if I like this, because it should always be present, except for the last
    /// entry, (unless of course the last entry is necessarily delimeted by the segment delimeter
    /// and there is a separate "incomplete segment" that is always the last one)
    /// BUG: This can now parse `1:2-3:4 5:6-7:8` as valid
    // pub closing: Option<FrontPadded<'a, VerboseDelimeter>>,
    pub closing: FrontPadded<VerboseDelimeter>,
}

impl FullSpan for VerboseFullSegment {
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

impl VerboseFullSegment {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        // `\s*\d+`
        let start = VerboseSpace::optional_parser()
            .then(VerboseNumber::parser())
            .map(FromTuple::from_tuple);

        // `(\s*:\d+)?`
        let explicit_start_verse = VerboseSpace::optional_parser()
            .then(DelimitedNumber::by_chapter())
            // .from_tuple()
            .map(FromTuple::from_tuple)
            .or_not();

        // `(\s*-\d+(\s*:\d+)?)?`
        let end = VerboseSpace::optional_parser()
            .then(DelimitedNumber::by_range())
            .map(FromTuple::from_tuple)
            .then(
                VerboseSpace::optional_parser()
                    .then(DelimitedNumber::by_chapter())
                    .map(FromTuple::from_tuple)
                    .or_not(),
            )
            .or_not();

        let closing = VerboseSpace::optional_parser()
            .then(VerboseDelimeter::segment_delimeter())
            .map(FromTuple::from_tuple);
        // .or_not();

        start
            .then(explicit_start_verse)
            .then(end)
            .then(closing)
            .map(FromTuple::from_tuple)
    }
}

#[derive(Clone, Debug)]
pub struct VerboseSegments {
    pub segments: Vec<VerboseFullSegment>,
}

impl VerboseSegments {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        VerboseFullSegment::parser()
            .repeated()
            .at_least(1)
            .collect()
            .lazy()
            .map(|segments| Self { segments })
    }

    pub fn parse(input: &str) -> Option<Self> {
        Self::parser().parse(input).into_output()
    }
}

impl FullSpan for VerboseSegments {
    fn full_span(&self) -> SimpleSpan {
        let start = self
            .segments
            .first()
            .expect("There is always at least 1 segment parsed")
            .full_span_start();
        let end = self
            .segments
            .last()
            .expect("There is always at least 1 segment parsed")
            .full_span_end();
        SimpleSpan::from(start..end)
    }
}
