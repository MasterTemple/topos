use itertools::Itertools;
use line_col::LineColLookup;
use regex::{Match, Regex};

use crate::{
    data::data::BibleData,
    filter::filter::BibleFilter,
    matcher::instance::{BibleInstance, Location},
    segments::{parse::SegmentInput, segments::Segments},
};

pub struct BibleMatcher<'a> {
    data: &'a BibleData<'a>,
    filtered_books: Regex,
}

impl<'a> BibleMatcher<'a> {
    pub fn new(data: &'a BibleData<'a>, filtered_books: Regex) -> Self {
        Self {
            data,
            filtered_books,
        }
    }

    /// How can I make it so that I can iter over lines and take a string input or a BufReader
    /// input (I don't want to convert BufReader to a string because of performance overhead)
    pub fn search(&self, input: &str) -> Vec<BibleInstance> {
        let mut matches: Vec<BibleInstance> = vec![];
        let mut prev: Option<Match<'_>> = None;
        let lookup = LineColLookup::new(input);
        // basically execute behind once
        for cur in self.filtered_books.captures_iter(input) {
            // this is just the book name
            let cur = cur.get(1).unwrap();
            match prev {
                Some(prev) => {
                    if let Some(m) = BibleInstance::try_process(
                        &lookup,
                        self.data,
                        input,
                        prev,
                        Some(cur.start()),
                    ) {
                        matches.push(m);
                    }
                }
                None => (),
            };
            prev = Some(cur);
        }

        // handle last one
        if let Some(prev) = prev {
            if let Some(m) = BibleInstance::try_process(&lookup, self.data, input, prev, None) {
                matches.push(m);
            }
        }

        return matches;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    #[test]
    fn matcher() {
        let data = BibleData::base();
        let filtered_books = BibleFilter::default()
            // .add(Operation::Include(GenreFilter::new("Pauline")))
            .create_regex()
            .unwrap();
        let matcher = BibleMatcher {
            data,
            filtered_books,
        };
        let results = matcher.search(
            vec![
                "Hello there",
                "Here is some text",
                "Oh wow, John 3:16",
                "John 1:1-2 and Ephesians 4:28",
                "Even John 1:1-4, 1 John 2:1-10",
                "Can I even do Ephesians",
                "1:1-3? I guess not",
                "Last, John 3:16",
            ]
            .join("\n")
            .as_str(),
        );
        dbg!(results);
    }
}
