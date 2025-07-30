use once_cell::sync::Lazy;

use crate::segments::segments::Passage;

use super::{books::Books, genres::Genres};

#[derive(Clone, Debug)]
pub struct BibleData {
    books: Books,
    genres: Genres,
    // testaments: Test
}

impl BibleData {
    pub fn books(&self) -> &Books {
        &self.books
    }
    pub fn genres(&self) -> &Genres {
        &self.genres
    }

    // pub fn parse(&self, input: &str) -> Option<BookSegments> {
    //
    // }
}

impl Default for BibleData {
    fn default() -> Self {
        Self {
            books: Default::default(),
            genres: Default::default(),
        }
    }
}

// impl BibleData {
//     pub fn base() -> &'static Self {
//         &DEFAULT_DATA
//     }
// }
//
// static DEFAULT_DATA: Lazy<BibleData> = Lazy::new(|| BibleData {
//     books: Books::base().clone(),
//     genres: Genres::default(),
// });
