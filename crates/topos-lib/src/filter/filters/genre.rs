use std::collections::BTreeSet;

use crate::filter::filter::IsFilter;

pub struct GenreFilter {
    input: String,
}

impl IsFilter for GenreFilter {
    fn get_ids(&self, data: &crate::data::data::BibleData) -> std::collections::BTreeSet<u8> {
        let mut ids = BTreeSet::new();
        if let Some(value) = data.books().search(&self.input) {
            data.genres();
            ids.insert(value);
        }
        ids
    }
}
