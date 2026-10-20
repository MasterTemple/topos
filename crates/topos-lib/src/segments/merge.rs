//! Passages joined: a list of verses as ranges (`Romans 8:28, 29, 30, 32` is `8:28-30,32`), and
//! several passages as one per book, in order, with overlapping and adjacent parts joined.

use std::collections::BTreeMap;

use crate::{
    data::{bible_data::BibleData, books::BookId, chapter_verses::ChapterVerses},
    segments::{
        coverage::VerseRange,
        passage::{Passage, Segments},
        segment::Segment,
        units::chapter_verse::ChapterVerse,
        verse_bounds::VerseBounds,
    },
};

/// A part of a passage from one verse to another; a whole chapter ends at
/// [`VerseRange::END_OF_CHAPTER`], so it stays a whole chapter when written back
#[derive(Clone, Copy, Debug)]
struct Span {
    start: ChapterVerse,
    end: ChapterVerse,
}

impl Span {
    fn of(segment: &Segment) -> Self {
        Self {
            start: ChapterVerse::new(segment.starting_chapter(), segment.starting_verse()),
            end: ChapterVerse::new(
                segment.ending_chapter(),
                (segment.ending_verse()).unwrap_or(VerseRange::END_OF_CHAPTER),
            ),
        }
    }

    /// The first verse after it: the next chapter's first after its chapter's last verse
    fn next(&self, versification: Option<&ChapterVerses>) -> ChapterVerse {
        let last = versification.and_then(|v| v.get_last_verse(self.end.chapter));
        if self.end.verse == VerseRange::END_OF_CHAPTER || Some(self.end.verse) == last {
            ChapterVerse::new(self.end.chapter.saturating_add(1), 1)
        } else {
            ChapterVerse::new(self.end.chapter, self.end.verse.saturating_add(1))
        }
    }

    fn segment(&self, versification: Option<&ChapterVerses>) -> Segment {
        let (start, end) = (self.start, self.end);
        if end.verse != VerseRange::END_OF_CHAPTER {
            return Segment::chapter_range(start.chapter, start.verse, end.chapter, end.verse);
        }
        if start.verse <= 1 {
            return Segment::full_chapter_range(start.chapter, end.chapter);
        }
        // From a verse to the end of a chapter (without versification, its end stays unknown)
        let last = versification.and_then(|v| v.get_last_verse(end.chapter));
        let last = last.unwrap_or(VerseRange::END_OF_CHAPTER);
        Segment::chapter_range(start.chapter, start.verse, end.chapter, last)
    }
}

/// The segments sorted, with those that overlap or follow each other joined
fn join(
    segments: impl IntoIterator<Item = Segment>,
    versification: Option<&ChapterVerses>,
) -> Segments {
    let mut spans: Vec<Span> = segments.into_iter().map(|s| Span::of(&s)).collect();
    spans.sort_by_key(|span| (span.start, span.end));
    let mut joined: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans {
        match joined.last_mut() {
            Some(last) if span.start <= last.next(versification) => {
                last.end = last.end.max(span.end);
            }
            _ => joined.push(span),
        }
    }
    Segments(
        joined
            .iter()
            .map(|span| span.segment(versification))
            .collect(),
    )
}

impl Passage {
    /**
    The verses as one passage, sorted, without repeats, and with verses that follow each other as
    ranges: `8:32, 8:28, 8:29, 8:30` is `8:28-30,32`; with `versification`, a chapter's last verse
    and the next chapter's first join too (`3:36, 4:1` is `3:36-4:1`). [`None`] for no verses

    ```
    use topos_bible::{
        data::BibleData,
        segments::{Passage, units::chapter_verse::ChapterVerse},
    };

    let data = BibleData::default();
    let romans = data.books().search("Romans").unwrap();
    let verses = [32, 28, 29, 30, 28].map(|verse| ChapterVerse::new(8, verse));
    let versification = data.chapter_verses().get_chapter_verses(&romans);
    let passage = Passage::from_verses(romans, verses, versification).unwrap();
    assert_eq!(passage.segments.to_string(), "8:28-30,32");
    ```
    */
    pub fn from_verses(
        book: BookId,
        verses: impl IntoIterator<Item = ChapterVerse>,
        versification: Option<&ChapterVerses>,
    ) -> Option<Passage> {
        let verses = verses
            .into_iter()
            .map(|cv| Segment::chapter_verse(cv.chapter, cv.verse));
        let segments = join(verses, versification);
        (!segments.is_empty()).then(|| segments.with_book(book))
    }

