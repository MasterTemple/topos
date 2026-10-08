//! USFM references (`JHN 3:16-18`), as USX's `loc` attributes and YouVersion write them.

use crate::{
    data::books::{BookId, Books},
    error::{ToposError, ToposResult},
    segments::{passage::Passage, segment::Segment, verse_bounds::VerseBounds},
};

impl Passage {
    /**
    The passage as USFM references, one per segment, joined with `; `
    - `John 3:16-18; 4` is `JHN 3:16-18; JHN 4`
    - Chapters are always written, so `Jude 5` is `JUD 1:5`
    - [`None`] if the book has no USFM code in the data
    */
    pub fn to_usfm(&self, books: &Books) -> Option<String> {
        let book = books.get_usfm(self.book)?;
        let refs: Vec<String> = self
            .segments
            .iter()
            .map(|seg| {
                let (c1, c2) = (seg.starting_chapter(), seg.ending_chapter());
                let v1 = seg.starting_verse();
                match seg {
                    Segment::ChapterVerse(_) => format!("{book} {c1}:{v1}"),
                    Segment::FullChapter(_) => format!("{book} {c1}"),
                    Segment::FullChapterRange(_) => format!("{book} {c1}-{c2}"),
                    Segment::FullChapterVerseRange(_) => {
                        format!(
                            "{book} {c1}-{c2}:{}",
                            seg.ending_verse().unwrap_or_default()
                        )
                    }
                    Segment::ChapterVerseRange(_) => {
                        format!(
                            "{book} {c1}:{v1}-{}",
                            seg.ending_verse().unwrap_or_default()
                        )
                    }
                    Segment::ChapterRange(_) => format!(
                        "{book} {c1}:{v1}-{c2}:{}",
                        seg.ending_verse().unwrap_or_default()
                    ),
                }
            })
            .collect();
        Some(refs.join("; "))
    }
}

impl Books {
    /**
    Parses USFM references in one book: `JHN 3:16`, `JHN 3:16-18`, `JHN 3:16-4:2`, `JHN 3`,
    `JHN 3-4`, `JHN 3-4:2`, and several joined with `;` (`JHN 3:16; 4:1` or `JHN 3:16; JHN 4:1`)
    - Book codes ignore case
    - Every number is read as written: `JUD 5` is chapter 5 (USFM always writes the chapter)
    */
    pub fn parse_usfm(&self, input: &str) -> ToposResult<Passage> {
        let invalid = || ToposError::InvalidUsfm(input.to_string());
        let mut book: Option<BookId> = None;
        let mut segments = vec![];
        for part in input.split(';').map(str::trim) {
            let (code, numbers) = match part.split_once(char::is_whitespace) {
                Some((code, numbers)) if self.search_usfm(code).is_some() => {
                    (Some(code), numbers.trim())
                }
                _ => (None, part),
            };
            let part_book = match code {
                Some(code) => self.search_usfm(code),
                None => book,
            }
            .ok_or_else(invalid)?;
            if *book.get_or_insert(part_book) != part_book {
                return Err(invalid());
            }
            segments.push(Self::usfm_segment(numbers).ok_or_else(invalid)?);
        }
        let book = book.ok_or_else(invalid)?;
        Ok(Passage {
            book,
            segments: crate::segments::passage::Segments(segments.into()),
        })
    }

    /// `3:16-18` as a segment
    fn usfm_segment(numbers: &str) -> Option<Segment> {
        let point = |s: &str| -> Option<(u8, Option<u8>)> {
            match s.split_once(':') {
                Some((c, v)) => Some((c.trim().parse().ok()?, Some(v.trim().parse().ok()?))),
                None => Some((s.trim().parse().ok()?, None)),
            }
        };
        let (start, end) = match numbers.split_once('-') {
            Some((start, end)) => (start, Some(end)),
            None => (numbers, None),
        };
        let start = point(start)?;
        Some(match (start, end) {
            ((c, None), None) => Segment::full_chapter(c),
            ((c, Some(v)), None) => Segment::chapter_verse(c, v),
            ((c, Some(v)), Some(end)) => match point(end)? {
                // `3:16-18`: the end is a verse in the same chapter
                (v2, None) => Segment::chapter_verse_range(c, v, v2),
                (c2, Some(v2)) => Segment::chapter_range(c, v, c2, v2),
            },
            ((c, None), Some(end)) => match point(end)? {
                (c2, None) => Segment::full_chapter_range(c, c2),
                (c2, Some(v2)) => Segment::chapter_range(c, 1, c2, v2),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::data::BibleData;

    #[test]
    fn round_trips() {
        let data = BibleData::default();
        let books = data.books();
        for (reference, usfm) in [
            ("John 3:16", "JHN 3:16"),
            ("John 3:16-18", "JHN 3:16-18"),
            ("John 3:16-4:2", "JHN 3:16-4:2"),
            ("1 Samuel 3", "1SA 3"),
            ("Romans 1-3", "ROM 1-3"),
            ("John 3:16, 18; 4", "JHN 3:16; JHN 3:18; JHN 4"),
        ] {
            let passage = books.parse(reference).unwrap();
            assert_eq!(passage.to_usfm(books).unwrap(), usfm, "{reference}");
            assert_eq!(books.parse_usfm(usfm).unwrap(), passage, "{usfm}");
        }
        // The book can be left out after the first, and codes ignore case
        assert_eq!(
            books.parse_usfm("jhn 3:16; 4:1").unwrap(),
            books.parse("John 3:16; 4:1").unwrap()
        );
        assert_eq!(
            data.parse("Jude 5").unwrap().to_usfm(books).unwrap(),
            "JUD 1:5"
        );
    }

    #[test]
    fn rejects_invalid_usfm() {
        let books = BibleData::default();
        let books = books.books();
        for input in ["", "JHN", "XYZ 3:16", "JHN 3:16; ROM 1:1", "JHN x", "3:16"] {
            assert!(books.parse_usfm(input).is_err(), "{input}");
        }
    }

    #[test]
    fn every_book_has_a_code() {
        let data = BibleData::with_deuterocanon();
        let books = data.books();
        let missing: Vec<_> = books
            .ids()
            .filter(|&id| books.get_usfm(id).is_none())
            .filter_map(|id| books.get_osis(id).cloned())
            .collect();
        // USFM has whole Greek Esther and Daniel (ESG, DAG), not books of their additions
        assert_eq!(missing, ["AddEsth", "AddDan"]);
        assert_eq!(books.search_usfm("1co"), books.search("1 Corinthians"));
    }
}
