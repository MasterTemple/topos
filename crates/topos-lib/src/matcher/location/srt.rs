use chumsky::{
    prelude::*,
    text::{Char, digits, inline_whitespace, newline},
};
use from_nested_tuple::FromTuple;

// TODO: Make this more general
#[derive(FromTuple)]
pub struct SRTLocation {
    start: f32,
    end: f32,
}

#[derive(FromTuple)]
pub struct SRTDocument<'a> {
    segments: Vec<SRTSegment<'a>>,
}

#[derive(FromTuple)]
pub struct SRTSegment<'a> {
    id: u32,
    start: SRTTimeStamp,
    end: SRTTimeStamp,
    text: &'a str,
}

#[derive(FromTuple)]
pub struct SRTTimeStamp {
    h: u32,
    m: u32,
    s: u32,
}

fn num<'a>() -> impl Parser<'a, &'a str, u32> {
    digits(10)
        .to_slice()
        .from_str()
        .try_map(|v, span| v.map_err(|_| EmptyErr::default()))
}

fn colon<'a>() -> impl Parser<'a, &'a str, char> {
    just(':')
}
fn comma<'a>() -> impl Parser<'a, &'a str, char> {
    just(',')
}

fn arrow<'a>() -> impl Parser<'a, &'a str, &'a str> {
    just("-->").padded_by(inline_whitespace())
}

impl SRTTimeStamp {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Self> {
        num()
            .then_ignore(colon())
            .then(num())
            .then_ignore(comma())
            .then(num())
            .map(FromTuple::from_tuple)
    }
}

impl<'a> SRTSegment<'a> {
    pub fn parser() -> impl Parser<'a, &'a str, Self> {
        num()
            .then_ignore(newline())
            .then(SRTTimeStamp::parser())
            .then_ignore(arrow())
            .then(SRTTimeStamp::parser())
            .then_ignore(newline())
            .then(
                any()
                    .filter(|c: &char| !c.is_newline())
                    .repeated()
                    .at_least(1)
                    .to_slice(),
            )
            .then_ignore(newline())
            .map(FromTuple::from_tuple)
    }
}

// TODO: Test this and make more fault tolerant
impl<'a> SRTDocument<'a> {
    pub fn parser() -> impl Parser<'a, &'a str, Self> {
        SRTSegment::parser()
            .padded()
            .repeated()
            .collect()
            .map(FromTuple::from_tuple)
    }
}
