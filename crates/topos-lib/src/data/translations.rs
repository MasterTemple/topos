/*!
Translations named after a reference (`John 3:16 ESV`, `John 3:16 (KJV)`), read only by the
methods that take [`Translations`]:
[`BibleData::parse_with_translation`](crate::data::BibleData::parse_with_translation),
[`BibleMatcher::search_with_translations`](crate::matcher::BibleMatcher::search_with_translations),
[`BibleMatcher::problems_with_translations`](crate::matcher::BibleMatcher::problems_with_translations),
and [`BibleMatcher::complete_with_translations`](crate::matcher::BibleMatcher::complete_with_translations).

Only the names the caller lists are translations, so a word after a reference is never read as
one by accident, and nothing else ([`BibleData::parse`](crate::data::BibleData::parse),
[`BibleMatcher::search`](crate::matcher::BibleMatcher::search)) looks for them.

A translation can have its own chapter and verse counts ([`Translations::set_chapter_verses`]),
which references naming it are checked against: `3 John 1:15 ESV` exists with the ESV's counts,
though the default counts (the KJV's) end at verse 14. Books without them use the data's counts.
*/

use std::{collections::BTreeMap, ops::Range};

use crate::{
    data::{books::BookId, chapter_verses::ChapterVerses},
    segments::{Passage, grammar::SegmentList},
};

/**
The translations a reference can name: each name and alias is matched ignoring case, and a match
gives the name it was listed with

```
use topos_bible::data::translations::Translations;

let translations = Translations::new(["ESV", "KJV", "New King James Version"]).alias("AV", "KJV");
assert_eq!(translations.get("esv"), Some("ESV"));
assert_eq!(translations.get("av"), Some("KJV"));
assert_eq!(translations.get("NIV"), None);
```
*/
#[derive(Clone, Debug, Default)]
pub struct Translations {
    /// The names, in the order they were listed
    names: Vec<String>,
    /// The lowercased name or alias, and the name it gives, longest first
    keys: Vec<(String, String)>,
    /// Each translation's own chapter and verse counts, by name
    chapter_verses: BTreeMap<String, BTreeMap<BookId, ChapterVerses>>,
}

impl Translations {
    pub fn new<S: Into<String>>(names: impl IntoIterator<Item = S>) -> Self {
        names
            .into_iter()
            .fold(Self::default(), |translations, name| {
                let name = name.into();
                translations.alias(name.clone(), name)
            })
    }

    /// Another way to write `name` (`AV` for `KJV`); a name or alias listed again is replaced
    pub fn alias(mut self, alias: impl Into<String>, name: impl Into<String>) -> Self {
        let key = alias.into().trim().to_lowercase();
        if key.is_empty() {
            return self;
        }
        let name = name.into();
        if !self.names.contains(&name) {
            self.names.push(name.clone());
        }
        self.keys.retain(|(k, _)| *k != key);
        self.keys.push((key, name));
        self.keys
            .sort_by_key(|(k, _)| std::cmp::Reverse(k.chars().count()));
        self
    }

