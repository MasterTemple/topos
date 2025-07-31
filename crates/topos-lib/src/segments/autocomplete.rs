use itertools::Itertools;

use crate::{
    data::{
        books::BookId,
        chapter_verses::{BookChapterVerses, ChapterVerses},
    },
    segments::{
        segment::Segment, segments::Segments, units::chapter_verse::ChapterVerse,
        verse_bounds::VerseBounds,
    },
};

// pub struct SegmentAutoCompleter {
//     chapter_verses: BookChapterVerses,
// }

pub struct SegmentAutoCompleter(BookChapterVerses);

#[derive(Clone, Copy, Debug)]
pub enum SegmentJoiner {
    None,
    /// Joined by characters like `-`
    Range,
    /// Joined by characters like `,` or `;`
    Separate,
    /// Joined by characters like `:`
    Chapter,
}

impl SegmentAutoCompleter {
    pub fn suggest(
        &self,
        book: &BookId,
        segments: &Segments,
        joiner: SegmentJoiner,
        // TODO: This will probably have to return Segments because there will be modifications to
        // the input segments when there is a range (because it gets joined)
        // Or I have some enum that has MergeLastSegment or AppendSegment
    ) -> Option<Vec<Segments>> {
        let chapter_verses = self.0.get_chapter_verses(book)?;
        let last_chapter = chapter_verses.get_chapter_count();

        // extract last segment, but if there is not one, suggest every chapter
        let Some(last) = segments.last() else {
            let first_chapter = 1;
            return Some(
                (1..=last_chapter)
                    .map(|ch| Segment::full_chapter(ch).as_segments())
                    .collect(),
            );
        };

        let current_verse = last.ending_verse();
        let current_chapter = last.ending_chapter();
        let last_verse = chapter_verses.get_last_verse(current_chapter)?;

        let remaining_verses = if let Some(current_verse) = current_verse {
            (current_verse + 1..=last_verse)
                .map(|v| Segment::chapter_verse(current_chapter, v))
                .collect_vec()
        } else {
            if let SegmentJoiner::Chapter = joiner {
                (1..=last_verse)
                    .map(|v| Segment::chapter_verse(current_chapter, v))
                    .collect_vec()
            } else {
                vec![]
            }
        };
        let remaining_chapters = (current_chapter + 1..=last_chapter)
            .map(|ch| Segment::full_chapter(ch))
            .collect_vec();

        // let remaining_chapters = 1;

        Some(match joiner {
            // if the joiner is a range, I will want to add
            SegmentJoiner::Range => {
                // let before_range = &segments[0..segments.len() - 1];
                let mut before_range = segments.clone();
                before_range.pop();

                let mut results = vec![];
                for seg in remaining_verses {
                    let mut prev = before_range.clone();
                    prev.push(seg);
                    results.push(prev);
                }
                for seg in remaining_chapters {
                    let mut prev = before_range.clone();
                    prev.push(seg);
                    results.push(prev);
                }
                results
            }
            // if it is separate, just suggest things that are after this, both verses and
            // chapters
            SegmentJoiner::Chapter | SegmentJoiner::None | SegmentJoiner::Separate => {
                let mut results = vec![];
                for seg in remaining_verses {
                    let mut prev = segments.clone();
                    prev.push(seg);
                    results.push(prev);
                }
                for seg in remaining_chapters {
                    let mut prev = segments.clone();
                    prev.push(seg);
                    results.push(prev);
                }
                results
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete() {
        let engine = SegmentAutoCompleter(BookChapterVerses::default());

        use SegmentJoiner as Joiner;

        // genesis has 50 chapters
        // chapter 1 has 31 verses
        // chapter 2 has 25 verses
        let genesis = |input: &str, joiner: SegmentJoiner| {
            engine
                .suggest(&BookId(1), &Segments::parse_str(input).unwrap(), joiner)
                .unwrap()
                .len()
        };

        assert_eq!(
            engine
                .suggest(&BookId(1), &Segments::new(), SegmentJoiner::None)
                .unwrap()
                .len(),
            50 // chapters
        );

        assert_eq!(
            genesis("1", Joiner::Range),
            49 // remaining chapters
        );

        assert_eq!(
            genesis("1", Joiner::Separate),
            49 // remaining chapters
        );

        assert_eq!(
            genesis("1", Joiner::Chapter),
            49 + 31 // remaining chapters + verses
        );

        // ---

        assert_eq!(
            genesis("1:1", Joiner::Range),
            49 + 30 // remaining chapters + remaining verses
        );

        // this should not be valid
        // assert_eq!(
        //     genesis("1:1", Joiner::Chapter),
        //     49 + 30 // remaining chapters + remaining verses
        // );

        assert_eq!(
            genesis("1:1", Joiner::Separate),
            49 + 30 // remaining chapters + remaining verses
        );

        // ---

        assert_eq!(
            genesis("1:2", Joiner::Range),
            49 + 29 // remaining chapters + remaining verses
        );

        assert_eq!(
            genesis("1:2", Joiner::Separate),
            49 + 29 // remaining chapters + remaining verses
        );

        // ---

        assert_eq!(
            genesis("2", Joiner::Range),
            48 // remaining chapters
        );

        assert_eq!(
            genesis("2", Joiner::Separate),
            48 // remaining chapters
        );

        assert_eq!(
            genesis("2", Joiner::Chapter),
            48 + 25 // remaining chapters + verses
        );

        // ---

        assert_eq!(
            genesis("2:1", Joiner::Range),
            48 + 24 // remaining chapters + remaining verses
        );

        assert_eq!(
            genesis("2:1", Joiner::Separate),
            48 + 24 // remaining chapters + remaining verses
        );

        // ---

        assert_eq!(
            genesis("2:2", Joiner::Range),
            48 + 23 // remaining chapters + remaining verses
        );

        assert_eq!(
            genesis("2:2", Joiner::Separate),
            48 + 23 // remaining chapters + remaining verses
        );

        // --
    }
}
