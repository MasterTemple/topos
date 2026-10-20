use serde::{Deserialize, Serialize};

use crate::{
    data::{
        books::{BookId, Books, BooksInput},
        chapter_verses::{BookChapterVerses, BookChapterVersesInput},
        genres::{Genres, GenresInput},
        patch::DataPatch,
        translations::{TranslatedPassage, Translation, Translations},
    },
    error::ToposResult,
    segments::{Passage, Segment, grammar::SegmentList, resolve::Resolver},
};

#[derive(Clone, Debug, Default)]
pub struct BibleData {
    books: Books,
    genres: Genres,
    chapter_verses: BookChapterVerses,
}

/**
Custom data, as read from a JSON config; anything left out uses the defaults

```jsonc
{
  "books": [{ "id": 1, "book": "Genesis", "abbreviation": "Gn", "abbreviations": ["gen"] }],
  "genres": [{ "title": "Torah", "abbreviations": ["law"], "books": ["Genesis"] }],
  "chapter_verses": { "Genesis": [31, 25] }
}
```
*/
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BibleDataInput {
    #[serde(default)]
    pub books: Option<BooksInput>,
    #[serde(default)]
    pub genres: Option<GenresInput>,
    #[serde(default)]
    pub chapter_verses: Option<BookChapterVersesInput>,
}

impl BibleData {
    /// Genres and chapter counts refer to books by name, so they are read against these books
    pub fn new(input: BibleDataInput) -> ToposResult<Self> {
        let books = match input.books {
            Some(books) => Books::new(books)?,
            None => Books::default(),
        };
        let genres = Genres::create(&books, input.genres.unwrap_or_default());
        let chapter_verses =
            BookChapterVerses::create(&books, input.chapter_verses.unwrap_or_default());
        Ok(Self {
            books,
            genres,
            chapter_verses,
        })
    }

    /**
    One reference, like `jn 3:16-18` or `Jude 5`, resolved with the book's chapters and verses
    the way search does: `Jude 5` is verse 5 (Jude has one chapter), and a reference that doesn't
    exist (`John 3:99`) is [`None`]; parts after one that doesn't exist are ignored, like search
    */
    pub fn parse(&self, input: &str) -> Option<Passage> {
        let (book, _, segments) = self.books.split_reference(input)?;
        self.resolve(book, &SegmentList::parse(segments))
    }

    /**
    A book's name or abbreviation alone (`Genesis`, `gen`, `1 Cor.`), as the whole book (see
    [`BibleData::whole_book`]); [`None`] for anything else, including a reference (`Gen 1`), which
    [`BibleData::parse`] reads
    - A name alone is unambiguous when it is all of the input, so this suits a single reference
      typed or stored on its own; in running text, `Mark`, `Job`, and `Acts` are also words (see
      [`BibleMatcher::with_books_alone`](crate::matcher::BibleMatcher::with_books_alone))

    ```
    use topos_bible::data::BibleData;

    let data = BibleData::default();
    let genesis = data.parse_book(" Gen ").unwrap();
    assert_eq!(genesis.segments.to_string(), "1-50");
    assert_eq!(data.parse_book("Jude").unwrap().segments.to_string(), "1");
    assert_eq!(data.parse_book("Gen 1"), None);
    // A reference, else a book alone
    let parse = |input: &str| data.parse(input).or_else(|| data.parse_book(input));
    assert_eq!(parse("Gen 1").unwrap().segments.to_string(), "1");
    assert_eq!(parse("Gen"), Some(genesis));
    ```
    */
    pub fn parse_book(&self, input: &str) -> Option<Passage> {
        self.whole_book(self.books.search(input.trim())?)
    }

    /**
    Every chapter of `book`: `1-50` for Genesis, and `1` for a single-chapter book like Jude;
    [`None`] if the data has no chapter counts for it
    */
    pub fn whole_book(&self, book: BookId) -> Option<Passage> {
        let chapters = self
            .chapter_verses
            .get_chapter_verses(&book)?
            .get_chapter_count();
        if chapters == 0 {
            return None;
        }
        Some(
            Segment::full_chapter_range(1, chapters)
                .as_segments()
                .with_book(book),
        )
    }

