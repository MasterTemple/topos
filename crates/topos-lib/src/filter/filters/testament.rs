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

#[cfg(test)]
mod tests {
    use crate::filter::{filter::Operation, filters::testament::TestamentFilter};

    macro_rules! mk_test {
        ($fn_name: ident, [$($filter:expr),+ $(,)?], $count:literal) => {
            #[test]
            fn $fn_name() {
                let data = crate::data::data::BibleData::default();
                let mut filter = crate::filter::filter::BibleFilter::new(&data);
                $(
                    filter.add_filter($filter);
                )*
                assert_eq!(filter.ids().len(), $count);
            }
        };
    }

    mk_test!(
        exclude_both,
        [
            Operation::Exclude(TestamentFilter::Old),
            Operation::Exclude(TestamentFilter::New),
        ],
        0
    );

    mk_test!(
        exclude_include,
        [
            Operation::Exclude(TestamentFilter::New),
            Operation::Include(TestamentFilter::New),
        ],
        27
    );

    mk_test!(
        include_nt_filter,
        [Operation::Include(TestamentFilter::New)],
        27
    );

    mk_test!(
        include_ot_filter,
        [Operation::Include(TestamentFilter::Old)],
        39
    );

    mk_test!(
        exclude_nt_filter,
        [Operation::Exclude(TestamentFilter::New)],
        39
    );

    mk_test!(
        exclude_ot_filter,
        [Operation::Exclude(TestamentFilter::Old)],
        27
    );
}
