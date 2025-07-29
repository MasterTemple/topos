use std::collections::BTreeSet;

use crate::{data::books::BookId, filter::filter::IsFilter};

pub struct BookFilter {
    input: String,
}

impl BookFilter {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
        }
    }
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

#[cfg(test)]
mod tests {
    use crate::{
        data::data::BibleData,
        filter::{
            filter::{BibleFilter, Operation},
            filters::{book::BookFilter, genre::GenreFilter, testament::TestamentFilter},
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
        pentateuch_without_genesis,
        [
            Operation::Include(GenreFilter::new("pentateuch")),
            Operation::Exclude(BookFilter::new("genesis")),
        ],
        4
    );

    mk_test!(
        pentateuch_and_revelation,
        [
            Operation::Include(GenreFilter::new("pentateuch")),
            Operation::Include(BookFilter::new("rev")),
        ],
        6
    );

    mk_test!(
        no_genesis,
        [Operation::Exclude(BookFilter::new("genesis")),],
        65
    );

    mk_test!(
        no_genesis_or_john,
        [
            Operation::Exclude(BookFilter::new("genesis")),
            Operation::Exclude(BookFilter::new("john")),
        ],
        64
    );
}
