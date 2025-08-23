use crate::segments::{
    parser::verbose::VerboseFullSegment,
    segment::Segment,
    units::{
        chapter_range::ChapterRange, chapter_verse::ChapterVerse,
        chapter_verse_range::ChapterVerseRange, full_chapter::FullChapter,
        full_chapter_range::FullChapterRange, full_chapter_verse_range::FullChapterVerseRange,
    },
};

// BUG: This is wrong, it needs to be an enum that holds the data without options
pub struct FormattableSegment {
    pub raw: VerboseFullSegment,
    pub parsed: Segment,
}

pub enum VerboseSegmentPair {
    /// - This is a single chapter/verse reference
    /// - Ex: `1:2` in `John 1:2`
    ChapterVerse { parsed: ChapterVerse },
    /// - This is a range of verse references within a single chapter
    /// - Ex: `1:2-3` `John 1:2-3`
    ChapterVerseRange { parsed: ChapterVerseRange },
    /// - This is a range of verse references across a multiple chapters
    /// - Ex: `John 1:2-3:4`
    ChapterRange { parsed: ChapterRange },
    /// - This is a single chapter reference
    /// - Ex: `1` in `John 1`
    FullChapter { parsed: FullChapter },
    /// - This is a full chapter range reference
    /// - Ex: `1-2` in `John 1-2`
    FullChapterRange { parsed: FullChapterRange },
    /// - This is a full chapter to chapter-verse range reference
    /// - Ex: `1-2:3` in `John 1-2:3`
    FullChapterVerseRange { parsed: FullChapterVerseRange },
}

pub struct FormattableSegments {
    pub segments: Vec<FormattableSegment>,
}
