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
    use crate::{
        data::data::BibleData,
        filter::{
            filter::{BibleFilter, Operation},
            filters::testament::TestamentFilter,
        },
    };

    macro_rules! mk_test {
        ($fn_name: ident, [$($filter:expr),+ $(,)?], $count:literal) => {
            #[test]
            fn $fn_name() {
                let data = BibleData::default();
                let mut filter = BibleFilter::new(&data);
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

    #[test]
    fn include_new_testament_filter() {
        let data = BibleData::default();
        let mut filter = BibleFilter::new(&data);
        filter.add_filter(Operation::Include(TestamentFilter::New));
        assert_eq!(filter.ids().len(), 27);
    }

    #[test]
    fn include_old_testament_filter() {
        let data = BibleData::default();
        let mut filter = BibleFilter::new(&data);
        filter.add_filter(Operation::Include(TestamentFilter::Old));
        assert_eq!(filter.ids().len(), 39);
    }

    #[test]
    fn exclude_new_testament_filter() {
        let data = BibleData::default();
        let mut filter = BibleFilter::new(&data);
        filter.add_filter(Operation::Exclude(TestamentFilter::New));
        assert_eq!(filter.ids().len(), 39);
    }

    #[test]
    fn exclude_old_testament_filter() {
        let data = BibleData::default();
        let mut filter = BibleFilter::new(&data);
        filter.add_filter(Operation::Exclude(TestamentFilter::Old));
        assert_eq!(filter.ids().len(), 27);
    }
}
