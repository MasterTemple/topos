use crate::filter::{books::Books, genres::Genres};

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
}
