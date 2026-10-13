use std::collections::BTreeSet;

use crate::{
    data::{bible_data::BibleData, books::BookId},
    matcher::instance::BibleMatch,
    segments::passage::Passage,
};

/// Decides which matches to keep, after every book has been matched
#[derive(Clone, Debug, Default)]
pub struct ComplexFilter {
    /// [`None`] keeps every book
    books: Option<BTreeSet<BookId>>,
    /// Keep matches entirely inside one of these
    inside_of: Vec<Passage>,
    /// Keep matches that share any verse with one of these
    any_overlap: Vec<Passage>,
    /// Keep matches that name a verse of one of these: whole chapters in a match don't count
    /// (`John 3` doesn't overlap `John 3:16`, but `John 3:14-18` does)
    explicit_overlap: Vec<Passage>,
    /// Keep matches that are exactly one of these (the same verses, however they are written)
    exact: Vec<Passage>,
    /// Drop matches that share any verse with one of these
    exclude_overlap: Vec<Passage>,
    /// Keep matches that name a verse: [`None`] keeps whole chapters too, and `Some(false)` drops
    /// single-chapter books (`Jude 25`)
    verses: Option<bool>,
    /// Keep matches followed by one of these translations (see [`ComplexFilter::translation`])
    translations: Vec<String>,
}

impl ComplexFilter {
    /// Only keep matches in these books
    pub fn books(&mut self, books: BTreeSet<BookId>) {
        self.books = Some(books);
    }

    pub fn inside(&mut self, psg: Passage) {
        self.inside_of.push(psg);
    }

    pub fn any_overlap(&mut self, psg: Passage) {
        self.any_overlap.push(psg);
    }

    pub fn explicit_overlap(&mut self, psg: Passage) {
        self.explicit_overlap.push(psg);
    }

    pub fn exact_overlap(&mut self, psg: Passage) {
        self.exact.push(psg);
    }

    pub fn exclude_overlap(&mut self, psg: Passage) {
        self.exclude_overlap.push(psg);
    }

    /**
    Only keep matches that name a verse: whole chapters don't count (`John 3`, `John 2-4`), but a
    match with a verse anywhere does (`John 2; 3:16`)
    - A single-chapter book's numbers are verses (`Jude 25` is `Jude 1:25`), so its matches always
      name one; `single_chapter_books: false` drops them (`Jude 25`, `Jude 1:25`, `Obadiah 3`)
    */
    pub fn verses_only(&mut self, single_chapter_books: bool) {
        self.verses = Some(single_chapter_books);
    }

    /**
    Only keep matches followed by this translation (`John 3:16 ESV`, `John 3:16 (ESV)`); several
    keep matches followed by any of them
    - [`ComplexFilter::keep`] only sees passages, so this is checked by
      [`BibleMatcher`](crate::matcher::BibleMatcher)'s searches
    */
    pub fn translation(&mut self, name: impl Into<String>) {
        self.translations.push(name.into());
    }

    /// The translations a match must be followed by (any of them), if any were given
    pub fn translations(&self) -> &[String] {
        &self.translations
    }

    /// The passages that include matches (inside, any, explicit, and exact)
    fn inclusions(&self) -> impl Iterator<Item = &Passage> {
        (self.inside_of.iter())
            .chain(&self.any_overlap)
            .chain(&self.explicit_overlap)
            .chain(&self.exact)
    }

    /**
    - The passage filters that include matches are joined with a logical OR: a match is kept if
      it is inside an `inside` passage, overlaps an `any_overlap` one, names a verse of an
      `explicit_overlap` one, or is exactly an `exact` one
    - Then a match that shares any verse with an `exclude_overlap` passage is dropped
    - With [`ComplexFilter::verses_only`], a match that names no verse is dropped
    - Translations ([`ComplexFilter::translation`]) aren't checked here
    */
    pub fn keep(&self, psg: &Passage, data: &BibleData) -> bool {
        if self.books.as_ref().is_some_and(|b| !b.contains(&psg.book)) {
            return false;
        }
        let versification = data.chapter_verses().get_chapter_verses(&psg.book);
        if let Some(single_chapter_books) = self.verses {
            let single_chapter = versification.is_some_and(|v| v.has_one_chapter());
            if psg.explicit_verses().is_none() || (single_chapter && !single_chapter_books) {
                return false;
            }
        }
        let overlaps = |other: &Passage| other.overlaps_passage(psg, versification);
        let included = self.inclusions().next().is_none()
            || self
                .inside_of
                .iter()
                .any(|outer| outer.contains_passage(psg, versification))
            || self.any_overlap.iter().any(overlaps)
            || (!self.explicit_overlap.is_empty()
                && psg.explicit_verses().is_some_and(|named| {
                    (self.explicit_overlap.iter())
                        .any(|other| other.overlaps_passage(&named, versification))
                }))
            || self.exact.iter().any(|other| {
                other.contains_passage(psg, versification)
                    && psg.contains_passage(other, versification)
            });
        included && !self.exclude_overlap.iter().any(overlaps)
    }

    /// The only books a kept match can be in, or [`None`] for any book: the allowed books,
    /// narrowed to the books of the passages that include matches, when there are any
    pub fn possible_books(&self) -> Option<BTreeSet<BookId>> {
        let mut books = self.books.clone();
        if self.inclusions().next().is_some() {
            let passages: BTreeSet<BookId> = self.inclusions().map(|psg| psg.book).collect();
            books = Some(match books {
                Some(books) => books.intersection(&passages).copied().collect(),
                None => passages,
            });
        }
        books
    }

    pub fn as_filter<'a>(&'a self, data: &'a BibleData) -> FilteredBibleMatches<'a> {
        FilteredBibleMatches::new(self, data)
    }
}

pub struct FilteredBibleMatches<'a> {
    filter: &'a ComplexFilter,
    data: &'a BibleData,
    matches: Vec<BibleMatch>,
}

impl<'a> FilteredBibleMatches<'a> {
    pub fn new(filter: &'a ComplexFilter, data: &'a BibleData) -> Self {
        Self {
            filter,
            data,
            matches: vec![],
        }
    }

    pub fn try_add(&mut self, m: BibleMatch) {
        if self.filter.keep(&m.psg, self.data) {
            self.matches.push(m);
        }
    }

    pub fn matches(self) -> Vec<BibleMatch> {
        self.matches
    }
}
