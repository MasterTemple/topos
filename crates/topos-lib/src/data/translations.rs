/*!
Translations named after a reference (`John 3:16 ESV`, `John 3:16 (KJV)`), for
[`BibleData::parse_with_translation`](crate::data::bible_data::BibleData::parse_with_translation).

Only the names the caller lists are translations, so a word after a reference is never read as
one by accident, and nothing else (search, [`BibleData::parse`](crate::data::BibleData::parse))
looks for them.
*/

use std::ops::Range;

use crate::segments::Passage;

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
    /// The lowercased name or alias, and the name it gives, longest first
    keys: Vec<(String, String)>,
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
        self.keys.retain(|(k, _)| *k != key);
        self.keys.push((key, name.into()));
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
    pub(crate) fn find(&self, text: &str) -> Option<(&str, Range<usize>)> {
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

    /// What can come between a reference and its translation
    fn is_gap(c: char) -> bool {
        c.is_whitespace() || matches!(c, ',' | ';' | ':' | '(' | '[' | '-' | '–' | '—' | '/')
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
