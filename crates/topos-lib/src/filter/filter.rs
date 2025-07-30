use std::collections::BTreeSet;

use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    data::{books::BookId, data::BibleData},
    matcher::{matcher::BibleMatcher, matches::ComplexFilter},
};

pub trait IsFilter {
    /// These are the ids that correspond to the argument, excluded or included
    fn get_ids(&self, data: &BibleData) -> BTreeSet<BookId>;
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

impl<T: IsFilter> IsFilter for Operation<T> {
    fn get_ids(&self, data: &BibleData) -> BTreeSet<BookId> {
        self.inner().get_ids(data)
    }
}

#[derive(Clone)]
pub struct BibleFilter<'a> {
    data: &'a BibleData<'a>,
    /// indicates whether or not there has been an inclusion, which implicitly calls an exclusion
    /// on all the original data
    /// i dont need to use this if an exclusion is called at the beginning, but then again, there
    /// is no point in doing that, unless i am only doing an exclusion
    has_done_an_inclusion: bool,
    ids: BTreeSet<BookId>,
}

impl<'a> BibleFilter<'a> {
    pub fn new(data: &'a BibleData) -> Self {
        // this should start full
        let ids = (1..=66).map_into().collect();
        let has_done_an_inclusion = false;
        Self {
            data,
            ids,
            has_done_an_inclusion,
        }
    }

    pub fn push<T: IsFilter>(&mut self, op: Operation<T>) {
        let ids = op.get_ids(self.data);

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

    pub fn add<T: IsFilter>(mut self, op: Operation<T>) -> BibleFilter<'a> {
        self.push(op);
        self
    }

    pub fn ids(&self) -> &BTreeSet<BookId> {
        &self.ids
    }

    /**
    The problem is that a RegEx isn't enough
    I need to create/return a struct that contains that regex and the segment regex, so that
    */
    pub fn create_regex(&self) -> Result<Regex, String> {
        let books_pattern: String = self
            .data
            .books()
            .iter_keys_and_ids()
            .filter_map(|(key, id)| (self.ids.contains(&id).then_some(key)))
            .join("|");

        // let book_regex = Regex::new(format!(r"\b(((?:)(?i){books_pattern})[A-z]*)\.?").as_str())
        // I am including a chapter number to reduce false positives on abbreviations
        let book_regex = Regex::new(format!(r"\b(((?:)(?i){books_pattern}))\.?\s*\d").as_str())
            .map_err(|e| format!("Failed to compile book_regex because of bad user input.\n{e}"))?;

        Ok(book_regex)
    }

    pub fn create_matcher(&self) -> Result<BibleMatcher, String> {
        Ok(BibleMatcher::new(
            self.data,
            self.create_regex()?,
            ComplexFilter::default(),
        ))
    }
}

impl Default for BibleFilter<'static> {
    fn default() -> Self {
        Self::new(BibleData::base())
    }
}

static DEFAULT_FILTER: Lazy<BibleFilter<'static>> = Lazy::new(|| BibleFilter::default());

impl<'a> BibleFilter<'a> {
    pub fn base() -> &'static Self {
        &DEFAULT_FILTER
    }
}

#[cfg(test)]
mod tests {
    use crate::filter::{
        filter::{BibleFilter, Operation},
        filters::genre::GenreFilter,
    };

    #[test]
    fn make_regex() {
        let re = BibleFilter::default()
            // .add(Operation::Include(GenreFilter::new("Pauline")))
            .create_regex()
            .unwrap();
        println!(r#"rg "{}""#, re.as_str());
    }
}
