//! Which books to search, as data (a settings file's or an app's JSON): testaments, genres, and
//! books, included and excluded, resolved the way [`BibleFilter`] resolves them.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    data::{bible_data::BibleData, books::BookId},
    error::ToposResult,
    filter::{
        bible_filter::BibleFilter,
        filters::{book::BookFilter, genre::GenreFilter, testament::TestamentFilter},
    },
};

/**
Which books to search, as [`BibleFilter`] filters them:

- Included testaments limit the search to them
- Included genres and books add up, within those testaments
- Excluded testaments, genres, and books are always left out

So the New Testament with `Pauline Epistles` is Paul's letters, `Pentateuch` with `Revelation`
(a book) is six books, and nothing at all is every book. Genres and books go by any of their
names or abbreviations (`Gospels`, `pauline`, `1 Cor`), and testaments are `old` or `new`.

In JSON every field is optional, in snake case or camel case (`exclude_books` or
`excludeBooks`), and unknown fields are an error:

```
use topos_bible::{data::BibleData, filter::book_scope::BookScope};

let data = BibleData::default();
let scope: BookScope =
    serde_json::from_str(r#"{ "genres": ["Gospels"], "excludeBooks": ["John"] }"#).unwrap();
let books: Vec<u8> = scope.books(&data).unwrap().into_iter().map(|b| b.0).collect();
assert_eq!(books, [40, 41, 42]);

// Filters that leave nothing say why
let none: BookScope =
    serde_json::from_str(r#"{ "testaments": ["old"], "genres": ["Pauline Epistles"] }"#).unwrap();
assert!(none.books(&data).unwrap().is_empty());
assert!(none.filter(&data).unwrap().contradiction().is_some());
```
*/
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BookScope {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub testaments: Vec<TestamentFilter>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub books: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", alias = "excludeTestaments")]
    pub exclude_testaments: Vec<TestamentFilter>,
    #[serde(skip_serializing_if = "Vec::is_empty", alias = "excludeGenres")]
    pub exclude_genres: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", alias = "excludeBooks")]
    pub exclude_books: Vec<String>,
}

impl BookScope {
    /// Whether it names nothing, so every book is searched
    pub fn is_everything(&self) -> bool {
        *self == Self::default()
    }

    /// Adds its testaments, genres, and books to `filter` (an error names an unknown genre or
    /// book)
    pub fn apply(&self, filter: &mut BibleFilter) -> ToposResult<()> {
        filter.include_many(self.testaments.iter().copied())?;
        filter.include_many(self.genres.iter().map(GenreFilter::new))?;
        filter.include_many(self.books.iter().map(BookFilter::new))?;
        filter.exclude_many(self.exclude_testaments.iter().copied())?;
        filter.exclude_many(self.exclude_genres.iter().map(GenreFilter::new))?;
        filter.exclude_many(self.exclude_books.iter().map(BookFilter::new))?;
        Ok(())
    }

    /// A filter over `data` with these books, which also says why nothing can match, if so
    /// ([`BibleFilter::contradiction`]), and makes a matcher
    pub fn filter(&self, data: &BibleData) -> ToposResult<BibleFilter> {
        let mut filter = BibleFilter::new(data.clone());
        self.apply(&mut filter)?;
        Ok(filter)
    }

    /// The books searched, in order: every book for an empty scope, and none when its filters
    /// contradict each other (see [`Self::filter`] for why)
    pub fn books(&self, data: &BibleData) -> ToposResult<BTreeSet<BookId>> {
        Ok(self.filter(data)?.ids().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn books(json: &str) -> ToposResult<Vec<u8>> {
        let scope: BookScope = serde_json::from_str(json).unwrap();
        let books = scope.books(&BibleData::default())?;
        Ok(books.into_iter().map(|b| b.0).collect())
    }

    #[test]
    fn books_by_testament_genre_and_book() {
        assert_eq!(books("{}").unwrap().len(), 66);
        assert!(BookScope::default().is_everything());
        let nt = books(r#"{ "testaments": ["New"] }"#).unwrap();
        assert_eq!((nt.len(), nt[0], nt[26]), (27, 40, 66));
        let paul = books(r#"{ "testaments": ["nt"], "genres": ["pauline"] }"#).unwrap();
        assert_eq!(paul.len(), 13);
        // Revelation is a book, not a genre
        assert!(books(r#"{ "genres": ["Pentateuch", "Revelation"] }"#).is_err());
        assert_eq!(
            books(r#"{ "genres": ["Pentateuch"], "books": ["Rev"] }"#).unwrap(),
            [1, 2, 3, 4, 5, 66]
        );
        assert_eq!(
            books(r#"{ "genres": ["Gospels"], "exclude_books": ["John"] }"#).unwrap(),
            [40, 41, 42]
        );
        assert_eq!(
            books(r#"{ "excludeTestaments": ["old"] }"#).unwrap().len(),
            27
        );
        assert!(
            books(r#"{ "testaments": ["old"], "genres": ["Pauline Epistles"] }"#)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn json_is_checked_and_written_back() {
        assert!(serde_json::from_str::<BookScope>(r#"{ "book": ["John"] }"#).is_err());
        assert!(serde_json::from_str::<BookScope>(r#"{ "testaments": ["middle"] }"#).is_err());
        let scope = BookScope {
            testaments: vec![TestamentFilter::New],
            exclude_books: vec!["John".into()],
            ..BookScope::default()
        };
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, r#"{"testaments":["new"],"exclude_books":["John"]}"#);
        assert_eq!(serde_json::from_str::<BookScope>(&json).unwrap(), scope);
    }
}
