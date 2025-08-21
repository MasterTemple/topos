use chumsky::{prelude::*, text::whitespace};
use from_nested_tuple::FromTuple;

use crate::{
    components::{Delimeter, SUBVERSE, delim_chapter, delim_range, delim_segment},
    roman_numerals::{ROMAN_NUMERALS, parse_roman_numeral},
};

pub trait SpanLen {
    fn span_len(&self) -> usize;
    fn as_span(&self, start: usize) -> SimpleSpan {
        let end = start + self.span_len();
        SimpleSpan::from(start..end)
    }
}

impl<T: SpanLen> SpanLen for Option<T> {
    fn span_len(&self) -> usize {
        self.as_ref().map(|s| s.span_len()).unwrap_or(0)
    }
}

#[derive(Clone, Debug)]
pub struct Lengthed<T = (), Len = u8> {
    pub value: T,
    pub len: Len,
}

impl Lengthed {
    pub fn from_span(span: SimpleSpan) -> Self {
        let len = (span.end - span.start) as u8;
        Self { value: (), len }
    }
}

impl<T, Len> Lengthed<T, Len> {
    pub fn new(value: T, len: Len) -> Self {
        Self { value, len }
    }
}

impl<T> Lengthed<T, usize> {
    // TODO: make this parse into a u8 or be marked as too long
    pub fn long_parser<'a>(child: impl Parser<'a, &'a str, T>) -> impl Parser<'a, &'a str, Self> {
        child.map_with(|value, e| {
            let span = e.span();
            Self::new(value, span.end - span.start)
        })
    }
}

impl<T> Lengthed<T> {
    pub fn parser<'a>(child: impl Parser<'a, &'a str, T>) -> impl Parser<'a, &'a str, Self> {
        child.try_map(|value, span| {
            let diff = span.end - span.start;
            let diff = u8::try_from(diff).map_err(|_| EmptyErr::default())?;
            Ok(Self::new(value, diff))
        })
    }
}

impl<T> SpanLen for Lengthed<T> {
    fn span_len(&self) -> usize {
        self.len as usize
    }
}

impl<T> SpanLen for Lengthed<T, usize> {
    fn span_len(&self) -> usize {
        self.len
    }
}

#[derive(Clone, Debug)]
pub struct VerboseDecimal {
    pub parsed: Lengthed<u8>,
}

impl VerboseDecimal {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        Lengthed::parser(
            any()
                .filter(|c: &char| c.is_numeric())
                .repeated()
                .at_least(1)
                .at_most(3)
                .to_slice()
                .try_map(|slice: &str, _| slice.parse().map_err(|_| EmptyErr::default())),
        )
        .map(|parsed| Self { parsed })
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
        Lengthed::parser(
            any()
                .filter(|c: &char| ROMAN_NUMERALS.contains(c))
                .repeated()
                .at_least(1)
                .at_most(9) // Just to keep the parser from getting trolled
                .to_slice()
                .try_map(|slice, _| {
                    // BUG: This should fail on bad parsing
                    Ok(parse_roman_numeral(slice)) //.map_err(|_| EmptyErr::default())?;
                }),
        )
        .map(|parsed| Self { parsed })
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
            .then(one_of(SUBVERSE).or_not())
            .map(FromTuple::from_tuple)
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
            .map(FromTuple::from_tuple)
    }

    pub fn by_range<'a>() -> impl Parser<'a, &'a str, Self> {
        VerboseDelimeter::range_delimeter()
            .then(FrontPadded::parser(VerboseNumber::parser()))
            .map(FromTuple::from_tuple)
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

impl<T> FrontPadded<T> {
    pub fn parser<'a>(child: impl Parser<'a, &'a str, T>) -> impl Parser<'a, &'a str, Self> {
        VerboseSpace::optional_parser()
            .then(child)
            .map(FromTuple::from_tuple)
    }
}

impl<T: SpanLen> SpanLen for FrontPadded<T> {
    fn span_len(&self) -> usize {
        let space_len = self.space.span_len();
        let value_len = self.value.span_len();
        space_len + value_len
    }
}

/**
- The reason leading whitespace is included is that this is to be used on the segments that come *right after* a matched book name
- This is a full segment, which all segments but potentially the last one are (it may be incomplete, as the user is still typing it)
*/
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