    /**
    One reference, like [`BibleData::parse`], and the translation named right after it, if any:
    `John 3:16 ESV`, `John 3:16, ESV`, `John 3:16 (ESV)`, or `John 3:16 [ESV]`
    - Only the names in `translations` are translations, matched ignoring case
    - A reference without one is still parsed, with no translation
    - A name is never read as part of the reference, so `Ps 22, LXX` is Psalm 22 in the LXX
      (not Psalms 22 and 70)

    ```
    use topos_bible::data::{BibleData, translations::Translations};

    let data = BibleData::default();
    let translations = Translations::new(["ESV", "KJV"]);
    let found = data.parse_with_translation("John 3:16 (kjv)", &translations).unwrap();
    assert_eq!(found.passage, data.parse("John 3:16").unwrap());
    assert_eq!(found.translation.unwrap().name, "KJV");
    let found = data.parse_with_translation("John 3:16", &translations).unwrap();
    assert_eq!(found.translation, None);
    ```
    */
    pub fn parse_with_translation(
        &self,
        input: &str,
        translations: &Translations,
    ) -> Option<TranslatedPassage> {
        let (book, offset, segments) = self.books.split_reference(input)?;
        let (list, named) = translations.read_segments(segments, segments.len());
        let translation = named.map(|(name, range)| Translation::at(name, range, offset));
        let versification = (translation.as_ref())
            .and_then(|t| translations.chapter_verses(&t.name, book))
            .or_else(|| self.chapter_verses.get_chapter_verses(&book));
        let resolved = Resolver::for_book(versification).resolve(&list.nodes);
        let passage = (!resolved.segments.is_empty()).then(|| resolved.segments.with_book(book))?;
        Some(TranslatedPassage {
            passage,
            translation,
        })
    }

    /// The segments of a reference to `book`, checked against its chapters and verses
    fn resolve(&self, book: BookId, list: &SegmentList) -> Option<Passage> {
        let versification = self.chapter_verses.get_chapter_verses(&book);
        let resolved = Resolver::for_book(versification).resolve(&list.nodes);
        (!resolved.segments.is_empty()).then(|| resolved.segments.with_book(book))
    }

    /// The default data with the deuterocanonical books added (see [`DataPatch::deuterocanon`])
    pub fn with_deuterocanon() -> Self {
        let mut input = BibleDataInput::defaults();
        input
            .merge(DataPatch::deuterocanon())
            .expect("the deuterocanon data merges into the defaults");
        Self::new(input).expect("the default data with the deuterocanon is valid")
    }

    pub fn books(&self) -> &Books {
        &self.books
    }

    pub fn genres(&self) -> &Genres {
        &self.genres
    }

