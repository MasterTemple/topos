use chumsky::prelude::*;
use chumsky::span::SimpleSpan;
use chumsky::text::whitespace;
use from_nested_tuple::FromTuple;

use crate::components::{SUBVERSE, delim_chapter, delim_range, delim_segment};
use crate::{
    components::Delimeter,
    roman_numerals::{ROMAN_NUMERALS, parse_roman_numeral},
};

// pub trait ChumskyExt<'src, I, T, E>: Parser<'src, I, T, E> + Sized
// where
//     I: Input<'src, Span = SimpleSpan>,
//     E: extra::ParserExtra<'src, I>,
// {
//     fn spanned(self) -> impl Parser<'src, I, Spanned<T>, E> {
//         self.map_with(|value, e| Spanned::new(value, e.span()))
//     }
// }

// pub trait ChumskyExt<'a, T>: Parser<'a, &'a str, T> + Sized {
//     fn spanned(self) -> impl Parser<'a, &'a str, Spanned<T>> {
//         self.map_with(|value, e| Spanned::new(value, e.span()))
//     }
// }
//
// // impl<'src, I, T, E> ChumskyExt<'src, I, T, E> for T
// // where
// //     I: Input<'src, Span = SimpleSpan>,
// //     E: extra::ParserExtra<'src, I>,
// //     T: Parser<'src, I, T, E>,
// // {
// // }
//
// impl<'a, T> ChumskyExt<'a, T> for T
// where
//     // I: Input<'src, Token = u8>,
//     // E: extra::ParserExtra<'src, &'src str>,
//     T: Parser<'a, &'a str, T>,
// {
//     // fn spanned(self) -> impl Parser<'src, &'src str, Spanned<T>> {
//     //     Spanned::parser(self)
//     // }
// }
//
// pub trait FromTupleChumskyExt<'src, I, T, E>: Parser<'src, I, T, E> + Sized
// where
//     I: Input<'src, Span = SimpleSpan>,
//     E: extra::ParserExtra<'src, I>,
// {
//     fn from_tuple<O>(self) -> impl Parser<'src, I, O, E>
//     where
//         O: FromTuple<Tuple = T>,
//         Self: Sized,
//     {
//         self.map(FromTuple::from_tuple)
//     }
// }
//
// // impl<'src, I, T, E> ChumskyExt<'src, I, T, E> for T
// // where
// //     I: Input<'src, Token = u8>,
// //     E: extra::ParserExtra<'src, I>,
// //     T: Parser<'src, I, T, E>,
// // {
// //     fn spanned(self) -> impl Parser<'src, I, Spanned<T>, E> {
// //         Spanned::parser(self)
// //     }
// // }
//
// impl<'src, I, T, E> FromTupleChumskyExt<'src, I, T, E> for T
// where
//     I: Input<'src, Span = SimpleSpan>,
//     E: extra::ParserExtra<'src, I>,
//     T: Parser<'src, I, T, E>,
// {
// }
//
// // impl<'src, T> ChumskyExt<'src, &'src str, T, extra::Default> for T
// // where
// //     // I: Input<'src, Token = u8>,
// //     // E: extra::ParserExtra<'src, &'src str>,
// //     T: Parser<'src, &'src str, T>,
// // {
// //     // fn spanned(self) -> impl Parser<'src, &'src str, Spanned<T>> {
// //     //     Spanned::parser(self)
// //     // }
// // }
//
// // impl<'src, I, E, T> ChumskyExt for T
// // where
// //     I: Input<'src, Token = u8>,
// //     E: extra::ParserExtra<'src, I>,
// //     T: Parser<'src, I, E>,
// // {
// // }

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

#[derive(Clone, Debug, FromTuple)]
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
            .then(Spanned::parser(one_of(SUBVERSE)).or_not())
            .map(FromTuple::from_tuple)
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
        Spanned::parser(whitespace().to_slice()).map(|actual| Self { actual })
    }

    pub fn optional_parser() -> impl Parser<'a, &'a str, Option<Self>> {
        Spanned::parser(whitespace().at_least(1).to_slice())
            .or_not()
            .map(|actual| actual.map(|actual| Self { actual }))
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
        Spanned::parser(delim_segment()).map(|actual| Self {
            actual,
            parsed: Delimeter::Segment,
        })
    }

    pub fn chapter_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
        Spanned::parser(delim_chapter()).map(|actual| Self {
            actual,
            parsed: Delimeter::Chapter,
        })
    }

    pub fn range_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
        Spanned::parser(delim_range()).map(|actual| Self {
            actual,
            parsed: Delimeter::Range,
        })
    }
}

