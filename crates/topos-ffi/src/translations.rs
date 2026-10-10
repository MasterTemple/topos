//! Translations a reference can name (`John 3:16 KJV`), for `Topos::parse_with_translation`,
//! `Topos::search_with_translations`, and `Topos::complete_with_translations`.

use boltffi::*;
use topos_lib::data::{
    books::BookId,
    chapter_verses::ChapterVerses,
    translations::{Translation as CoreTranslation, Translations},
};

use crate::{Match, Offsets, Passage};

/// A translation named after a reference, with its offsets in the searched or parsed text
#[data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Translation {
    /// The name it was added with (`KJV` for `kjv`, or for an alias like `AV`)
    pub name: String,
    pub start: u32,
    pub end: u32,
}

impl Translation {
    pub(crate) fn new(translation: CoreTranslation, offsets: &Offsets<'_>) -> Self {
        Self {
            start: offsets.of_byte(translation.span.start),
            end: offsets.of_byte(translation.span.end),
            name: translation.name,
        }
    }
}

/// A passage and the translation named after it (see `Topos::parse_with_translation`)
#[data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslatedPassage {
    pub passage: Passage,
    pub translation: Option<Translation>,
}

/// A reference and the translation named after it (see `Topos::search_with_translations`)
#[data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslatedMatch {
    pub reference: Match,
    pub translation: Option<Translation>,
}

/**
The translations references can name, like the Bibles a plugin has. Chain the methods (each
returns new translations)

```ts
const translations = ToposTranslations.create()
  .add("ESV", [])
  .add("KJV", ["AV"])
  .chapterVerses("ESV", 64, [15]); // 3 John's verse counts in the ESV
topos.parseWithTranslation("3 John 15 (esv)", OffsetUnit.Utf16, translations, BookStyle.Name);
// { passage: 3 John 1:15, translation: { name: "ESV", start: 11, end: 14 } }
```

- Names and aliases are matched ignoring case; only these are translations, so other words after
  a reference never are
- A reference naming a translation with verse counts is checked against them instead of the
  default counts (the KJV's), for the books it has them for
*/
#[derive(Clone, Debug, Default)]
pub struct ToposTranslations {
    pub(crate) translations: Translations,
}

/// For Rust crates that build on this one, like logoxide's, which adds its packages' verse counts
impl From<Translations> for ToposTranslations {
    fn from(translations: Translations) -> Self {
        Self { translations }
    }
}

impl ToposTranslations {
    /// The translations themselves, for Rust crates that build on this one
    pub fn core(&self) -> &Translations {
        &self.translations
    }
}

#[export]
impl ToposTranslations {
    /// No translations
    pub fn create() -> Self {
        Self::default()
    }

    /// A translation and other ways to write it (`KJV`, `["AV"]`); adding a name again adds its
    /// aliases
    pub fn add(&self, name: String, aliases: Vec<String>) -> Self {
        let translations = aliases.into_iter().fold(
            self.translations.clone().alias(name.clone(), name.clone()),
            |translations, alias| translations.alias(alias, name.clone()),
        );
        Self { translations }
    }

    /// A book's verse counts in a translation, one per chapter (a book's own verse counts)
    pub fn chapter_verses(&self, name: String, book_id: u8, counts: Vec<u8>) -> Self {
        let translations = self.translations.clone().with_chapter_verses(
            &name,
            BookId(book_id),
            ChapterVerses::new(counts),
        );
        Self { translations }
    }

    /// The names, in the order they were added
    pub fn names(&self) -> Vec<String> {
        self.translations.names().map(String::from).collect()
    }

    /// The name that `name` or an alias of it was added with, ignoring case
    pub fn get(&self, name: String) -> Option<String> {
        self.translations.get(&name).map(String::from)
    }
}
