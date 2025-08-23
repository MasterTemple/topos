use derive_more::{Deref, DerefMut, IntoIterator};
use serde::{Deserialize, Serialize};
use topos_parser::minimal::MinimalSegments;

use crate::{
    data::books::BookId,
    segments::{
        segment::{ChapterlessFormat, Segment},
        verse_bounds::VerseBounds,
    },
};

#[derive(Clone, Debug, Serialize, Deserialize)]
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

/// TODO: I need Segments and PartialSegments/Incomplete segments to be unified under a large
/// Segment type that I can use for auto-completions
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

    pub fn with_suggestion(&self, segment: Segment) -> Self {
        let mut new = self.clone();
        new.push(segment);
        new
    }
}

impl Segments {
    pub fn format(&self, verse_seperator: &str, chapter_seperator: &str) -> String {
        let mut prev_chapter = None;
        let mut output = String::new();
        for seg in self.iter() {
            let is_cross_chapter_segment = seg.starting_chapter() != seg.ending_chapter();
            let current_chapter = seg.ending_chapter();
            if let Some(chapter) = prev_chapter {
                if is_cross_chapter_segment || chapter == current_chapter {
                    output.push_str(verse_seperator);
                    output.push_str(&seg.chapterless_format());
                } else {
                    output.push_str(chapter_seperator);
                    output.push_str(&seg.to_string());
                }
            } else {
                output.push_str(&seg.to_string());
            }
            prev_chapter = Some(current_chapter);
        }
        output
    }
}

impl Segments {
    pub fn parse(segment_window: &str) -> Option<Self> {
        MinimalSegments::parse(segment_window).map(Segments::from)
    }
}

impl std::str::FromStr for Segments {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| String::from("Failed to parse segments"))
    }
}

impl std::fmt::Display for Segments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format(",", "; "))
    }
}

impl From<MinimalSegments> for Segments {
    fn from(value: MinimalSegments) -> Self {
        let mut segments = Segments::new();
        for seg in value.segments {
            let new = if let Some(start_verse) = seg.explicit_start_verse {
                let start_chapter = seg.start;
                if let Some(end) = seg.end {
                    if let Some(end_verse) = end.1 {
                        let end_chapter = end.0;
                        Segment::chapter_range(start_chapter, start_verse, end_chapter, end_verse)
                    } else {
                        let end_verse = end.0;
                        Segment::chapter_verse_range(start_chapter, start_verse, end_verse)
                    }
                } else {
                    Segment::chapter_verse(start_chapter, start_verse)
                }
            } else {
                if let Some(end) = seg.end {
                    if let Some(end_verse) = end.1 {
                        let start_chapter = seg.start;
                        let end_chapter = end.0;
                        Segment::chapter_range(start_chapter, 1, end_chapter, end_verse)
                    } else {
                        if let Some(prev) = segments.last() {
                            let start_verse = seg.start;
                            let end_verse = end.0;
                            Segment::chapter_verse_range(
                                prev.ending_chapter(),
                                start_verse,
                                end_verse,
                            )
                        } else {
                            let start_chapter = seg.start;
                            let end_chapter = end.0;
                            Segment::full_chapter_range(start_chapter, end_chapter)
                        }
                    }
                } else {
                    if let Some(prev) = segments.last() {
                        Segment::chapter_verse(prev.ending_chapter(), seg.start)
                    } else {
                        Segment::full_chapter(seg.start)
                    }
                }
            };
            segments.push(new);
        }
        segments
    }
}
