use std::path::PathBuf;

use topos_lib::matcher::{instance::BibleMatch, matcher::BibleMatcher};

pub struct PathMatches {
    pub path: Option<PathBuf>,
    pub matches: Vec<BibleMatch>,
}
impl PathMatches {
    pub fn new(matches: Vec<BibleMatch>) -> Self {
        Self {
            path: None,
            matches,
        }
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    pub fn from_text(text: String, matcher: &BibleMatcher) -> PathMatches {
        let matches = matcher.search(&text);
        PathMatches::new(matches)
    }

    pub fn from_file(path: PathBuf, matcher: &BibleMatcher) -> PathMatches {
        let text = std::fs::read_to_string(&path).unwrap();
        Self::from_text(text, matcher).with_path(path)
    }
}