#[derive(Clone, Debug, FromTuple)]
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

impl<'a> DelimitedNumber<'a> {
    pub fn by_chapter() -> impl Parser<'a, &'a str, Self> {
        VerboseDelimeter::chapter_delimeter()
            .then(FrontPadded::parser(VerboseNumber::parser()))
            .map(FromTuple::from_tuple)
    }

    pub fn by_range() -> impl Parser<'a, &'a str, Self> {
        VerboseDelimeter::range_delimeter()
            .then(FrontPadded::parser(VerboseNumber::parser()))
            .map(FromTuple::from_tuple)
    }
}

#[derive(Clone, Debug)]
pub enum IncompleteDelimitedNumber<'a> {
    Empty,
    Delimeter(VerboseDelimeter),
    Both {
        delimeter: VerboseDelimeter,
        padded_number: FrontPadded<'a, Option<VerboseNumber<'a>>>,
    },
}

impl<'a> IncompleteDelimitedNumber<'a> {
    fn parse_delimeter(
        delim: impl Parser<'a, &'a str, VerboseDelimeter>,
    ) -> impl Parser<'a, &'a str, Self> {
        delim
            .then(FrontPadded::parser(VerboseNumber::parser().or_not()).or_not())
            .or_not()
            .map(|opt| {
                if let Some((delimeter, padded_number)) = opt {
                    if let Some(padded_number) = padded_number {
                        Self::Both {
                            delimeter,
                            padded_number,
                        }
                    } else {
                        Self::Delimeter(delimeter)
                    }
                } else {
                    Self::Empty
                }
            })
    }

    pub fn by_chapter() -> impl Parser<'a, &'a str, Self> {
        Self::parse_delimeter(VerboseDelimeter::chapter_delimeter())
    }

    pub fn by_range() -> impl Parser<'a, &'a str, Self> {
        Self::parse_delimeter(VerboseDelimeter::range_delimeter())
    }
}

// pub type FrontPaddedDelimetedNumber<'a> = FrontPadded<'a, DelimitedNumber<'a>>;

/// Each atomic unit should be front-padded
#[derive(Clone, Debug, FromTuple)]
pub struct FrontPadded<'a, T> {
    // TODO: this should [`Option<T>`] to more clearly indicate if there is space or not
    // but if I do that, then I break [`FullSpan`] on [`VerboseSpace<'a>`]
    pub space: Option<VerboseSpace<'a>>,
    pub value: T,
}