    pub fn chapter_verses(&self) -> &BookChapterVerses {
        &self.chapter_verses
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_like_search() {
        let data = BibleData::default();
        let parse = |input: &str| {
            let passage = data.parse(input)?;
            Some(format!(
                "{} {}",
                data.books().get_name(passage.book)?,
                passage.segments
            ))
        };
        // A single-chapter book's number is a verse
        assert_eq!(parse("Jude 5").as_deref(), Some("Jude 1:5"));
        assert_eq!(parse("jn 3:16-18").as_deref(), Some("John 3:16-18"));
        assert_eq!(parse("John 3").as_deref(), Some("John 3"));
        // Like search, references that don't exist aren't references
        assert_eq!(parse("John 3:99"), None);
        assert_eq!(parse("John 3:16, 4:99").as_deref(), Some("John 3:16"));
        assert_eq!(parse("not a reference"), None);
    }

    use super::*;

    #[test]
    fn parses_a_translation_after_a_reference() {
        use crate::data::chapter_verses::ChapterVerses;
        let data = BibleData::default();
        let translations =
            Translations::new(["ESV", "KJV", "LXX", "New King James Version"]).alias("AV", "KJV");
        let parse = |input: &str| {
            let found = data.parse_with_translation(input, &translations)?;
            let name = data.books().get_name(found.passage.book)?;
            let translation = found.translation.map(|t| t.name);
            Some((
                format!("{name} {}", found.passage.segments),
                translation.unwrap_or_default(),
            ))
        };
        let both = |reference: &str, translation: &str| {
            Some((reference.to_string(), translation.to_string()))
        };
        assert_eq!(parse("John 3:16 ESV"), both("John 3:16", "ESV"));
        assert_eq!(parse("see Jn 3:16-18, esv."), both("John 3:16-18", "ESV"));
        assert_eq!(parse("John 3:16 (AV)"), both("John 3:16", "KJV"));
        assert_eq!(parse("John 3:16 [KJV] says"), both("John 3:16", "KJV"));
        assert_eq!(parse("Jude 5 KJV"), both("Jude 1:5", "KJV"));
        assert_eq!(
            parse("John 3:16; 4:1 — New King James Version"),
            both("John 3:16; 4:1", "New King James Version")
        );
        // A name that would be a Roman numeral ends the reference
        assert_eq!(parse("Ps 22, LXX"), both("Psalm 22", "LXX"));
        // No translation, or one that isn't right after the reference
        assert_eq!(parse("John 3:16"), both("John 3:16", ""));
        assert_eq!(parse("John 3:16 NIV"), both("John 3:16", ""));
        assert_eq!(parse("John 3:16 is in the ESV"), both("John 3:16", ""));
        assert_eq!(parse("John 3:16 (ESV"), both("John 3:16", ""));
        // A name glued to the reference isn't one (and the reference reads like `parse`'s)
        assert_eq!(parse("John 3:16ESV"), both("John 3", ""));
        // Without a reference, there is nothing
        assert_eq!(parse("John ESV"), None);
        assert_eq!(parse("John 3:99 ESV"), None);
        // The span is in the input
        let found = data
            .parse_with_translation("Read Jn 3:16 (kjv)", &translations)
            .unwrap();
        assert_eq!(found.translation.unwrap().span, 14..17);
        // A translation's own chapter and verse counts
        let third_john = data.books().search("3 John").unwrap();
        let translations =
            translations.with_chapter_verses("ESV", third_john, ChapterVerses::new(vec![15]));
        let parse = |input: &str| {
            let found = data.parse_with_translation(input, &translations)?;
            Some(found.passage.segments.to_string())
        };
        // The default counts (the KJV's) end 3 John at verse 14, the ESV's at 15
        assert_eq!(parse("3 John 1:15"), None);
        assert_eq!(parse("3 John 1:15 KJV"), None);
        assert_eq!(parse("3 John 1:15 esv"), Some("1:15".into()));
        assert_eq!(parse("3 John 1:16 ESV"), None);
        // Without translations, it parses like `parse`
        let found = data
            .parse_with_translation("Ps 22, LXX", &Translations::default())
            .unwrap();
        assert_eq!(Some(found.passage), data.parse("Ps 22, LXX"));
        assert_eq!(found.translation, None);
    }

    #[test]
    fn custom_data_from_json() {
        let input: BibleDataInput = serde_json::from_str(
            r#"{
                "books": [{ "id": 1, "book": "Genesis", "abbreviation": "Gn", "abbreviations": ["bereshit"] }],
                "chapter_verses": { "Genesis": [31, 25] }
            }"#,
        )
        .unwrap();
        let data = BibleData::new(input).unwrap();
        let genesis = data.books().search("Bereshit").unwrap();
        assert_eq!(data.books().search("Exodus"), None);
        let versification = data.chapter_verses().get_chapter_verses(&genesis).unwrap();
        assert_eq!(versification.get_chapter_count(), 2);
    }

