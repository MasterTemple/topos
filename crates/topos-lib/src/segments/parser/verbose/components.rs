use chumsky::{prelude::*, text::whitespace};
use from_nested_tuple::FromTuple;

use crate::segments::parser::{
    components::{Decimal, Delimeter, Subverse},
    roman_numeral::RomanNumerals,
    verbose::len::{Lengthed, SpanLen},
};

#[derive(Clone, Debug)]
pub struct VerboseDecimal {
    pub parsed: Lengthed<u8>,
}

impl VerboseDecimal {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        Lengthed::parser(Decimal::parser()).map(|parsed| Self { parsed })
    }
}

impl SpanLen for VerboseDecimal {
    fn span_len(&self) -> usize {
        self.parsed.span_len()
    }
}

#[derive(Clone, Debug)]
pub struct VerboseRomanNumeral {
    // pub span: SimpleSpan,
    pub parsed: Lengthed<u8>,
}

impl VerboseRomanNumeral {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        Lengthed::parser(RomanNumerals::parser()).map(|parsed| Self { parsed })
    }
}

impl SpanLen for VerboseRomanNumeral {
    fn span_len(&self) -> usize {
        self.parsed.span_len()
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

    pub fn parsed(&self) -> u8 {
        match self {
            VerboseNumberKind::Decimal(v) => v.parsed.value,
            VerboseNumberKind::Roman(v) => v.parsed.value,
        }
    }
}

impl SpanLen for VerboseNumberKind {
    fn span_len(&self) -> usize {
        match self {
            VerboseNumberKind::Decimal(num) => num.span_len(),
            VerboseNumberKind::Roman(num) => num.span_len(),
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
            .then(Subverse::optional_parser())
            .from_tuple()
    }

    pub fn parsed(&self) -> u8 {
        self.number.parsed()
    }
}

impl SpanLen for VerboseNumber {
    fn span_len(&self) -> usize {
        let optional_character_offset = match self.subverse {
            Some(_) => 1,
            None => 0,
        };
        self.number.span_len() + optional_character_offset
    }
}

// TODO: I technically don't need to store the span of spaces, since I can calculate them between non-space characters
#[derive(Clone, Debug)]
pub struct VerboseSpace {
    // pub span: SimpleSpan,
    pub len: Lengthed,
}

impl VerboseSpace {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        whitespace().to_span().map(|span| Self {
            len: Lengthed::from_span(span),
        })
    }

    pub fn optional_parser<'a>() -> impl Parser<'a, &'a str, Option<Self>> {
        whitespace().at_least(1).to_span().or_not().map(|span| {
            span.map(|span| Self {
                len: Lengthed::from_span(span),
            })
        })
    }
}

impl SpanLen for VerboseSpace {
    fn span_len(&self) -> usize {
        self.len.span_len()
    }
}

#[derive(Clone, Debug)]
pub struct VerboseDelimeter {
    pub actual: char,
    pub parsed: Delimeter,
}

impl VerboseDelimeter {
    pub fn segment_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
        Delimeter::segment_parser().map(|actual| Self {
            actual,
            parsed: Delimeter::Segment,
        })
    }

    pub fn chapter_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
        Delimeter::chapter_parser().map(|actual| Self {
            actual,
            parsed: Delimeter::Chapter,
        })
    }

    pub fn range_delimeter<'a>() -> impl Parser<'a, &'a str, Self> {
        Delimeter::range_parser().map(|actual| Self {
            actual,
            parsed: Delimeter::Range,
        })
    }
}

impl SpanLen for VerboseDelimeter {
    fn span_len(&self) -> usize {
        1
    }
}

#[derive(Clone, Debug, FromTuple)]
pub struct DelimitedNumber {
    pub delimeter: VerboseDelimeter,
    pub padded_number: FrontPadded<VerboseNumber>,
}

impl DelimitedNumber {
    pub fn by_chapter<'a>() -> impl Parser<'a, &'a str, Self> {
        VerboseDelimeter::chapter_delimeter()
            .then(FrontPadded::parser(VerboseNumber::parser()))
            .from_tuple()
    }

    pub fn by_range<'a>() -> impl Parser<'a, &'a str, Self> {
        VerboseDelimeter::range_delimeter()
            .then(FrontPadded::parser(VerboseNumber::parser()))
            .from_tuple()
    }

    pub fn parsed(&self) -> u8 {
        self.padded_number.value.parsed()
    }
}

impl SpanLen for DelimitedNumber {
    fn span_len(&self) -> usize {
        self.delimeter.span_len() + self.padded_number.span_len()
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

impl<T> std::ops::Deref for FrontPadded<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> FrontPadded<T> {
    pub fn parser<'a>(child: impl Parser<'a, &'a str, T>) -> impl Parser<'a, &'a str, Self> {
        VerboseSpace::optional_parser()
            .then(child)
            .from_tuple()
    }
}

impl<T: SpanLen> SpanLen for FrontPadded<T> {
    fn span_len(&self) -> usize {
        let space_len = self.space.span_len();
        let value_len = self.value.span_len();
        space_len + value_len
    }
}

impl FrontPadded<DelimitedNumber> {
    pub fn parsed_value(&self) -> u8 {
        self.value.parsed()
    }
}

impl FrontPadded<VerboseNumber> {
    pub fn parsed_value(&self) -> u8 {
        self.value.parsed()
    }
}
/*
let new = if let Some(start_verse) = seg
    .explicit_start_verse
    .as_ref()
    .map(FrontPadded::parsed_value)
{
    // let start_verse = value.parsed();
    let start_chapter = seg.start.parsed();
    if let Some(end) = seg.end {
        // `1:2-3:4`
        if let Some(end_verse) = end.1.as_ref().map(FrontPadded::parsed_value) {
            let end_chapter = end.0.parsed_value();
            Segment::chapter_range(start_chapter, start_verse, end_chapter, end_verse)
        }
        // `1:2-3`
        else {
            let end_verse = end.0.parsed_value();
            Segment::chapter_verse_range(start_chapter, start_verse, end_verse)
        }
    // `1:2`
    } else {
        Segment::chapter_verse(start_chapter, start_verse)
    }
} else {
*/
