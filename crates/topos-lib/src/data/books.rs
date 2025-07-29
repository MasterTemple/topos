use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// eventually this will have a locale so i can group by languages
#[derive(Clone, Debug)]
pub struct Books {
    /// map of abbreviations and actual name (all lowercase) to book id (for searching)
    abbreviations_to_book_id: BTreeMap<String, u8>,
    /// map of book id to book name (for display)
    book_id_to_name: BTreeMap<u8, String>,
    /// map of book id to abbreviation (for display)
    book_id_to_abbreviation: BTreeMap<u8, String>,
}

impl Books {
    pub fn search(&self, name: &str) -> Option<u8> {
        let name = Self::normalize_book_name(name);
        self.abbrev_to_id().get(&name).cloned()
    }
    pub fn abbrev_to_id(&self) -> &BTreeMap<String, u8> {
        &self.abbreviations_to_book_id
    }
    pub fn id_to_name(&self) -> &BTreeMap<u8, String> {
        &self.book_id_to_name
    }
    pub fn id_to_abbrev(&self) -> &BTreeMap<u8, String> {
        &self.book_id_to_abbreviation
    }
}

static DEFAULT_BOOKS_JSON: &'static str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/data/default_books.json"
));

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
    id: u8,

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
    /// - TODO: if not provided, the first abbreviations
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

#[derive(Clone, Debug, Serialize, Deserialize)]
// #[derive(Deref, DerefMut, IntoIterator)]
pub struct BooksInput(Vec<Book>);

impl Default for BooksInput {
    fn default() -> Self {
        serde_json::from_str(&DEFAULT_BOOKS_JSON)
            .map_err(|_| format!("Could not parse default file"))
            .unwrap()
    }
}

impl Default for Books {
    fn default() -> Self {
        let data = BooksInput::default();
        Self::new(data).expect("The default provided data data should always compile")
    }
}

impl<'a> Books {
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

        Ok(Books {
            // book_regex,
            abbreviations_to_book_id,
            book_id_to_name,
            book_id_to_abbreviation,
        })
    }

    pub fn normalize_book_name(name: &str) -> String {
        name.to_lowercase()
            .trim()
            .trim_end_matches(".")
            .trim()
            .to_string()
    }
}
