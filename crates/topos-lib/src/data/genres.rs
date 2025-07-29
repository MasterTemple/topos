use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Genre {
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
pub struct Genres {
    genres: BTreeMap<String, Genre>,
}

impl From<GenresInput> for Genres {
    fn from(value: GenresInput) -> Self {
        // let mut genres = BTreeMap::default();
        let genres = value.0.into_iter().map(|g| (g.title.clone(), g)).collect();
        Self { genres }
    }
}

impl Default for Genres {
    fn default() -> Self {
        Self::from(GenresInput::default())
    }
}

impl Genres {
    pub fn add(&mut self, genre: Genre) {
        _ = self.genres.insert(genre.title.clone(), genre);
    }
    pub fn add_many(&mut self, input: GenresInput) {
        for genre in input.0 {
            self.add(genre);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenresInput(Vec<Genre>);

impl Default for GenresInput {
    fn default() -> Self {
        serde_json::from_str(&DEFAULT_GENRES_JSON)
            .map_err(|_| format!("Could not parse default file"))
            .unwrap()
    }
}
