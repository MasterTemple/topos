//! Compares every golden file in `tests/golden/` with what the crate gives now
//! (see `docs/spec/golden-format.md`).

mod common;

use std::path::{Path, PathBuf};

use common::golden;
use topos_bible_markup::Markup;

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("tests/golden exists")
        .map(|e| e.expect("a directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|e| e == "golden") {
            out.push(path);
        }
    }
}

/// The first differing line of two section contents, for the failure message
fn first_difference(expected: &str, actual: &str) -> String {
    let (e, a): (Vec<&str>, Vec<&str>) =
        (expected.split('\n').collect(), actual.split('\n').collect());
    for i in 0..e.len().max(a.len()) {
        let (x, y) = (e.get(i), a.get(i));
        if x != y {
            return format!(
                "line {}:\n    expected: {}\n    actual:   {}",
                i + 1,
                x.map_or("<none>".to_string(), |s| golden::escape(s)),
                y.map_or("<none>".to_string(), |s| golden::escape(s)),
            );
        }
    }
    String::from("(no line differs)")
}

#[test]
fn golden_files() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut paths = Vec::new();
    files(&root, &mut paths);
    assert!(paths.len() > 100, "the golden corpus is missing");
    // `GOLDEN_FILTER=osis/` runs only the files whose path contains it
    let only = std::env::var("GOLDEN_FILTER").ok();
    let mut failures = Vec::new();
    let mut checked = 0;
    for path in &paths {
        let name = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();
        if only.as_deref().is_some_and(|f| !name.contains(f)) {
            continue;
        }
        checked += 1;
        let text = std::fs::read_to_string(path).expect("a UTF-8 golden file");
        let sections = golden::read(&text);
        let markup_name = &sections
            .iter()
            .find(|(n, _)| n == "markup")
            .expect("a markup section")
            .1;
        let markup = Markup::from_name(markup_name).expect("a known markup");
        let inputs: Vec<String> = sections
            .iter()
            .filter(|(n, _)| n == "input")
            .map(|(_, c)| c.clone())
            .collect();
        let expected: Vec<&(String, String)> = sections
            .iter()
            .filter(|(n, _)| n != "markup" && n != "input")
            .collect();
        let actual = match std::panic::catch_unwind(|| golden::outputs(markup, &inputs)) {
            Ok(actual) => actual,
            Err(_) => {
                failures.push(format!("{name}: panicked"));
                continue;
            }
        };
        if expected.len() != actual.len() {
            failures.push(format!(
                "{name}: {} sections expected, {} computed",
                expected.len(),
                actual.len()
            ));
            continue;
        }
        let mut counts = std::collections::HashMap::<&str, usize>::new();
        for ((en, ec), (an, ac)) in expected
            .iter()
            .map(|s| (&s.0, &s.1))
            .zip(actual.iter().map(|s| (&s.0, &s.1)))
        {
            let n = counts.entry(en.as_str()).or_default();
            *n += 1;
            if en != an {
                failures.push(format!(
                    "{name}: section {en} expected where {an} was computed"
                ));
                break;
            }
            if ec != ac {
                failures.push(format!(
                    "{name}: section `{en}` #{n} differs at {}",
                    first_difference(ec, ac)
                ));
            }
        }
    }
    assert!(checked > 0, "no golden file matched GOLDEN_FILTER");
    if !failures.is_empty() {
        let shown: Vec<&String> = failures.iter().take(40).collect();
        panic!(
            "{} of the golden sections differ ({} files checked):\n{}",
            failures.len(),
            checked,
            shown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
