use std::{collections::BTreeMap, rc::Rc};

use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::segments::segments::{Passage, Segments};

/// This is not guaranteed to be a valid key, I just am using a unique type
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    derive_more::From,
    derive_more::Deref,
    derive_more::DerefMut,
)]
pub struct BookId(pub u8);

// How do I want to do this so that I am not creating tons of strings? should I use Cow<String>s?
// pub struct BookInfo {
//     id: BookId,
//     name: String,
//     abbrev: String,
// }

/// eventually this will have a locale so i can group by languages
#[derive(Clone, Debug)]
pub struct Books {
    /// map of abbreviations and actual name (all lowercase) to book id (for searching)
    input_to_book_id: BTreeMap<String, BookId>,
    /// map of book id to book name (for display)
    book_id_to_name: BTreeMap<BookId, String>,
    /// map of book id to abbreviation (for display)
    book_id_to_abbreviation: BTreeMap<BookId, String>,
    /// match any book; used to parse complex filters
    passage_regex: Regex,
}

impl Books {
    fn key_to_id(&self) -> &BTreeMap<String, BookId> {
        &self.input_to_book_id
    }
    fn id_to_name(&self) -> &BTreeMap<BookId, String> {
        &self.book_id_to_name
    }
    fn id_to_abbrev(&self) -> &BTreeMap<BookId, String> {
        &self.book_id_to_abbreviation
    }
}

impl Books {
    pub fn iter_keys_and_ids(&self) -> impl Iterator<Item = (&String, &BookId)> {
        self.key_to_id().iter()
    }
    pub fn search(&self, name: &str) -> Option<BookId> {
        let name = Self::normalize_book_name(name);
        self.key_to_id().get(&name).cloned()
    }
    pub fn get_name(&self, id: BookId) -> Option<&String> {
        self.id_to_name().get(&id)
    }
    pub fn get_abbrev(&self, id: BookId) -> Option<&String> {
        self.id_to_abbrev().get(&id)
    }
}

impl Default for Books {
    fn default() -> Self {
        Self::base().clone()
    }
}

impl Books {
    /// - You only want to use this when you have custom data
    /// - If you would like English book names, please just use [`Default::default()`]
    pub fn new(data: BooksInput) -> Result<Self, String> {
        let mut abbreviations_to_book_id = BTreeMap::new();
        let mut book_id_to_name = BTreeMap::new();
        let mut book_id_to_abbreviation = BTreeMap::new();

        for book in data.0 {
            abbreviations_to_book_id.insert(Books::normalize_book_name(&book.book), book.id);
            book_id_to_name.insert(book.id, book.book);
            book_id_to_abbreviation.insert(book.id, book.abbreviation);
            for abbreviation in book.abbreviations {
                abbreviations_to_book_id.insert(Books::normalize_book_name(&abbreviation), book.id);
            }
        }

        let books_pattern: String = abbreviations_to_book_id.keys().join("|");

        let passage_regex = Regex::new(format!(r"\b(((?:)(?i){books_pattern}))\.?(.*)").as_str())
            .map_err(|e| {
            format!("Failed to compile book_regex because of bad user input.\n{e}")
        })?;

        Ok(Books {
            // book_regex,
            input_to_book_id: abbreviations_to_book_id,
            book_id_to_name,
            book_id_to_abbreviation,
            passage_regex,
        })
    }

    pub fn parse(&self, input: &str) -> Option<Passage> {
        let m = &self.passage_regex.captures_iter(input).next()?;
        let book = m.get(1)?.as_str();
        let book = self.search(book)?;
        let segments = m.get(2)?.as_str();
        let segments = Segments::parse_str(segments)?;
        Some(segments.with_book(book))
    }

    pub fn normalize_book_name(name: &str) -> String {
        name.to_lowercase()
            .trim()
            .trim_end_matches(".")
            .trim()
            .to_string()
    }

    /// - This is a global reference to the default book data. If you want to clone it, just use
    /// [`Default::default`]
    /// - This lets me reference it in other defaults without having to clone it
    pub fn base() -> &'static Self {
        &DEFAULT_BOOKS
    }
}

static DEFAULT_BOOKS: Lazy<Books> = Lazy::new(|| {
    let data = BooksInput::default();
    Books::new(data).expect("The default provided books data should always compile")
});

// static DEFAULT_BOOKS: Lazy<Arc<Books>> = Lazy::new(|| {
//     let data = BooksInput::default();
//     Arc::new(Books::new(data).expect("The default provided books data should always compile"))
// });

/**
Example:
```jsonc
[
  {
    "id": 1,
    "book": "Genesis",
    "abbreviation": "Gn",
    "abbreviations": [
      "gen",
      "ge",
      "gn"
    ]
  },
  // ...
]
```
*/
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Book {
    /// - book id, starting at 1
    /// - Genesis = 1
    /// - Matthew = 40
    #[serde(alias = "num")]
    #[serde(alias = "number")]
    id: BookId,

    /// - the display name
    /// - case is kept
    /// - does not need to be repeated in abbreviations
    #[serde(alias = "name")]
    #[serde(alias = "book_name")]
    #[serde(alias = "display_name")]
    book: String,

    /// - the display abbreviation
    /// - case is kept
    /// - does not need to be repeated in abbreviations
    /// - TODO: if not provided, the first abbreviations as title case; do that by changing this to
    /// a BookInput struct
    #[serde(alias = "abbr")]
    #[serde(alias = "abbrv")]
    #[serde(alias = "abbrev")]
    abbreviation: String,

    /// - does not need to include book name or abbreviation
    /// - meant for matching/parsing references
    #[serde(alias = "abbrs")]
    #[serde(alias = "abbrvs")]
    #[serde(alias = "abbrevs")]
    #[serde(default)]
    abbreviations: Vec<String>,
}

// static DEFAULT_BOOKS_JSON: &'static str = include_str!(concat!(
//     env!("CARGO_MANIFEST_DIR"),
//     "/src/data/default_books.json"
// ));

static DEFAULT_BOOKS_JSON: &'static str = include_str!("./default_books.json");

#[derive(Clone, Debug, Serialize, Deserialize)]
// #[derive(Deref, DerefMut, IntoIterator)]
pub struct BooksInput(Vec<Book>);

impl Default for BooksInput {
    fn default() -> Self {
        serde_json::from_str(&DEFAULT_BOOKS_JSON)
            .map_err(|_| format!("Could not parse default book file"))
            .unwrap()
    }
}
