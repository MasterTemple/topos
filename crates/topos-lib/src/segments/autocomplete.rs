use crate::{
    data::books::BookId,
    segments::{segment::Segment, segments::Segments},
};

pub struct SegmentAutoCompleter {
    // book_chapters_verses:
}

pub enum CompletionKind {
    Chapter,
}

impl SegmentAutoCompleter {
    pub fn suggest(&self, book: &BookId, segments: &Segments) -> Option<Segment> {
        //
        todo!()
    }
}
