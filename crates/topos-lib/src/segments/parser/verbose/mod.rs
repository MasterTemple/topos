use chumsky::{prelude::*, text::whitespace};
use from_nested_tuple::FromTuple;

use crate::segments::parser::{
    components::{Decimal, Delimeter, Subverse},
    roman_numeral::RomanNumerals,
    verbose::{
        components::{DelimitedNumber, FrontPadded, VerboseDelimeter, VerboseNumber, VerboseSpace},
        len::SpanLen,
    },
};

pub mod components;
pub mod len;

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
            .from_tuple();

        // `(\s*:\d+)?`
        let explicit_start_verse = VerboseSpace::optional_parser()
            .then(DelimitedNumber::by_chapter())
            // .from_tuple()
            .from_tuple()
            .or_not();

        // `(\s*-\d+(\s*:\d+)?)?`
        let end = VerboseSpace::optional_parser()
            .then(DelimitedNumber::by_range())
            .from_tuple()
            .then(
                VerboseSpace::optional_parser()
                    .then(DelimitedNumber::by_chapter())
                    .from_tuple()
                    .or_not(),
            )
            .or_not();

        let closing = VerboseSpace::optional_parser()
            .then(VerboseDelimeter::segment_delimeter())
            .from_tuple();
        // .or_not();

        start
            .then(explicit_start_verse)
            .then(end)
            .then(closing)
            .from_tuple()
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
