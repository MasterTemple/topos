use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::data::books::{BookId, Books};

// TODO: Optimization, use Cow<String> for Genre Title, perhaps even with a new-type wrapper
// struct
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Genres {
    genres: BTreeMap<String, GenreInput>,
    /// Key/Abbreviation to Genre Title
    key_to_genre: BTreeMap<String, String>,
    genre_to_ids: BTreeMap<String, BTreeSet<BookId>>,
}

impl Genres {
    pub fn create(books: &Books, input: GenresInput) -> Self {
        let mut genres = BTreeMap::default();
        let mut key_to_genre = BTreeMap::default();
        let mut genre_to_keys = BTreeMap::default();

        for genre in input.0 {
            // search for book ids now so i only have to do it once
            if let Some(books_in_genre) = &genre.books {
                let ids = books_in_genre
                    .iter()
                    .filter_map(|b| books.search(b))
                    .collect();
                genre_to_keys.insert(genre.title.clone(), ids);
            }

            // use normalized abbreviations + title as keys
            for ab in &genre.abbreviations {
                let key = Self::normalize_key(ab);
                key_to_genre.insert(key, genre.title.clone());
            }
            let key = Self::normalize_key(&genre.title);
            key_to_genre.insert(key, genre.title.clone());

            // use title as the genre key
            genres.insert(genre.title.clone(), genre);
        }

        Self {
            genres,
            key_to_genre,
            genre_to_ids: genre_to_keys,
        }
    }

    /// - But this returned struct gives the user the input data; I need a separate type
    /// - Also this should return the key :/, not all the data
    pub fn search(&self, input: &str) -> Option<&GenreInput> {
        let key = Self::normalize_key(input);
        let key = self.key_to_genre.get(&key)?;
        self.genres.get(key)
    }

    pub fn genre_ids(&self, input: &str) -> Option<BTreeSet<BookId>> {
        let key = Self::normalize_key(input);
        let key = self.key_to_genre.get(&key)?;
        self.genre_to_ids.get(key).cloned()
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

// impl From<GenresInput> for Genres {
//     fn from(value: GenresInput) -> Self {
//         // let mut genres = BTreeMap::default();
//         let genres = value.0.into_iter().map(|g| (g.title.clone(), g)).collect();
//         Self { genres }
//     }
// }

impl Default for Genres {
    fn default() -> Self {
        Self::create(Books::base(), GenresInput::default())
    }
}

impl Genres {
    pub fn add(&mut self, genre: GenreInput) {
        _ = self.genres.insert(genre.title.clone(), genre);
    }
    pub fn add_many(&mut self, input: GenresInput) {
        for genre in input.0 {
            self.add(genre);
        }
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