    #[test]
    fn deuterocanon() {
        let data = BibleData::with_deuterocanon();
        let books = data.books();
        let name = |p: &Passage| format!("{} {}", books.get_name(p.book).unwrap(), p.segments);
        assert_eq!(name(&data.parse("Sir 24:3").unwrap()), "Sirach 24:3");
        assert_eq!(
            name(&data.parse("1 Macc 1:1-4").unwrap()),
            "1 Maccabees 1:1-4"
        );
        // A single-chapter book's number is a verse
        assert_eq!(
            name(&data.parse("Bel 5").unwrap()),
            "Bel and the Dragon 1:5"
        );
        assert_eq!(data.parse("Tob 1:99"), None);
        // Every added book has an OSIS id, and every book of SWORD's versifications is known
        for osis in [
            "Tob", "Jdt", "EsthGr", "AddEsth", "Wis", "Sir", "Bar", "EpJer", "PrAzar", "Sus",
            "Bel", "AddDan", "1Macc", "2Macc", "1Esd", "PrMan", "AddPs", "3Macc", "2Esd", "4Macc",
            "Odes", "PssSol", "1En", "EpLao",
        ] {
            let id = books.search_osis(osis).unwrap();
            assert!(
                data.chapter_verses().get_chapter_verses(&id).is_some(),
                "{osis}"
            );
        }
        // The other books with USFM 3 codes, at the ids logoxide's book table gives them; they
        // have no chapter and verse counts, so any chapter and verse parses
        for (id, usfm, osis, reference, expected) in [
            (91, "DAG", "DanGr", "Gk Dan 3:24", "Greek Daniel 3:24"),
            (92, "PS3", "5ApocSyrPss", "Syr Pss 2:1", "Syriac Psalms 2:1"),
            (93, "2BA", "2Bar", "2 Bar 29:5", "2 Baruch 29:5"),
            (
                94,
                "LBA",
                "EpBar",
                "Letter of Baruch 1:2",
                "Letter of Baruch 1:2",
            ),
            (95, "JUB", "Jub", "Jubilees 1:1", "Jubilees 1:1"),
            (96, "1MQ", "1Meq", "1 Meqabyan 2:3", "1 Meqabyan 2:3"),
            (97, "2MQ", "2Meq", "II Meq 1:1", "2 Meqabyan 1:1"),
            (98, "3MQ", "3Meq", "3 Meq 4:5-6", "3 Meqabyan 4:5-6"),
            (99, "REP", "Rep", "Reproof 1:1", "Reproof 1:1"),
            (100, "4BA", "4Bar", "4 Baruch 9:1", "4 Baruch 9:1"),
            (101, "EZA", "4Ezra", "4 Ezra 7:28", "4 Ezra 7:28"),
            (102, "5EZ", "5Ezra", "5 Ezra 2:1", "5 Ezra 2:1"),
            (103, "6EZ", "6Ezra", "6 Ezra 15:1", "6 Ezra 15:1"),
        ] {
            let book = BookId(id);
            assert_eq!(books.search_usfm(usfm), Some(book), "{usfm}");
            assert_eq!(books.search_osis(osis), Some(book), "{osis}");
            assert_eq!(name(&data.parse(reference).unwrap()), expected);
        }
        assert_eq!(data.genres().genre_ids("apocrypha").unwrap().len(), 37);
        // Words aren't references without a verse
        let matcher = crate::matcher::BibleMatcher::new(
            data.clone(),
            crate::matcher::matches::ComplexFilter::default(),
        );
        assert!(matcher.search("a bar 2 drinks, sir 3").is_empty());
        assert!(matcher.search("one rep 5 times").is_empty());
        let found = matcher.search("Jubilees 1:1 and 2 Baruch 29:5 but Baruch 3:9 and 2 Bar. 3:1");
        let found: Vec<_> = found.iter().map(|m| name(&m.psg)).collect();
        assert_eq!(
            found,
            [
                "Jubilees 1:1",
                "2 Baruch 29:5",
                "Baruch 3:9",
                "2 Baruch 3:1"
            ]
        );
        let found = matcher.search("see Sir 24:3 and Wisdom 7:26");
        let found: Vec<_> = found.iter().map(|m| name(&m.psg)).collect();
        assert_eq!(found, ["Sirach 24:3", "Wisdom of Solomon 7:26"]);
        // The defaults are unchanged
        assert_eq!(BibleData::default().books().search_osis("Tob"), None);
    }
}
