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
    segments::{Passage, grammar::SegmentList, resolve::Resolver},
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
        // The reference ends where the first translation starts, if only separators come between
        let named = translations.find(segments).and_then(|(name, range)| {
            let list = SegmentList::parse(&segments[..range.start]);
            let (_, following) = translations.following(&segments[list.end()..])?;
            (list.end() + following.start == range.start).then(|| {
                let translation = Translation {
                    name: name.to_string(),
                    span: offset + range.start..offset + range.end,
                };
                (list, translation)
            })
        });
        let (list, translation) = match named {
            Some((list, translation)) => (list, Some(translation)),
            None => (SegmentList::parse(segments), None),
        };
        let passage = self.resolve(book, &list)?;
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
        assert_eq!(data.genres().genre_ids("apocrypha").unwrap().len(), 24);
        // Words aren't references without a verse
        let matcher = crate::matcher::BibleMatcher::new(
            data.clone(),
            crate::matcher::matches::ComplexFilter::default(),
        );
        assert!(matcher.search("a bar 2 drinks, sir 3").is_empty());
        let found = matcher.search("see Sir 24:3 and Wisdom 7:26");
        let found: Vec<_> = found.iter().map(|m| name(&m.psg)).collect();
        assert_eq!(found, ["Sirach 24:3", "Wisdom of Solomon 7:26"]);
        // The defaults are unchanged
        assert_eq!(BibleData::default().books().search_osis("Tob"), None);
    }
}
