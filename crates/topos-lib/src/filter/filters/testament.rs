use std::str::FromStr;

use itertools::Itertools;

use crate::{data::books::BookId, filter::filter::IsFilter};

#[derive(Copy, Clone, Debug)]
pub enum TestamentFilter {
    Old,
    New,
}

impl TestamentFilter {
    pub fn contains(&self, book_id: BookId) -> bool {
        match self {
            TestamentFilter::Old => 1 <= *book_id && *book_id <= 39,
            TestamentFilter::New => 40 <= *book_id && *book_id <= 66,
        }
    }
}

impl IsFilter for TestamentFilter {
    fn get_ids(&self, _data: &crate::data::data::BibleData) -> std::collections::BTreeSet<BookId> {
        match self {
            TestamentFilter::Old => 1..=39,
            TestamentFilter::New => 40..=66,
        }
        .map_into()
        .collect()
    }
}

impl FromStr for TestamentFilter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "n" | "nt" | "new" | "new testament" => Self::New,
            "o" | "ot" | "old" | "old testament" => Self::Old,
            _ => Err("Invalid Testament")?,
        })
    }
}