impl<'a, T: FullSpan> FullSpan for FrontPadded<'a, T> {
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

impl<'a, T> FrontPadded<'a, T> {
    pub fn parser(child: impl Parser<'a, &'a str, T>) -> impl Parser<'a, &'a str, Self> {
        VerboseSpace::optional_parser()
            .then(child)
            .map(FromTuple::from_tuple)
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
pub struct VerboseFullSegment<'a> {
    /// `\s*\d+(\s*:\d+)?(\s*-\d+(\s*:\d+)?)?`
    /// `\s*\d+`
    pub start: FrontPadded<'a, VerboseNumber<'a>>,
    /// `\s*\d+(\s*:\d+)?(\s*-\d+(\s*:\d+)?)?`
    /// ----->`(\s*:\d+)?`
    pub explicit_start_verse: Option<FrontPadded<'a, DelimitedNumber<'a>>>,
    /// `\s*\d+(\s*:\d+)?(\s*-\d+(\s*:\d+)?)?`
    /// --------------->`(\s*-\d+(\s*:\d+)?)?`
    pub end: Option<(
        FrontPadded<'a, DelimitedNumber<'a>>,
        Option<FrontPadded<'a, DelimitedNumber<'a>>>,
    )>,
    /// TODO: I don't know if I like this, because it should always be present, except for the last
    /// entry, (unless of course the last entry is necessarily delimeted by the segment delimeter
    /// and there is a separate "incomplete segment" that is always the last one)
    /// BUG: This can now parse `1:2-3:4 5:6-7:8` as valid
    // pub closing: Option<FrontPadded<'a, VerboseDelimeter>>,
    pub closing: FrontPadded<'a, VerboseDelimeter>,
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

impl<'a> VerboseFullSegment<'a> {
    pub fn parser() -> impl Parser<'a, &'a str, Self> {
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

/**
```bash
John
John 1
John 1:
John 1:1
John 1:1-
John 1:1-2
John 1:1-2:2

John 1
John 1-2
John 1-2:
John 1-2:2
```
*/
#[derive(Clone, Debug)]
pub enum VerboseIncompleteSegment<'a> {
    // `John `
    // `John 1`
    Start(Option<FrontPadded<'a, Option<VerboseNumber<'a>>>>),
    // `John 1:`
    // `John 1:1`
    StartVerse {
        start_chapter: FrontPadded<'a, VerboseNumber<'a>>,
        start_verse: FrontPadded<'a, IncompleteDelimitedNumber<'a>>,
    },
    // `John 1:1-`
    // `John 1:1-2`
    // `John 1-2`
    End {
        start_chapter: FrontPadded<'a, VerboseNumber<'a>>,
        start_verse: Option<FrontPadded<'a, DelimitedNumber<'a>>>,
        // TODO: I need a different type that can have incomplete delimeter and number
        end: FrontPadded<'a, IncompleteDelimitedNumber<'a>>,
    },
    // `John 1-2:`
    // `John 1:1-2:2`
    // `John 1-2:2`
    EndVerse {},
}

#[derive(Clone, Debug)]
pub struct VerboseSegments<'a> {
    pub segments: Vec<VerboseFullSegment<'a>>,
    // pub incomplete: VerboseIncompleteSegment<'a>,
    pub span: SimpleSpan,
}

impl<'a> VerboseSegments<'a> {
    pub fn parser() -> impl Parser<'a, &'a str, Self> {
        VerboseFullSegment::parser()
            .repeated()
            .at_least(1)
            .collect()
            .lazy()
            .map_with(|segments, e| {
                let span = e.span();
                Self { segments, span }
            })
    }

    pub fn parse(input: &'a str) -> Option<Self> {
        Self::parser().parse(input).into_output()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::Parser;

    use crate::verbose::VerboseFullSegment;

    #[test]
    fn test_full_segment<'a>() {
        let p = |input: &'a str| {
            VerboseFullSegment::<'a>::parser()
                .parse(input)
                .into_result()
        };
        assert!(p("1,").is_ok());
        assert!(p("1:1,").is_ok());
        assert!(p("1-2:1,").is_ok());
        assert!(p("1:1-2,").is_ok());
        assert!(p("1:1-2:3,").is_ok());
        assert!(p("1:1-2:3,").is_ok());
        assert!(p("1:1-2:3,").is_ok());
        assert!(p("1:1-2:   3,").is_ok());
        assert!(p("1:1- 2:   3,").is_ok());
        assert!(p("1: 1- 2:   3,").is_ok());
        assert!(p("1 : 1- 2:   3,").is_ok());
        assert!(p(" 1 : 1- 2:   3,").is_ok());
        assert!(p(" 1 : 1 - 2:   3,").is_ok());
    }

    #[test]
    fn test_minimal_list<'a>() {
        let p = |input: &'a str, len: usize| {
            VerboseSegments::parser()
                .parse(input)
                .into_result()
                .is_ok_and(|v| v.segments.len() == len)
        };
        assert!(p(" 1 : 1- 2:   3  ,", 1));
        assert!(p(" 1 : 1- 2:   3  , 4,", 2));
        assert!(p(" 1 : 1- 2:   3  , 4:5,", 2));
        assert!(p(" 1 : 1- 2:   3  , 4:5-7,", 2));
        assert!(p(" 1 : 1a- 2:   3  , 4:5-7,", 2));
        // BUG: This should not be
        // assert!(p(" 1 : 1a- 2:   3   4:5-7", 2));
    }
}
