pub enum SpaceOptions {
    DontTouch,
    RemoveAll,
    Normalize,
}

pub enum RomanNumeralOptions {
    DontTouch,
    MakeUppercase,
    MakeLowercase,
    MakeAllDecimal,
    MakeChaptersDecimal,
    MakeVersesDecimal,
}

pub enum DelimeterOptions {
    DontTouch,
    Normalize,
    NormalizeWith {
        chapter: Option<String>,
        range: Option<String>,
        chapter_segment: Option<String>,
        verse_segment: Option<String>,
    },
}

pub struct RangeOptions {
    /// `1-2:1` instead of `1:1-2:1`
    pub exclude_verse_1_for_chapter_range: bool,
    /// `1:1-2` instead of `1:1,2`
    pub join_adjacent_verses: bool,
    /// `Jude 1:1` instead of `Jude 1`
    pub use_chapter_in_single_chapter_books: bool,
}

pub struct FormatOptions {
    /// have spacing for numbers and each type of delimeter that overwrite this
    pub general_spacing: SpaceOptions,
    pub include_subverse: bool,
    pub roman: RomanNumeralOptions,
    pub delim: DelimeterOptions,
    pub range: RangeOptions,
}
