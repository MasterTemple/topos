use topos_parser::spanned_length::VerboseFullSegment;

pub enum VerseFormatContext {
    None,
    PrevChapter {
        previous_chapter: u8,
    },
    PrevChapterVerse {
        previous_chapter: u8,
        previous_verse: u8,
    },
}

pub struct FullFormatContext {
    // TODO: this really should be a parsed segment.. this leads me to realize that the formatter
    // should be a part of `topos-lib`
    pub segment: VerboseFullSegment,
    pub start: usize,
    pub is_after_range: bool,
    pub verse: VerseFormatContext,
}
