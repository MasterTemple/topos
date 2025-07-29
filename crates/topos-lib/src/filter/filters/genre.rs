use crate::{data::books::BookId, filter::filter::IsFilter};

pub struct GenreFilter {
    input: String,
}

impl IsFilter for GenreFilter {
    fn get_ids(&self, data: &crate::data::data::BibleData) -> std::collections::BTreeSet<BookId> {
        data.genres().genre_ids(&self.input).unwrap_or_default()
    }
}
