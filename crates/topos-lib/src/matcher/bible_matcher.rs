use std::{borrow::Cow, collections::BTreeSet};

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
    segments::{Passage, resolve::SegmentSpan},
};

#[derive(Clone, Debug)]
pub struct BibleMatcher {
    data: BibleData,
    /// Every book is always matched (so `1 John` is never mistaken for `John`), then filtered here
    complex_filter: ComplexFilter,
    /// When set, references without a book name (`1:1-5`) are matched too
    context: Option<BookContext>,
    /// Whether a book named alone (`Romans`) is found, as the whole book
    books_alone: bool,
}

impl BibleMatcher {
    pub fn new(data: BibleData, complex_filter: ComplexFilter) -> Self {
        Self {
            data,
            complex_filter,
            context: None,
            books_alone: false,
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

    /**
    Also find books named alone, without a chapter (`Romans`, `1 Cor.`), each as the whole book
    (`Romans 1-16`; see [`BibleData::whole_book`]); off by default
    - In running text many names are also words (`Mark`, `Job`, `Acts`), so this is for text that
      is meant to be references, like a search box's `Romans and Jude, Titus`
    - Abbreviations that are also common words (`is`, `am`) are never found alone, and a name
      followed by a number is a reference, or nothing (`John 3:99`)

    ```
    use topos_bible::matcher::BibleMatcher;

    let matcher = BibleMatcher::default().with_books_alone(true);
    let books = matcher.data().books();
    let found: Vec<_> = matcher
        .search("Romans and Jude, Titus 2:11")
        .into_iter()
        .map(|m| format!("{} {}", books.get_name(m.psg.book).unwrap(), m.psg.segments))
        .collect();
    assert_eq!(found, ["Romans 1-16", "Jude 1", "Titus 2:11"]);
    assert!(BibleMatcher::default().search("Romans and Jude").is_empty());
    ```
    */
    pub fn with_books_alone(mut self, books_alone: bool) -> Self {
        self.books_alone = books_alone;
        self
    }

    /// Whether books named alone are found (see [`Self::with_books_alone`])
    pub fn books_alone(&self) -> bool {
        self.books_alone
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
            books_alone: self.books_alone,
        }
    }

    /// Whether this matcher's filters keep a passage (except its translations: see
    /// [`Self::filters_translations`])
    pub fn keeps(&self, psg: &Passage) -> bool {
        self.complex_filter.keep(psg, &self.data)
    }

    /**
    Whether matches must be followed by a translation
    ([`BibleFilter::filter_translation`]), which [`Self::keeps`] can't check: then filtering
    [`Self::without_filters`]'s results isn't the same as searching, so search the text again
    */
    pub fn filters_translations(&self) -> bool {
        !self.complex_filter.translations().is_empty()
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
    - With a [`BookContext`], references without a book name are found too, and with
      [`Self::with_books_alone`], books named alone
    - Matches are then filtered by book, passage, and translation (a translation filter reads the
      names it was given after each reference, as [`Self::search_with_translations`] does)
    */
    pub fn search(&self, text: &str) -> Vec<BibleMatch> {
        self.search_translated(text, None, false, |reference, _, _| reference)
    }

    /**
    [`BibleMatcher::search`], with where each segment of each match is written in `text`, one per
    segment of its passage (`John 3:16` and `4:5-7` in `John 3:16; 4:5-7`, the first with the
    book's name), and where its range's end is (the `7`)
    - Slower than [`BibleMatcher::search`], so it is for one line or note at a time, when the parts
      are needed (tapping a part of a reference opens that part)
    */
    pub fn search_with_parts(&self, text: &str) -> Vec<(BibleMatch, Vec<SegmentSpan>)> {
        self.search_translated(text, None, true, |reference, _, parts| (reference, parts))
    }

    /**
    [`BibleMatcher::search`], with the translation each reference names right after it
    (`John 3:16 ESV`, `John 3:16 (ESV)`), if it is one of `translations`
    - A reference naming a translation is checked against that translation's chapter and verse
      counts, when it has them (see [`Translations::set_chapter_verses`])
    - A name ends the reference, so `Ps 22, LXX` is Psalm 22 (not Psalms 22 and 70)
    - References without a book name (with a [`BookContext`]) get the translation after them, but
      are checked against the data's counts
    - A translation filter's names are looked up in `translations` (so an alias names the same
      translation), and those it doesn't list are read too
    */
    pub fn search_with_translations(
        &self,
        text: &str,
        translations: &Translations,
    ) -> Vec<TranslatedMatch> {
        self.search_translated(
            text,
            Some(translations),
            false,
            |reference, translation, _| TranslatedMatch {
                reference,
                translation,
            },
        )
    }

    fn search_translated<T>(
        &self,
        text: &str,
        translations: Option<&Translations>,
        with_parts: bool,
        to_match: impl Fn(BibleMatch, Option<Translation>, Vec<SegmentSpan>) -> T,
    ) -> Vec<T> {
        let original = text;
        let text = SearchText::new(original);
        let index = LineIndex::new(original);
        let data = self.data();
        let translations = self.translations_to_read(translations);
        let translations = translations.as_deref();

        let starts: Vec<_> = data.books().candidates(text.as_str()).collect();
        let mut found: Vec<(FoundPassage, Option<Translation>)> = starts
            .iter()
            .enumerate()
            .filter_map(|(idx, cur)| {
                let next_start = starts.get(idx + 1).map(|next| next.start());
                FoundPassage::find_translated(
                    data,
                    text.as_str(),
                    *cur,
                    next_start,
                    translations,
                    with_parts,
                )
            })
            .collect();

        if let Some(context) = self.context() {
            let taken: Vec<_> = found.iter().map(|(f, _)| f.bytes.clone()).collect();
            let book_starts: Vec<_> = starts.iter().map(|s| s.start()).collect();
            let bare = context.find_bare(data, text.as_str(), &taken, &book_starts, with_parts);
            found.extend(bare.into_iter().map(|f| {
                let after = &text.as_str()[f.bytes.end..];
                let translation = translations
                    .and_then(|t| t.following(after))
                    .map(|(name, range)| Translation::at(name, range, f.bytes.end));
                (f, translation)
            }));
            found.sort_by_key(|(f, _)| f.bytes.start);
        }

        if self.books_alone {
            let taken: Vec<_> = found.iter().map(|(f, _)| f.bytes.clone()).collect();
            let overlaps = |bytes: &std::ops::Range<usize>| {
                taken
                    .iter()
                    .any(|t| t.start < bytes.end && bytes.start < t.end)
            };
            let alone = (data.books().book_regex().find_iter(text.as_str()))
                .filter_map(|name| FoundPassage::book_alone(data, text.as_str(), name, with_parts))
                .filter(|f| !overlaps(&f.bytes));
            found.extend(alone.map(|f| (f, None)).collect::<Vec<_>>());
            found.sort_by_key(|(f, _)| f.bytes.start);
        }

        found
            .into_iter()
            .filter(|(found, translation)| {
                self.keeps(&found.psg) && self.keeps_translation(translation.as_ref(), translations)
            })
            .map(|(found, translation)| {
                let bytes = text.original_range(found.bytes);
                let location = LineColLocation::new(&index, bytes.start, bytes.end);
                let translation = translation.map(|t| Translation {
                    span: text.original_range(t.span),
                    ..t
                });
                let parts = (found.parts.into_iter())
                    .map(|part| SegmentSpan {
                        written: text.original_range(part.written),
                        end: part.end.map(|end| text.original(end)),
                    })
                    .collect();
                let reference = BibleMatch {
                    location,
                    psg: found.psg,
                };
                to_match(reference, translation, parts)
            })
            .collect()
    }

    /// The translations to read after references: the caller's, plus the translation filter's
    /// names it doesn't list
    fn translations_to_read<'t>(
        &self,
        translations: Option<&'t Translations>,
    ) -> Option<Cow<'t, Translations>> {
        let names = self.complex_filter.translations();
        let Some(translations) = translations else {
            return (!names.is_empty()).then(|| Cow::Owned(Translations::new(names)));
        };
        let missing: Vec<_> = (names.iter())
            .filter(|name| translations.get(name).is_none())
            .collect();
        Some(match missing.is_empty() {
            true => Cow::Borrowed(translations),
            false => Cow::Owned(
                (missing.into_iter()).fold(translations.clone(), |t, name| t.alias(name, name)),
            ),
        })
    }

