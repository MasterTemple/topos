use derive_more::{Deref, DerefMut, IntoIterator};
use serde::{Deserialize, Serialize};

use crate::{
    data::books::BookId,
    segments::{segment::Segment, verse_bounds::VerseBounds},
};

#[derive(Clone, Debug)]
pub struct Passage {
    pub book: BookId,
    pub segments: Segments,
}

impl Passage {
    pub fn overlaps_with(&self, other: &Passage) -> bool {
        if self.book != other.book {
            return false;
        }
        self.segments.contains_overlap(&other.segments)
    }

    /// TODO: check if passage entirely contains the other passage
    pub fn contains(&self, other: &Passage) -> bool {
        todo!()
    }
}

#[derive(Clone, Debug, Deref, DerefMut, Serialize, Deserialize, IntoIterator)]
pub struct Segments(pub Vec<Segment>);

impl Segments {
    pub fn new() -> Self {
        Self(vec![])
    }

    // pub fn overlaps_segment(&self, other: impl Into<Segment>) -> bool {
    pub fn overlaps_with(&self, other: &impl VerseBounds) -> bool {
        self.iter().any(|this| this.overlaps_with(other))
    }

    /// - This can be better optimized, but that is not a priority right now
    /// - I just need some way to order the segments and do it in linear time
    pub fn contains_overlap(&self, other: &Segments) -> bool {
        self.iter().any(|this| other.overlaps_with(this))
    }

    pub fn with_book(self, book_id: BookId) -> Passage {
        Passage {
            book: book_id,
            segments: self,
        }
    }
}

impl Segments {
    pub fn format(&self, verse_seperator: &str, chapter_seperator: &str) -> String {
        let mut prev_chapter = None;
        let mut output = String::new();
        for seg in self.iter() {
            let current_chapter = seg.starting_chapter();
            if let Some(chapter) = prev_chapter {
                if chapter == current_chapter {
                    output.push_str(verse_seperator);
                } else {
                    output.push_str(chapter_seperator);
                }
            }
            prev_chapter = Some(current_chapter);
            output.push_str(&seg.to_string());
        }
        output
    }
}

impl std::fmt::Display for Segments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format(",", "; "))
    }
}
