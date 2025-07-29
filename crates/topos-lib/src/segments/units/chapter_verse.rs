use crate::segments::{
    parse::{ParsableSegment, SegmentParseMethods},
    segment::Segment,
    verse_bounds::VerseBounds,
};
use serde::{Deserialize, Serialize, de::Visitor};
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

/// - This is a single chapter/verse reference
/// - Ex: `1:2` in `John 1:2`
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord)]
pub struct ChapterVerse {
    pub chapter: u8,
    pub verse: u8,
}

impl Display for ChapterVerse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.chapter, self.verse)
    }
}

impl Serialize for ChapterVerse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

struct ChapterVerseVisitor;

impl<'de> Visitor<'de> for ChapterVerseVisitor {
    type Value = ChapterVerse;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("format '{}:{}'")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        Ok(ChapterVerse::new(
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom("missing chapter"))?,
            seq.next_element()?
                .ok_or_else(|| serde::de::Error::custom("missing verse"))?,
        ))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse().map_err(|e| E::custom(e))
    }
}

impl<'de> Deserialize<'de> for ChapterVerse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ChapterVerseVisitor)
    }
}

impl FromStr for ChapterVerse {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl VerseBounds for ChapterVerse {
    fn starting_chapter(&self) -> u8 {
        self.chapter
    }

    fn starting_verse(&self) -> u8 {
        self.verse
    }

    fn ending_chapter(&self) -> u8 {
        self.chapter
    }

    fn ending_verse(&self) -> Option<u8> {
        Some(self.verse)
    }
}

impl PartialOrd for ChapterVerse {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(
            self.chapter
                .cmp(&other.chapter)
                .then(self.verse.cmp(&other.verse)),
        )
    }
}

impl ChapterVerse {
    pub fn new(chapter: u8, verse: u8) -> Self {
        ChapterVerse { chapter, verse }
    }
}

impl Into<Segment> for ChapterVerse {
    fn into(self) -> Segment {
        Segment::ChapterVerse(self)
    }
}

impl TryFrom<Segment> for ChapterVerse {
    type Error = String;

    fn try_from(value: Segment) -> Result<Self, Self::Error> {
        Ok(match value {
            Segment::ChapterVerse(chapter_verse) => chapter_verse,
            Segment::ChapterVerseRange(_) => {
                Err(format!("Cannot coerce ChapterVerseRange into ChapterVerse"))?
            }
            Segment::ChapterRange(_) => {
                Err(format!("Cannot coerce ChapterRange into ChapterVerse"))?
            }
            Segment::FullChapter(_) => Err(format!("Cannot coerce FullChapter into ChapterVerse"))?,
            Segment::FullChapterRange(_) => {
                Err(format!("Cannot coerce FullChapterRange into ChapterVerse"))?
            }
        })
    }
}

impl ParsableSegment for ChapterVerse {
    const EXPECTED_FORMAT: &'static str = "{}:{}";

    fn parse_strict(input: &str) -> Result<Self, String> {
        let chars = &mut input.chars().peekable();

        let chapter = ChapterVerse::take_number(chars)?;
        ChapterVerse::expect_char(chars, ':')?;
        let verse = ChapterVerse::take_number(chars)?;
        ChapterVerse::expect_done(chars)?;

        Ok(ChapterVerse::new(chapter, verse))
    }
}

#[cfg(test)]
mod chapter_verse_tests {}
