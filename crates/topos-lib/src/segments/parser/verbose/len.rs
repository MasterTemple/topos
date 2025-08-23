use chumsky::{prelude::*, text::whitespace};

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