    /**
    This passage's segments sorted, with those that overlap or follow each other joined
    (`3:16-18, 17-20; 3` is `3`, and `5; 4` is `4-5`); whole chapters stay whole chapters
    */
    pub fn joined(&self, versification: Option<&ChapterVerses>) -> Passage {
        join(self.segments.iter().copied(), versification).with_book(self.book)
    }

    /**
    The passages as one per book, in the books' order, each [joined](Self::joined): the union
    of their verses

    ```
    use topos_bible::{data::BibleData, segments::Passage};

    let data = BibleData::default();
    let passages = ["Rom 8:30-32", "John 3:16", "Rom 8:28-29", "John 3"].map(|r| data.parse(r).unwrap());
    let merged: Vec<_> = Passage::merge(passages, &data)
        .iter()
        .map(|p| format!("{} {}", data.books().get_name(p.book).unwrap(), p.segments))
        .collect();
    assert_eq!(merged, ["John 3", "Romans 8:28-32"]);
    ```
    */
    pub fn merge(passages: impl IntoIterator<Item = Passage>, data: &BibleData) -> Vec<Passage> {
        let mut books: BTreeMap<BookId, Vec<Segment>> = BTreeMap::new();
        for passage in passages {
            (books.entry(passage.book).or_default()).extend(passage.segments.iter().copied());
        }
        books
            .into_iter()
            .map(|(book, segments)| {
                let versification = data.chapter_verses().get_chapter_verses(&book);
                join(segments, versification).with_book(book)
            })
            .filter(|passage| !passage.segments.is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(reference: &str) -> String {
        let data = BibleData::default();
        let passage = data.parse(reference).unwrap();
        let versification = data.chapter_verses().get_chapter_verses(&passage.book);
        passage.joined(versification).segments.to_string()
    }

    #[test]
    fn joins_segments() {
        assert_eq!(joined("John 3:16-18, 17-20; 3"), "3");
        assert_eq!(joined("John 5; 4"), "4-5");
        assert_eq!(joined("John 3:36; 4:1"), "3:36-4:1");
        assert_eq!(joined("John 3:16, 18"), "3:16,18");
        assert_eq!(joined("John 3:1-36; 4"), "3-4");
        assert_eq!(joined("John 3:16-4:2; 4"), "3:16-4:54");
        assert_eq!(joined("John 1-2:3; 2:4-10"), "1:1-2:10");
        assert_eq!(joined("Jude 3, 1, 2"), "1:1-3");
    }

    #[test]
    fn verses_without_versification() {
        let book = BookId(43);
        let verses = [(3, 36), (4, 1), (4, 2)].map(|(c, v)| ChapterVerse::new(c, v));
        let passage = Passage::from_verses(book, verses, None).unwrap();
        assert_eq!(passage.segments.to_string(), "3:36; 4:1-2");
        assert_eq!(Passage::from_verses(book, [], None), None);
    }

    #[test]
    fn merges_books() {
        let data = BibleData::default();
        let passages =
            ["Gen 1:1", "Rev 22:21", "Gen 1:2", "Rev 22:20"].map(|r| data.parse(r).unwrap());
        let merged: Vec<_> = Passage::merge(passages, &data)
            .iter()
            .map(|p| p.segments.to_string())
            .collect();
        assert_eq!(merged, ["1:1-2", "22:20-21"]);
        assert!(Passage::merge([], &data).is_empty());
    }
}
