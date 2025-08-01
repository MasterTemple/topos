use crate::{data::books::BookId, segments::segment::Segment};

pub struct CompletionOutput {
    pub book: BookId,
    pub segments: Segments,
    pub suggestions: Vec<Segment>,
}

impl CompletionOutput {
    pub fn new(book: BookId, segments: Segments, suggestions: Vec<Segments>) -> Self {
        Self {
            book,
            segments,
            suggestions,
        }
    }
}
