use std::str::FromStr;

use crate::filter::filter::IsFilter;

#[derive(Copy, Clone, Debug)]
pub enum Testament {
    Old,
    New,
}

impl Testament {
    pub fn contains(&self, book_id: u8) -> bool {
        match self {
            Testament::Old => 1 <= book_id && book_id <= 39,
            Testament::New => 40 <= book_id && book_id <= 66,
        }
    }
}

impl IsFilter for Testament {
    fn get_ids(&self, _data: &crate::data::data::BibleData) -> std::collections::BTreeSet<u8> {
        match self {
            Testament::Old => 1..=39,
            Testament::New => 40..=66,
        }
        .collect()
    }
}

impl FromStr for Testament {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "n" | "nt" | "new" | "new testament" => Self::New,
            "o" | "ot" | "old" | "old testament" => Self::Old,
            _ => Err("Invalid Testament")?,
        })
    }
}
