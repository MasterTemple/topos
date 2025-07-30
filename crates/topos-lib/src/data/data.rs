use once_cell::sync::Lazy;

use crate::segments::segments::Passage;

use super::{books::Books, genres::Genres};

pub struct BibleData<'a> {
    books: Books,
    genres: Genres<'a>,
    // testaments: Test
}

impl<'a> BibleData<'a> {
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

impl<'a> Default for BibleData<'a> {
    fn default() -> Self {
        Self {
            books: Default::default(),
            genres: Default::default(),
        }
    }
}

impl<'a> BibleData<'a> {
    pub fn base() -> &'static Self {
        &DEFAULT_DATA
    }
}

static DEFAULT_DATA: Lazy<BibleData> = Lazy::new(|| BibleData {
    books: Books::base().clone(),
    genres: Genres::base().clone(),
});