impl SpanLen for VerboseFullSegment {
    fn span_len(&self) -> usize {
        let start = self.start.span_len();
        let explicit_start_verse = self.explicit_start_verse.span_len();
        let end = if let Some(end) = &self.end {
            end.0.span_len() + end.1.span_len()
        } else {
            0
        };
        let closing = self.closing.span_len();

        start + explicit_start_verse + end + closing
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

impl SpanLen for VerboseSegments {
    fn span_len(&self) -> usize {
        self.segments.iter().map(|s| s.span_len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::Parser;

    #[test]
    fn test_full_segment<'a>() {
        let p = |input: &'a str| VerboseFullSegment::parser().parse(input).into_result();
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

pub enum SpaceOptions {
    DontTouch,
    RemoveAll,
    Normalize,
}

pub enum RomanNumeralOptions {
    DontTouch,
    MakeUppercase,
    MakeLowercase,
    MakeAllDecimal,
    MakeChaptersDecimal,
    MakeVersesDecimal,
}

pub enum DelimeterOptions {
    DontTouch,
    Normalize,
    NormalizeWith {
        chapter: Option<String>,
        range: Option<String>,
        chapter_segment: Option<String>,
        verse_segment: Option<String>,
    },
}

pub struct RangeOptions {
    /// `1-2:1` instead of `1:1-2:1`
    pub exclude_verse_1_for_chapter_range: bool,
    /// `1:1-2` instead of `1:1,2`
    pub join_adjacent_verses: bool,
    /// `Jude 1:1` instead of `Jude 1`
    pub use_chapter_in_single_chapter_books: bool,
}

pub struct FormatOptions {
    /// have spacing for numbers and each type of delimeter that overwrite this
    pub general_spacing: SpaceOptions,
    pub include_subverse: bool,
    pub roman: RomanNumeralOptions,
    pub delim: DelimeterOptions,
    pub range: RangeOptions,
}

pub enum VerseFormatContext {
    None,
    PrevChapter {
        previous_chapter: u8,
    },
    PrevChapterVerse {
        previous_chapter: u8,
        previous_verse: u8,
    },
}

pub struct FullFormatContext {
    // TODO: this really should be a parsed segment.. this leads me to realize that the formatter
    // should be a part of `topos-lib`
    pub segment: VerboseFullSegment,
    pub start: usize,
    pub is_after_range: bool,
    pub verse: VerseFormatContext,
}

pub enum FormattableToken {
    Delimeter(VerboseDelimeter),
    Number(VerboseNumber),
    Space(VerboseSpace),
}

impl SpanLen for FormattableToken {
    fn span_len(&self) -> usize {
        match self {
            FormattableToken::Delimeter(t) => t.span_len(),
            FormattableToken::Number(t) => t.span_len(),
            FormattableToken::Space(t) => t.span_len(),
        }
    }
}

impl FormattableToken {
    pub fn get_contents<'a>(&'_ self, s: &'a str, start: usize) -> &'a str {
        let span = self.as_span(start);
        &s[span.start..span.end]
    }
}

impl FormattableToken {
    pub fn format(
        self,
        input: &str,
        cx: &mut FullFormatContext,
        options: &FormatOptions,
    ) -> String {
        let actual = self.get_contents(input, cx.start);
        cx.start += self.span_len();
        match self {
            FormattableToken::Delimeter(token) => {
                match token.parsed {
                    Delimeter::Segment => {
                        cx.is_after_range = false;
                        match &options.delim {
                            DelimeterOptions::DontTouch => token.actual.to_string(),
                            // BUG: I need to know upcoming / current segment type, so I can
                            // specify ',' or ';'
                            DelimeterOptions::Normalize => ",".to_string(),
                            DelimeterOptions::NormalizeWith { verse_segment, .. } => {
                                verse_segment.clone().unwrap_or(String::from(","))
                            }
                        }
                    }
                    Delimeter::Chapter => match &options.delim {
                        DelimeterOptions::DontTouch => token.actual.to_string(),
                        DelimeterOptions::Normalize => todo!(),
                        DelimeterOptions::NormalizeWith { chapter, range, .. } => todo!(),
                    },
                    Delimeter::Range => {
                        cx.is_after_range = true;
                        todo!()
                    }
                }
            }
            FormattableToken::Number(token) => todo!(),
            FormattableToken::Space(token) => match options.general_spacing {
                SpaceOptions::DontTouch => actual.to_string(),
                SpaceOptions::RemoveAll => String::new(),
                SpaceOptions::Normalize => String::from(" "),
            },
        }
    }
}
