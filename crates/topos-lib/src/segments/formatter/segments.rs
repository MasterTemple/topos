use crate::segments::{
    parser::verbose::{
        VerboseFullSegment, VerboseSegments,
        components::{DelimitedNumber, FrontPadded, VerboseNumber},
    },
    segment::Segment,
    units::{
        chapter_range::ChapterRange, chapter_verse::ChapterVerse,
        chapter_verse_range::ChapterVerseRange, full_chapter::FullChapter,
        full_chapter_range::FullChapterRange, full_chapter_verse_range::FullChapterVerseRange,
        range_pair::RangePair,
    },
};

// // BUG: This is wrong, it needs to be an enum that holds the data without options
// pub struct FormattableSegment {
//     pub raw: VerboseFullSegment,
//     pub parsed: Segment,
// }

pub struct LeadingSpace {}

pub enum VerboseSegmentPair {
    /// - This is a single chapter/verse reference
    /// - Ex: `1:2` in `John 1:2`
    ChapterVerse(VerbosePair<VerboseChapterVerse, ChapterVerse>),
    /// - This is a range of verse references within a single chapter
    /// - Ex: `1:2-3` `John 1:2-3`
    ChapterVerseRange(VerbosePair<VerboseChapterVerseRange, ChapterVerseRange>),
    /// - This is a range of verse references across a multiple chapters
    /// - Ex: `John 1:2-3:4`
    ChapterRange(VerbosePair<VerboseChapterRange, ChapterRange>),
    // /// - This is a single chapter reference
    // /// - Ex: `1` in `John 1`
    // FullChapter { parsed: FullChapter },
    // /// - This is a full chapter range reference
    // /// - Ex: `1-2` in `John 1-2`
    // FullChapterRange { parsed: FullChapterRange },
    // /// - This is a full chapter to chapter-verse range reference
    // /// - Ex: `1-2:3` in `John 1-2:3`
    // FullChapterVerseRange { parsed: FullChapterVerseRange },
}

pub struct VerbosePair<Raw, Parsed> {
    raw: Raw,
    parsed: Parsed,
}

impl<Raw, Parsed> VerbosePair<Raw, Parsed> {
    pub fn new(raw: Raw, parsed: Parsed) -> Self {
        Self { raw, parsed }
    }
}

pub struct VerboseChapterVerse {
    pub start_chapter: FrontPadded<VerboseNumber>,
    pub start_verse: FrontPadded<DelimitedNumber>,
}

pub struct VerboseChapterVerseRange {
    // pub chapter: FrontPadded<VerboseNumber>,
    // pub verses: RangePair<FrontPadded<DelimitedNumber>>,
    pub start_chapter: FrontPadded<VerboseNumber>,
    pub start_verse: FrontPadded<DelimitedNumber>,
    pub end_verse: FrontPadded<DelimitedNumber>,
}

pub struct VerboseChapterRange {
    pub start_chapter: FrontPadded<VerboseNumber>,
    pub start_verse: FrontPadded<DelimitedNumber>,
    pub end_chapter: FrontPadded<DelimitedNumber>,
    pub end_verse: FrontPadded<DelimitedNumber>,
}

pub struct VerboseFullChapter {
    pub chapter: FrontPadded<VerboseNumber>,
}

pub struct VerboseFullChapterRange {
    pub start: FrontPadded<VerboseNumber>,
    pub end: FrontPadded<DelimitedNumber>,
}

pub struct VerboseFullChapterVerseRange {
    pub start_chapter: FrontPadded<VerboseNumber>,
    pub end: RangePair<FrontPadded<DelimitedNumber>>,
}

pub struct FormattableSegments {
    pub segments: Vec<VerboseSegmentPair>,
}

impl From<VerboseSegments> for FormattableSegments {
    fn from(value: VerboseSegments) -> Self {
        let mut segments = Vec::new();
        for seg in value.segments {
            let VerboseFullSegment {
                start,
                explicit_start_verse,
                end,
                closing,
            } = seg.clone();

            let new = if let Some(start_verse) = explicit_start_verse {
                let start_chapter = start;
                if let Some(end) = end {
                    match end {
                        // `1:2-3:4`
                        (end_chapter, Some(end_verse)) => {
                            let parsed = {
                                let start_chapter = start_chapter.parsed_value();
                                let start_verse = start_verse.parsed_value();
                                let end_verse = end_verse.parsed_value();
                                let end_chapter = end_chapter.parsed_value();

                                ChapterRange::new(
                                    start_chapter,
                                    start_verse,
                                    end_chapter,
                                    end_verse,
                                )
                            };

                            let raw = VerboseChapterRange {
                                start_chapter,
                                start_verse,
                                end_chapter,
                                end_verse,
                            };

                            VerboseSegmentPair::ChapterRange(VerbosePair::new(raw, parsed))
                        }
                        // `1:2-3`
                        (end_verse, None) => {
                            let parsed = {
                                let start_chapter = start_chapter.parsed_value();
                                let start_verse = start_verse.parsed_value();
                                let end_verse = end_verse.parsed_value();

                                ChapterVerseRange::new(start_chapter, start_verse, end_verse)
                            };

                            let raw = VerboseChapterVerseRange {
                                start_chapter,
                                start_verse,
                                end_verse,
                            };

                            VerboseSegmentPair::ChapterVerseRange(VerbosePair::new(raw, parsed))
                        }
                    }
                }
                // `1:2`
                else {
                    let parsed = {
                        let start_chapter = start_chapter.parsed_value();
                        let start_verse = start_verse.parsed_value();

                        ChapterVerse::new(start_chapter, start_verse)
                    };

                    let raw = VerboseChapterVerse {
                        start_chapter,
                        start_verse,
                    };

                    VerboseSegmentPair::ChapterVerse(VerbosePair::new(raw, parsed))
                }
            } else {
                todo!()
                // if let Some(end) = seg.end {
                //     // `1:2-3:4`
                //     if let Some(end_verse) = end.1 {
                //         let start_chapter = seg.start;
                //         let end_chapter = end.0;
                //         Segment::chapter_range(start_chapter, 1, end_chapter, end_verse)
                //     } else {
                //         // `3:4-5`
                //         if let Some(prev) = segments.last() {
                //             let start_verse = seg.start;
                //             let end_verse = end.0;
                //             Segment::chapter_verse_range(
                //                 prev.ending_chapter(),
                //                 start_verse,
                //                 end_verse,
                //             )
                //         }
                //         // `1-25`
                //         else {
                //             let start_chapter = seg.start;
                //             let end_chapter = end.0;
                //             Segment::full_chapter_range(start_chapter, end_chapter)
                //         }
                //     }
                // } else {
                //     // `1:1`
                //     if let Some(prev) = segments.last() {
                //         Segment::chapter_verse(prev.ending_chapter(), seg.start)
                //     }
                //     // `1`
                //     else {
                //         Segment::full_chapter(seg.start)
                //     }
                // }
            };
            segments.push(new);
        }
        Self { segments }
    }
}
