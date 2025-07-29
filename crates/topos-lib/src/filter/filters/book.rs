use std::collections::BTreeSet;

use crate::{data::books::BookId, filter::filter::IsFilter};

pub struct BookFilter {
    input: String,
}

impl IsFilter for BookFilter {
    fn get_ids(&self, data: &crate::data::data::BibleData) -> std::collections::BTreeSet<BookId> {
        let mut ids = BTreeSet::new();
        if let Some(value) = data.books().search(&self.input) {
            ids.insert(value);
        }
        ids
    }
}