    /// Whether the translation after a match is one the translation filter keeps
    fn keeps_translation(
        &self,
        translation: Option<&Translation>,
        translations: Option<&Translations>,
    ) -> bool {
        let names = self.complex_filter.translations();
        if names.is_empty() {
            return true;
        }
        let (Some(translation), Some(translations)) = (translation, translations) else {
            return false;
        };
        names
            .iter()
            .any(|name| translations.get(name) == Some(translation.name.as_str()))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn found(matcher: &BibleMatcher, text: &str) -> Vec<String> {
        let books = matcher.data().books();
        (matcher.search(text).into_iter())
            .map(|m| {
                let name = books.get_name(m.psg.book).cloned().unwrap_or_default();
                let written = &text[m.location.bytes.start..m.location.bytes.end];
                format!("{name} {} ({written})", m.psg.segments)
            })
            .collect()
    }

    #[test]
    fn books_alone() {
        let matcher = BibleMatcher::default().with_books_alone(true);
        assert_eq!(
            found(
                &matcher,
                "Romans and Jude, Titus. Also 1 Cor. and 1 John 2:1"
            ),
            [
                "Romans 1-16 (Romans)",
                "Jude 1 (Jude)",
                "Titus 1-3 (Titus)",
                "1 Corinthians 1-16 (1 Cor)",
                "1 John 2:1 (1 John 2:1)"
            ]
        );
        // Abbreviations that are words, names followed by a number, and odd casing
        assert!(found(&matcher, "it is so; John 3:99; mK").is_empty());
        // Off by default, and kept by a matcher without filters
        assert!(found(&BibleMatcher::default(), "Romans").is_empty());
        assert!(matcher.without_filters().books_alone());
        // Filters apply to them as to any reference
        let mut filter = BibleFilter::default();
        filter.filter_verses(true);
        let verses = filter.create_matcher().with_books_alone(true);
        assert_eq!(found(&verses, "Romans, Jude 5"), ["Jude 1:5 (Jude 5)"]);
        // Each one's part is its name
        let parts = matcher.search_with_parts("Read Romans.");
        assert_eq!(parts[0].1[0].written, 5..11);
    }
}
