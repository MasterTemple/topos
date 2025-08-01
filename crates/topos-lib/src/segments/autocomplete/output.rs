use crate::{
    data::books::BookId,
    segments::{segment::Segment, segments::Segments},
};

pub struct CompletionOutput {
    pub book: BookId,
    pub segments: Segments,
    pub suggestions: Vec<Segment>,
}

impl CompletionOutput {
    pub fn new(book: BookId, segments: Segments, suggestions: Vec<Segment>) -> Self {
        Self {
            book,
            segments,
            suggestions,
        }
    }
}
