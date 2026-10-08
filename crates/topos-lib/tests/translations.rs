//! The shared translation cases (`cases/translations.txt`), against the library

use topos_bible::{
    data::{BibleData, chapter_verses::ChapterVerses, translations::Translations},
    matcher::BibleMatcher,
    segments::{Passage, autocomplete::CompleteOptions},
};

/// Reads `translations:` and `counts:` lines, and returns the other cases as `(kind, input,
/// expected)` with the translations in effect for each
fn cases(data: &BibleData) -> Vec<(Translations, String, String, Vec<String>)> {
    let mut translations = Translations::default();
    let mut cases = vec![];
    for line in include_str!("cases/translations.txt").lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let (kind, rest) = line.split_once(": ").expect("a case has a kind");
        match kind {
            "translations" => {
                translations = Translations::default();
                for names in rest.split(", ") {
                    let mut names = names.split('=');
                    let name = names.next().expect("a name");
                    translations = translations.alias(name, name);
                    for alias in names {
                        translations = translations.alias(alias, name);
                    }
                }
            }
            "counts" => {
                let parts: Vec<&str> = rest.split(" | ").collect();
                let book = data.books().search(parts[1]).expect("a book");
                let counts = parts[2].split(' ').map(|n| n.parse().unwrap()).collect();
                translations.set_chapter_verses(parts[0], book, ChapterVerses::new(counts));
            }
            _ => {
                let (input, expected) = rest.split_once(" =>").expect("a case has `=>`");
                let expected = expected
                    .split(" | ")
                    .map(|e| e.trim().to_string())
                    .filter(|e| !e.is_empty())
                    .collect();
                let input = input.replace("\\n", "\n");
                cases.push((translations.clone(), kind.to_string(), input, expected));
            }
        }
    }
    cases
}

fn written(data: &BibleData, passage: &Passage, translation: Option<&str>) -> String {
    let book = data.books().get_name(passage.book).unwrap();
    match translation {
        Some(translation) => format!("{book} {} [{translation}]", passage.segments),
        None => format!("{book} {}", passage.segments),
    }
}

#[test]
fn translation_cases() {
    let matcher = BibleMatcher::default();
    let data = matcher.data();
    let mut failures = vec![];
    for (translations, kind, input, expected) in cases(data) {
        let actual: Vec<String> = match kind.as_str() {
            "parse" => data
                .parse_with_translation(&input, &translations)
                .map(|found| {
                    let name = found.translation.as_ref().map(|t| t.name.as_str());
                    written(data, &found.passage, name)
                })
                .into_iter()
                .collect(),
            "search" => matcher
                .search_with_translations(&input, &translations)
                .into_iter()
                .map(|found| {
                    let name = found.translation.as_ref().map(|t| t.name.as_str());
                    if let Some(t) = &found.translation {
                        assert!(
                            input[t.span.clone()].eq_ignore_ascii_case(&t.name)
                                || translations.get(&input[t.span.clone()]) == Some(&t.name),
                            "{input:?}: the span of {}",
                            t.name
                        );
                    }
                    written(data, &found.reference.psg, name)
                })
                .collect(),
            "problem" => matcher
                .problems_with_translations(&input, &translations)
                .into_iter()
                .map(|p| p.message)
                .collect(),
            "complete" => matcher
                .complete_with_translations(
                    &input,
                    input.len(),
                    &CompleteOptions::default(),
                    &translations,
                )
                .into_iter()
                .map(|c| c.label)
                .collect(),
            _ => panic!("unknown case kind {kind:?}"),
        };
        // Completions only need to start with the expected ones
        let ok = match kind.as_str() {
            "complete" if !expected.is_empty() => actual.starts_with(&expected),
            _ => actual == expected,
        };
        if !ok {
            failures.push(format!(
                "{kind}: {input:?}\n    expected {expected:?}\n    actual   {:?}",
                &actual[..actual.len().min(5)]
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// Without translations, the translated methods find what the plain ones do
#[test]
fn no_translations_is_plain() {
    let matcher = BibleMatcher::default();
    let none = Translations::default();
    for line in include_str!("cases/search.txt").lines() {
        let Some((input, _)) = line.split_once(" =>") else {
            continue;
        };
        if line.starts_with('#') {
            continue;
        }
        let input = input.trim_start_matches('?').replace("\\n", "\n");
        let plain: Vec<_> = matcher.search(&input).into_iter().map(|m| m.psg).collect();
        let translated: Vec<_> = matcher
            .search_with_translations(&input, &none)
            .into_iter()
            .map(|m| m.reference.psg)
            .collect();
        assert_eq!(plain, translated, "{input:?}");
        assert_eq!(
            matcher.data().parse(&input),
            matcher
                .data()
                .parse_with_translation(&input, &none)
                .map(|t| t.passage),
            "{input:?}"
        );
    }
}
