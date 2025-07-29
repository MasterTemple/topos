use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
};

use serde::{Deserialize, Serialize};

use crate::data::books::{BookId, Books};

/// This is not guaranteed to be a valid key, I just am using a unique type
#[derive(
    Clone,
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
pub struct GenreKey<'a>(Cow<'a, String>);

impl<'a> GenreKey<'a> {
    pub fn new(s: String) -> Self {
        Self(Cow::Owned(s))
    }
}

// TODO: Optimization, use Cow<String> for Genre Title, perhaps even with a new-type wrapper
// struct
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Genres<'a> {
    genres: BTreeMap<GenreKey<'a>, Genre<'a>>,
    /// Key/Abbreviation to Genre Title
    key_to_genre: BTreeMap<String, GenreKey<'a>>,
}

impl<'a> Genres<'a> {
    pub fn create(books: &Books, input: GenresInput) -> Self {
        let mut genres = BTreeMap::default();
        let mut key_to_genre = BTreeMap::default();
        // let mut genre_to_keys = BTreeMap::default();

        for genre in input.0 {
            // use title as key
            let ab = Self::normalize_key(&genre.title);
            let key = GenreKey::new(genre.title);
            key_to_genre.insert(ab, key.clone());

            // use normalized abbreviations as keys
            for ab in &genre.abbreviations {
                let ab = Self::normalize_key(ab);
                key_to_genre.insert(ab, key.clone());
            }

            // search for book ids now so i only have to do it once
            let ids = if let Some(books_in_genre) = &genre.books {
                let ids = books_in_genre
                    .iter()
                    .filter_map(|b| books.search(b))
                    .collect();
                // genre_to_keys.insert(key.clone(), ids);
                ids
            } else {
                BTreeSet::default()
            };

            let genre = Genre::new(key.clone(), ids);

            // use title as the genre key
            genres.insert(key.clone(), genre);
        }

        Self {
            genres,
            key_to_genre,
            // genre_to_ids: genre_to_keys,
        }
    }

    /// - But this returned struct gives the user the input data; I need a separate type
    /// - Also this should return the key :/, not all the data
    pub fn search(&'a self, input: &'_ str) -> Option<&'a GenreKey<'a>> {
        let key = Self::normalize_key(input);
        self.key_to_genre.get(&key)
    }

    pub fn get(&'a self, input: &'_ str) -> Option<&'a Genre<'a>> {
        let key = self.search(input)?;
        self.genres.get(&key)
    }

    pub fn genre_ids(&'a self, input: &'_ str) -> Option<&'a BTreeSet<BookId>> {
        Some(&self.get(input)?.books)
    }

    pub fn normalize_key(name: &str) -> String {
        name.to_lowercase()
            .trim()
            .trim_end_matches(".")
            .trim()
            .replace(" ", "-")
            .to_string()
    }
}

impl<'a> Default for Genres<'a> {
    fn default() -> Self {
        Self::create(Books::base(), GenresInput::default())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Genre<'a> {
    title: GenreKey<'a>,
    books: BTreeSet<BookId>,
}

impl<'a> Genre<'a> {
    pub fn new(key: GenreKey<'a>, books: BTreeSet<BookId>) -> Self {
        Self { title: key, books }
    }
}

/**
Example:
```jsonc
[
  {
    "title": "Major Prophets"
    "abbreviations": [ "major" ],
    "books": [ "Isaiah", "Jeremiah", "Lamentations", "Ezekiel", "Daniel" ]
  },
  {
    "title": "Minor Prophets"
    "abbreviations": [ "minor" ],
    "books": [ "Hosea", "Joel", "Amos", "Obadiah", "Jonah", "Micah", "Nahum", "Habakkuk", "Zephaniah", "Haggai", "Zechariah", "Malachi" ]
  },
  {
    "title": "Prophets"
    "abbreviations": [ "pr" ],
    "subcategories": [ "Major Prophets", "Minor Prophets" ],
  },
  // ...
]
```
*/
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenreInput {
    title: String,
    abbreviations: Vec<String>,
    books: Option<Vec<String>>,
    subcategories: Option<Vec<String>>,
}

static DEFAULT_GENRES_JSON: &'static str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/data/default_genres.json"
));

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenresInput(Vec<GenreInput>);

impl Default for GenresInput {
    fn default() -> Self {
        serde_json::from_str(&DEFAULT_GENRES_JSON)
            .map_err(|_| format!("Could not parse default file"))
            .unwrap()
    }
}
