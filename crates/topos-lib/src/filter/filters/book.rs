use std::collections::BTreeSet;

use crate::filter::filter::IsFilter;

pub struct BookFilter {
    input: String,
}

impl IsFilter for BookFilter {
    fn get_ids(&self, data: &crate::data::data::BibleData) -> std::collections::BTreeSet<u8> {
        let mut ids = BTreeSet::new();
        if let Some(value) = data.books().search(&self.input) {
            ids.insert(value)
        }
        ids
    }
}
