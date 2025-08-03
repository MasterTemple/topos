use crate::components::*;
use crate::roman_numerals::only_roman_numerals;
use chumsky::prelude::*;
use chumsky::text::whitespace;

/// This is 1/4th the size of the others, for searching I should parse this, but for auto-complete
/// I should use the others
/// `\d+(:\d+)?(-\d+(:\d+)?)?`
#[derive(Clone, Debug)]
pub struct MinimalSegment {
    /// `\d+(:\d+)?(-\d+(:\d+)?)?`
    /// `\d+`
    pub start: u8,
    /// `\d+(:\d+)?(-\d+(:\d+)?)?`
    /// -->`(:\d+)?`
    pub explicit_start_verse: Option<u8>,
    /// `\d+(:\d+)?(-\d+(:\d+)?)?`
    /// --------->`(-\d+(:\d+)?)?`
    pub end: Option<(u8, Option<u8>)>,
}

/// Call [`MinimalSegments::parse`], which will match all segments (at least 1) and return the span
#[derive(Clone, Debug)]
pub struct MinimalSegments {
    pub segments: Vec<MinimalSegment>,
    pub span: SimpleSpan,
}

impl MinimalSegments {
    pub fn parse(input: &str) -> Option<Self> {
        // PERF: Should I make this into a static value?
        minimal_full_segments_parser().parse(input).into_output()
    }
}

/// Just parse or fail, no context
fn only_decimals<'a>() -> impl Parser<'a, &'a str, u8> {
    whitespace()
        .ignore_then(decimal())
        .then_ignore(whitespace())
}

fn only_numbers<'a>() -> impl Parser<'a, &'a str, u8> {
    only_decimals()
        .or(only_roman_numerals())
        .then_ignore(optional_subverse())
}

fn minimal_full_segment_parser<'a>() -> impl Parser<'a, &'a str, MinimalSegment> {
    only_numbers()
        .then(delim_chapter().ignore_then(only_numbers()).or_not())
        .then(
            delim_range()
                .ignore_then(only_numbers())
                .then(delim_chapter().ignore_then(only_numbers()).or_not())
                .or_not(),
        )
        .map(|((start, explicit_start_verse), end)| MinimalSegment {
            start,
            explicit_start_verse,
            end,
        })
}

fn minimal_full_segments_parser<'a>() -> impl Parser<'a, &'a str, MinimalSegments> {
    minimal_full_segment_parser()
        .separated_by(delim_segment())
        .at_least(1)
        .allow_trailing()
        .collect::<Vec<_>>()
        .map_with(|segments, e| MinimalSegments {
            segments,
            span: e.span(),
        })
        .lazy()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal() {
        let p = |input: &str| minimal_full_segment_parser().parse(input).into_result();
        assert!(p("1").is_ok());
        assert!(p("1:1").is_ok());
        assert!(p("1-2:1").is_ok());
        assert!(p("1:1-2").is_ok());
        assert!(p("1:1-2:3").is_ok());
        assert!(p(" 1 : 1- 2:   3  ").is_ok());
    }

    #[test]
    fn test_minimal_list() {
        let p = |input: &str, len: usize| {
            minimal_full_segments_parser()
                .parse(input)
                .into_result()
                .is_ok_and(|v| v.segments.len() == len)
        };
        assert!(p(" 1 : 1- 2:   3  , 4", 2));
        assert!(p(" 1 : 1- 2:   3  , 4:5", 2));
        assert!(p(" 1 : 1- 2:   3  , 4:5-7", 2));
        assert!(p(" 1 : 1a- 2:   3  , 4:5-7", 2));
    }

    #[test]
    fn test_minimal_span() {
        let p = |input: &str| minimal_full_segments_parser().parse(input).into_result();
        _ = dbg!(p(" 1 : 1- 2:   3  , 4"));
        _ = dbg!(p(" 1 : 1- 2:   3  , 4:5"));
        _ = dbg!(p(" 1 : 1- 2:   3  , 4:5-7"));
        _ = dbg!(p(" 1 : 1a- 2:   3  , 4:5-7"));
        _ = dbg!(p(" 1 : 1a- 2:   3  , 4:5-7 this ends here"));
        // err
        _ = dbg!(p("hi ok but yes 1 : 1a- 2:   3  , 4:5-7 this ends here"));
    }
}
