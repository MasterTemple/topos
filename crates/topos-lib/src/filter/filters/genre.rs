use crate::filter::filter::IsFilter;

pub struct GenreFilter {
    input: String,
}

impl IsFilter for GenreFilter {
    fn get_ids(&self, data: &crate::data::data::BibleData) -> std::collections::BTreeSet<u8> {
        data.genres().genre_ids(&self.input).unwrap_or_default()
    }
}
