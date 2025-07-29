use std::collections::BTreeSet;

use itertools::Itertools;
use regex::Regex;

use crate::filter::data::BibleData;

pub trait GetIds {
    /// These are the ids that correspond to the argument, excluded or included
    fn ids(&self, data: &BibleData) -> BTreeSet<u8>;
}

pub enum Operation<T> {
    Include(T),
    Exclude(T),
}

impl<T> Operation<T> {
    pub fn inner(&self) -> &T {
        match self {
            Operation::Include(t) => t,
            Operation::Exclude(t) => t,
        }
    }
}

impl<T: GetIds> GetIds for Operation<T> {
    fn ids(&self, data: &BibleData) -> BTreeSet<u8> {
        self.inner().ids(data)
    }
}

pub struct BookFilter<'a> {
    data: &'a BibleData,
    /// indicates whether or not there has been an inclusion, which implicitly calls an exclusion
    /// on all the original data
    /// i dont need to use this if an exclusion is called at the beginning, but then again, there
    /// is no point in doing that, unless i am only doing an exclusion
    has_done_an_inclusion: bool,
    ids: BTreeSet<u8>,
}

impl<'a> BookFilter<'a> {
    pub fn new(data: &'a BibleData) -> Self {
        // this should start full
        let ids = (1..=66).collect();
        let has_done_an_inclusion = false;
        Self {
            data,
            ids,
            has_done_an_inclusion,
        }
    }

    pub fn add_filter<T: GetIds>(&mut self, op: Operation<T>) {
        let ids = op.ids(self.data);

        match op {
            Operation::Include(_) => {
                if self.has_done_an_inclusion {
                    self.ids.extend(ids);
                } else {
                    self.ids = ids;
                    self.has_done_an_inclusion = true;
                }
            }
            Operation::Exclude(_) => {
                self.ids.retain(|id| !ids.contains(&id));
            }
        };
    }

    pub fn create_regex(&self) -> Result<Regex, String> {
        let books_pattern: String = self
            .data
            .books()
            .abbrev_to_id()
            .iter()
            .filter_map(|(ab, id)| (self.ids.contains(&id).then_some(ab)))
            .join("|");

        let book_regex = Regex::new(format!(r"\b(((?:)(?i){books_pattern})[A-z]*)\.?").as_str())
            .map_err(|e| format!("Failed to compile book_regex because of bad user input.\n{e}"))?;

        Ok(book_regex)
    }
}