    /// The name that `name` or an alias of it was listed with
    pub fn get(&self, name: &str) -> Option<&str> {
        let key = name.trim().to_lowercase();
        self.keys
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, name)| name.as_str())
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// The names, in the order they were listed
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }

    /// The names that `typed` starts (or one of their aliases), ignoring case, in listed order
    pub fn starting_with(&self, typed: &str) -> Vec<&str> {
        let typed = typed.to_lowercase();
        self.names()
            .filter(|name| {
                self.keys
                    .iter()
                    .any(|(key, n)| n == name && key.starts_with(&typed))
            })
            .collect()
    }

    /// A book's chapter and verse counts in a translation (the number of verses in each chapter)
    pub fn set_chapter_verses(&mut self, name: &str, book: BookId, verses: ChapterVerses) {
        let name = self.get(name).unwrap_or(name).to_string();
        (self.chapter_verses.entry(name).or_default()).insert(book, verses);
    }

    /// [`Translations::set_chapter_verses`] for building
    pub fn with_chapter_verses(mut self, name: &str, book: BookId, verses: ChapterVerses) -> Self {
        self.set_chapter_verses(name, book, verses);
        self
    }

    /// A book's chapter and verse counts in a translation, if it has its own
    pub fn chapter_verses(&self, name: &str, book: BookId) -> Option<&ChapterVerses> {
        let name = self.get(name).unwrap_or(name);
        self.chapter_verses.get(name)?.get(&book)
    }

    /**
    The segments of a reference in `text[..end]` (`text` starts after its book name, and `end` is
    where the next book name starts), and the translation named right after them, with its name
    and byte range in `text`
    - The segments end where the translation starts, so a name is never read as part of the
      reference (`LXX` would be the Roman numeral 70)
    - The translation can be past `end`, since a name can start like a book (`ESV` like `Es`)
    */
    pub(crate) fn read_segments<'t>(
        &'t self,
        text: &str,
        end: usize,
    ) -> (SegmentList, Option<(&'t str, Range<usize>)>) {
        let window = &text[..end];
        let named = self.find(window).and_then(|(name, range)| {
            let list = SegmentList::parse(&window[..range.start]);
            let (_, following) = self.following(&window[list.end()..])?;
            (list.end() + following.start == range.start).then_some((list, (name, range)))
        });
        if let Some((list, named)) = named {
            return (list, Some(named));
        }
        let list = SegmentList::parse(window);
        let after = list.end();
        let named = (self.following(&text[after..]))
            .map(|(name, range)| (name, after + range.start..after + range.end));
        (list, named)
    }

    /// The longest name or alias at the start of `text`, followed by a word boundary: its name
    /// and length in bytes
    fn at_start(&self, text: &str) -> Option<(&str, usize)> {
        self.keys.iter().find_map(|(key, name)| {
            let mut ends = text.char_indices().map(|(i, _)| i).chain([text.len()]);
            let len = ends.nth(key.chars().count())?;
            let next = text[len..].chars().next();
            (text[..len].to_lowercase() == *key && !next.is_some_and(char::is_alphanumeric))
                .then_some((name.as_str(), len))
        })
    }

    /// The first translation in `text` that starts a word: its name and byte range
    fn find(&self, text: &str) -> Option<(&str, Range<usize>)> {
        let mut previous = None;
        text.char_indices().find_map(|(i, c)| {
            let starts_word = !previous.is_some_and(char::is_alphanumeric);
            previous = Some(c);
            starts_word
                .then(|| self.at_start(&text[i..]))
                .flatten()
                .map(|(name, len)| (name, i..i + len))
        })
    }

    /**
    The translation right after a reference, as `text` continues from its end: ` ESV`, `, ESV`,
    ` - ESV`, ` (ESV)`, or ` [ESV]`; its name and byte range in `text`
    */
    pub(crate) fn following(&self, text: &str) -> Option<(&str, Range<usize>)> {
        let start = text.len() - text.trim_start_matches(Self::is_gap).len();
        let (name, len) = self.at_start(&text[start..])?;
        let range = start..start + len;
        let closer = match text[..start].trim_end().chars().last() {
            Some('(') => Some(')'),
            Some('[') => Some(']'),
            _ => None,
        };
        match closer {
            Some(closer) if !text[range.end..].trim_start().starts_with(closer) => None,
            _ => Some((name, range)),
        }
    }

    /// What can come between a reference and its translation, on the same line
    pub(crate) fn is_gap(c: char) -> bool {
        (c.is_whitespace() && c != '\n' && c != '\r')
            || matches!(c, ',' | ';' | ':' | '(' | '[' | '-' | '–' | '—' | '/')
    }
}

/// A passage and the translation named after it (see [`Translations`])
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslatedPassage {
    pub passage: Passage,
    pub translation: Option<Translation>,
}

/// A translation named in the input
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Translation {
    /// The name the translation was listed with (`KJV` for `kjv` or an alias like `AV`)
    pub name: String,
    /// Where it is in the input, in bytes
    pub span: Range<usize>,
}

impl Translation {
    /// `name` and `range` (from [`Translations`]), with the range moved by `offset`
    pub(crate) fn at(name: &str, range: Range<usize>, offset: usize) -> Self {
        Self {
            name: name.to_string(),
            span: offset + range.start..offset + range.end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_name_first_and_whole_words() {
        let t = Translations::new(["NASB", "NASB95", "New King James Version", "NKJV"]);
        assert_eq!(t.at_start("NASB95 says"), Some(("NASB95", 6)));
        assert_eq!(t.at_start("nasb, and"), Some(("NASB", 4)));
        assert_eq!(t.at_start("NASB2020"), None);
        assert_eq!(
            t.at_start("new king james version"),
            Some(("New King James Version", 22))
        );
        assert_eq!(t.find("3:16 xNKJV NKJV"), Some(("NKJV", 11..15)));
    }

    #[test]
    fn names_and_counts() {
        let mut t = Translations::new(["KJV", "ESV"]).alias("AV", "KJV");
        assert_eq!(t.names().collect::<Vec<_>>(), ["KJV", "ESV"]);
        assert_eq!(t.starting_with("a"), ["KJV"]);
        assert_eq!(t.starting_with("E"), ["ESV"]);
        assert_eq!(t.starting_with(""), ["KJV", "ESV"]);
        t.set_chapter_verses("av", BookId(64), ChapterVerses::new(vec![14]));
        let counts = t.chapter_verses("KJV", BookId(64)).unwrap();
        assert_eq!(counts.verse_counts(), [14]);
        assert!(t.chapter_verses("ESV", BookId(64)).is_none());
    }

    #[test]
    fn following_a_reference() {
        let t = Translations::new(["ESV"]);
        assert_eq!(t.following(" ESV"), Some(("ESV", 1..4)));
        assert_eq!(t.following(" (esv)"), Some(("ESV", 2..5)));
        assert_eq!(t.following(" [ESV] says"), Some(("ESV", 2..5)));
        assert_eq!(t.following(" — ESV"), Some(("ESV", 5..8)));
        // An opened bracket must close
        assert_eq!(t.following(" (ESV"), None);
        assert_eq!(t.following(" says the ESV"), None);
        assert_eq!(t.following(""), None);
    }
}
