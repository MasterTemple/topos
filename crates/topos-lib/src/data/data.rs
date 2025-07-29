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
}
