use std::marker::PhantomData;

use line_col::LineColLookup;
use regex::{Match, Regex};

use crate::{
    data::data::BibleData,
    filter::filter::BibleFilter,
    matcher::{
        instance::BibleMatch,
        location::line_col::LineColLocation,
        matches::{ComplexFilter, FilteredBibleMatches},
    },
    segments::autocomplete::input::InputAutoCompleter,
};

#[derive(Clone, Debug)]
pub struct BibleMatcher<L = LineColLocation> {
    data: BibleData,
    /// The books to **not** match on **aren't** in this RegEx, so I won't process unnecessary books
    pub filtered_books: Regex,
    /// These are so I can check if the matches overlap with these
    complex_filter: ComplexFilter,
    _phantom: PhantomData<L>,
}

// TODO: I should have a search method for each type of Location
impl BibleMatcher {
    pub fn new(data: BibleData, filtered_books: Regex, complex_filter: ComplexFilter) -> Self {
        Self {
            data,
            filtered_books,
            complex_filter,
            _phantom: PhantomData,
        }
    }

    pub fn data(&self) -> &BibleData {
        &self.data
    }

    pub fn filter(&self) -> FilteredBibleMatches<'_> {
        self.complex_filter.as_filter()
    }

    pub fn completer(&self) -> InputAutoCompleter {
        InputAutoCompleter::new(self)
    }
}

/**
This is a trait that allows for generic location matching
*/
pub trait Matcher: Sized {
    type Input<'a>;
    fn search<'a>(matcher: &BibleMatcher<Self>, input: Self::Input<'a>) -> Vec<BibleMatch<Self>>;
    fn find<'a>(matcher: &BibleMatcher<Self>, input: Self::Input<'a>) -> Option<BibleMatch<Self>> {
        Self::search(matcher, input).into_iter().next()
    }
}

impl<L: Matcher> BibleMatcher<L> {
    pub fn search<'a>(&self, input: L::Input<'a>) -> Vec<BibleMatch<L>> {
        L::search(self, input)
    }

    pub fn find<'a>(&self, input: L::Input<'a>) -> Option<BibleMatch<L>> {
        L::find(self, input)
    }
}

impl Default for BibleMatcher {
    fn default() -> Self {
        BibleFilter::default()
            .create_matcher()
            .expect("The default provided matcher should always compile")
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::data::books::BookId;
//
//     use super::*;
//
//     use std::{
//         fs::File,
//         io::{BufRead, BufReader},
//     };
//
//     #[test]
//     fn matcher() {
//         let v = Arc::new(true);
//         let v = v.clone();
//         let v = v.to_owned();
//         let o = v.as_ref().clone();
//         let o = Arc::into_inner(v);
//         // let data = BibleData::base();
//         // let filtered_books = BibleFilter::default()
//         //     // .add(Operation::Include(GenreFilter::new("Pauline")))
//         //     .create_regex()
//         //     .unwrap();
//         //
//         // let matcher = BibleFilter::default().create_matcher().unwrap();
//         let matcher = BibleMatcherData::default();
//
//         let john = BookId(43);
//
//         // let matcher = BibleMatcher {
//         //     data,
//         //     filtered_books,
//         //     complex_filter: ComplexFilter::new(
//         //         vec![Segments::parse_str("3").unwrap().with_book(john)],
//         //         vec![Segments::parse_str("3:17-18").unwrap().with_book(john)],
//         //     ),
//         // };
//         let results = matcher.search(
//             vec![
//                 "Hello there",
//                 "Here is some text",
//                 "Oh wow, John 3:16",
//                 "John 1:1-2 and Ephesians 4:28",
//                 "Even John 1:1-4, 1 John 2:1-10",
//                 "Can I even do Ephesians",
//                 "1:1-3? I guess not",
//                 "Last, John 3:16",
//                 "Last, John 3:17",
//                 "Last, John 3:18",
//                 "Last, John 4:18",
//                 "Last, John 3:19",
//             ]
//             .join("\n")
//             .as_str(),
//         );
//         // dbg!(results);
//         for result in results {
//             let psg = result.psg;
//             println!("{} {}", *psg.book, psg.segments.iter().join("\n"));
//         }
//     }
// }
