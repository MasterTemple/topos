use std::collections::BTreeSet;

use crate::{
    data::{
        bible_data::BibleData,
        books::BookId,
        translations::{Translation, Translations},
    },
    filter::bible_filter::BibleFilter,
    matcher::{
        context::BookContext,
        instance::{BibleMatch, FoundPassage},
        line_col::{LineColLocation, LineIndex},
        matches::{ComplexFilter, FilteredBibleMatches},
        text::SearchText,
    },
    segments::Passage,
};

#[derive(Clone, Debug)]
pub struct BibleMatcher {
    data: BibleData,
    /// Every book is always matched (so `1 John` is never mistaken for `John`), then filtered here
    complex_filter: ComplexFilter,
    /// When set, references without a book name (`1:1-5`) are matched too
    context: Option<BookContext>,
}

impl BibleMatcher {
    pub fn new(data: BibleData, complex_filter: ComplexFilter) -> Self {
        Self {
            data,
            complex_filter,
            context: None,
        }
    }

    /// Also match references without a book name, using this context for their book
    pub fn with_context(mut self, context: BookContext) -> Self {
        self.context = Some(context);
        self
    }

    pub fn context(&self) -> Option<&BookContext> {
        self.context.as_ref()
    }

    pub fn data(&self) -> &BibleData {
        &self.data
    }

    pub fn filter(&self) -> FilteredBibleMatches<'_> {
        self.complex_filter.as_filter(&self.data)
    }

    /**
    The same matcher without its book and passage filters (keeping the data and book context)
    - Filters only drop matches after they are found, so filtering this matcher's results with
      [`Self::keeps`] gives exactly what this matcher finds. The CLI caches unfiltered results
      this way and filters them per search
    */
    pub fn without_filters(&self) -> Self {
        Self {
            data: self.data.clone(),
            complex_filter: ComplexFilter::default(),
            context: self.context.clone(),
        }
    }

    /// Whether this matcher's filters keep a passage
    pub fn keeps(&self, psg: &Passage) -> bool {
        self.complex_filter.keep(psg, &self.data)
    }

    /// The only books a kept match can be in ([`None`] for any book), to skip text that
    /// mentions none of them
    pub fn possible_books(&self) -> Option<BTreeSet<BookId>> {
        self.complex_filter.possible_books()
    }
}

impl BibleMatcher {
    /**
    Finds every reference in `text`
    - Every book name followed by a chapter is a candidate, and its segments end where the next
      candidate starts (so `John 1:1, 3 John 5` is `John 1:1` and `3 John 5`)
    - With a [`BookContext`], references without a book name are found too
    - Matches are then filtered by book and passage
    */
    pub fn search(&self, text: &str) -> Vec<BibleMatch> {
        self.search_translated(text, None, |reference, _| reference)
    }

    /**
    [`BibleMatcher::search`], with the translation each reference names right after it
    (`John 3:16 ESV`, `John 3:16 (ESV)`), if it is one of `translations`
    - A reference naming a translation is checked against that translation's chapter and verse
      counts, when it has them (see [`Translations::set_chapter_verses`])
    - A name ends the reference, so `Ps 22, LXX` is Psalm 22 (not Psalms 22 and 70)
    - References without a book name (with a [`BookContext`]) get the translation after them, but
      are checked against the data's counts
    */
    pub fn search_with_translations(
        &self,
        text: &str,
        translations: &Translations,
    ) -> Vec<TranslatedMatch> {
        self.search_translated(text, Some(translations), |reference, translation| {
            TranslatedMatch {
                reference,
                translation,
            }
        })
    }

    fn search_translated<T>(
        &self,
        text: &str,
        translations: Option<&Translations>,
        to_match: impl Fn(BibleMatch, Option<Translation>) -> T,
    ) -> Vec<T> {
        let original = text;
        let text = SearchText::new(original);
        let index = LineIndex::new(original);
        let data = self.data();

        let starts: Vec<_> = data.books().candidates(text.as_str()).collect();
        let mut found: Vec<(FoundPassage, Option<Translation>)> = starts
            .iter()
            .enumerate()
            .filter_map(|(idx, cur)| {
                let next_start = starts.get(idx + 1).map(|next| next.start());
                FoundPassage::find_translated(data, text.as_str(), *cur, next_start, translations)
            })
            .collect();

        if let Some(context) = self.context() {
            let taken: Vec<_> = found.iter().map(|(f, _)| f.bytes.clone()).collect();
            let book_starts: Vec<_> = starts.iter().map(|s| s.start()).collect();
            let bare = context.find_bare(data, text.as_str(), &taken, &book_starts);
            found.extend(bare.into_iter().map(|f| {
                let after = &text.as_str()[f.bytes.end..];
                let translation = translations
                    .and_then(|t| t.following(after))
                    .map(|(name, range)| Translation::at(name, range, f.bytes.end));
                (f, translation)
            }));
            found.sort_by_key(|(f, _)| f.bytes.start);
        }

        found
            .into_iter()
            .filter(|(found, _)| self.keeps(&found.psg))
            .map(|(found, translation)| {
                let bytes = text.original_range(found.bytes);
                let location = LineColLocation::new(&index, bytes.start, bytes.end);
                let translation = translation.map(|t| Translation {
                    span: text.original_range(t.span),
                    ..t
                });
                let reference = BibleMatch {
                    location,
                    psg: found.psg,
                };
                to_match(reference, translation)
            })
            .collect()
    }

    /// The first reference in `text`
    pub fn find(&self, text: &str) -> Option<BibleMatch> {
        self.search(text).into_iter().next()
    }
}

/// A reference found by [`BibleMatcher::search_with_translations`], and the translation it names
#[derive(Clone, Debug)]
pub struct TranslatedMatch {
    pub reference: BibleMatch,
    pub translation: Option<Translation>,
}

impl Default for BibleMatcher {
    fn default() -> Self {
        BibleFilter::default().create_matcher()
    }
}
