use chumsky::prelude::*;
use chumsky::span::SimpleSpan;

use crate::components::Delimeter;

#[derive(Clone, Debug)]
pub struct Spanned<T> {
    value: T,
    span: SimpleSpan,
}

#[derive(Clone, Debug)]
pub struct VerboseNumber<'a> {
    actual: Spanned<&'a str>,
    parsed: u8,
    subverse: Option<Spanned<&'a str>>,
}

#[derive(Clone, Debug)]
pub struct VerboseSpace<'a> {
    actual: Spanned<&'a str>,
}

#[derive(Clone, Debug)]
pub struct VerboseDelimeter<'a> {
    actual: Spanned<&'a str>,
    // position really determines this
    parsed: Delimeter,
}

#[derive(Clone, Debug)]
pub struct DelimitedNumber<'a> {
    delimeter: VerboseDelimeter<'a>,
    number: VerboseNumber<'a>,
}

pub type FrontPaddedDelimetedNumber<'a> = FrontPadded<'a, DelimitedNumber<'a>>;

#[derive(Clone, Debug)]
pub struct FrontPadded<'a, T> {
    space: VerboseSpace<'a>,
    value: T,
}

/**
The reason leading whitespace is included is that this is to be used on the segments that come *right after* a matched book name
*/
#[derive(Clone, Debug)]
pub struct VerboseSegment<'a> {
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
