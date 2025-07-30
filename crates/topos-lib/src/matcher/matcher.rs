use std::collections::BTreeSet;

use itertools::Itertools;
use line_col::LineColLookup;
use once_cell::sync::Lazy;
use regex::{Match, Regex};

use crate::{
    data::data::BibleData,
    filter::filter::BibleFilter,
    matcher::{
        instance::{BibleMatch, Location},
        matches::{ComplexFilter, FilteredBibleMatches},
    },
    segments::{
        parse::SegmentInput,
        segments::{BookSegments, Segments},
    },
};

#[derive(Clone)]
pub struct BibleMatcher<'a> {
    data: &'a BibleData<'a>,
    /// The books to **not** match on are removed from this RegEx, so I won't process unnecessary
    /// books
    filtered_books: Regex,
    /// These are so I can check if the matches overlap with these
    complex_filter: ComplexFilter,
}

impl<'a> BibleMatcher<'a> {
    pub fn new(
        data: &'a BibleData<'a>,
        filtered_books: Regex,
        complex_filter: ComplexFilter,
    ) -> Self {
        Self {
            data,
            filtered_books,
            complex_filter,
        }
    }

    /// How can I make it so that I can iter over lines and take a string input or a BufReader
    /// input (I don't want to convert BufReader to a string because of performance overhead)
    pub fn search(&self, input: &str) -> Vec<BibleMatch> {
        // let mut matches: Vec<BibleMatch> = vec![];
        let mut filtered = FilteredBibleMatches::new(&self.complex_filter);

        let mut prev: Option<Match<'_>> = None;
        let lookup = LineColLookup::new(input);
        // basically execute behind by 1 iteration (so I can see the start of the next match)
        for cur in self.filtered_books.captures_iter(input) {
            // this is just the book name
            let cur = cur.get(1).unwrap();
            if let Some(prev) = prev {
                if let Some(m) =
                    BibleMatch::try_match(&lookup, self.data, input, prev, Some(cur.start()))
                {
                    filtered.try_add(m);
                }
            }
            prev = Some(cur);
        }

        // handle last one
        if let Some(prev) = prev {
            if let Some(m) = BibleMatch::try_match(&lookup, self.data, input, prev, None) {
                filtered.try_add(m);
            }
        }

        return filtered.matches();
    }
}

static DEFAULT_MATCHER: Lazy<BibleMatcher<'static>> = Lazy::new(|| BibleMatcher::default());

impl<'a> BibleMatcher<'a> {
    pub fn base() -> &'static Self {
        &DEFAULT_MATCHER
    }
}

impl Default for BibleMatcher<'static> {
    fn default() -> Self {
        Self::base().clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::data::books::BookId;

    use super::*;

    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    #[test]
    fn matcher() {
        let v = std::rc::Rc::new(true);
        let v = v.clone();
        let v = v.to_owned();
        let o = v.as_ref().clone();
        let o = std::rc::Rc::into_inner(v);
        // let data = BibleData::base();
        // let filtered_books = BibleFilter::default()
        //     // .add(Operation::Include(GenreFilter::new("Pauline")))
        //     .create_regex()
        //     .unwrap();
        //
        // let matcher = BibleFilter::default().create_matcher().unwrap();
        let matcher = BibleMatcher::default();

        let john = BookId(43);

        // let matcher = BibleMatcher {
        //     data,
        //     filtered_books,
        //     complex_filter: ComplexFilter::new(
        //         vec![Segments::parse_str("3").unwrap().with_book(john)],
        //         vec![Segments::parse_str("3:17-18").unwrap().with_book(john)],
        //     ),
        // };
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
                "Last, John 3:17",
                "Last, John 3:18",
                "Last, John 4:18",
                "Last, John 3:19",
            ]
            .join("\n")
            .as_str(),
        );
        // dbg!(results);
        for result in results {
            let psg = result.psg;
            println!("{} {}", *psg.book, psg.segments.iter().join("\n"));
        }
    }
}
