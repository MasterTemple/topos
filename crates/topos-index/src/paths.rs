//! Which files a query looks at, by path: files and folders, globs, and regular expressions

use globset::{GlobBuilder, GlobMatcher};
use regex::Regex;

/// A pattern that couldn't be read
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    #[error("bad glob \"{pattern}\": {message}")]
    Glob { pattern: String, message: String },
    #[error("bad regular expression \"{pattern}\": {message}")]
    Regex { pattern: String, message: String },
}

#[derive(Clone, Debug)]
enum Pattern {
    /// With `anywhere` (no `/` in it), it matches a file's or folder's name at any depth
    Glob {
        glob: GlobMatcher,
        anywhere: bool,
    },
    Regex(Regex),
}

impl Pattern {
    /// Whether a path matches. Globs follow gitignore's rules, like ripgrep's `--glob`: one
    /// without a `/` matches a name at any depth (`*.md`, `drafts`), others match from the root
    /// (`Sermons/2025`, `**/drafts`), `*` doesn't cross a `/`, and a glob matching a folder
    /// matches everything in it. A regular expression matches anywhere in the path
    fn matches(&self, path: &str) -> bool {
        match self {
            Pattern::Regex(regex) => regex.is_match(path),
            Pattern::Glob {
                glob,
                anywhere: true,
            } => path.split('/').any(|name| glob.is_match(name)),
            Pattern::Glob {
                glob,
                anywhere: false,
            } => {
                glob.is_match(path)
                    || path
                        .match_indices('/')
                        .any(|(i, _)| glob.is_match(&path[..i]))
            }
        }
    }
}

/**
Which files a query looks at, by path (paths use `/` and are relative to the index's root). A
file is kept when it's
- one of the files, or in one of the folders, given with [`PathFilter::path`] (if any are),
- matched by an included glob or regular expression (if any are),
- matched by no excluded one (`!` before a glob or regular expression excludes, like ripgrep's
  `--glob '!drafts'`; `\!` is a literal `!`),
- and has one of the extensions (if any are given).
*/
#[derive(Clone, Debug, Default)]
pub struct PathFilter {
    paths: Vec<String>,
    include: Vec<Pattern>,
    exclude: Vec<Pattern>,
    extensions: Vec<String>,
}

/// A pattern and whether `!` excludes with it (`\!` is a literal `!`)
fn negated(pattern: &str) -> (bool, String) {
    match pattern.strip_prefix('!') {
        Some(rest) => (true, rest.to_string()),
        None => (
            false,
            pattern
                .strip_prefix("\\!")
                .map_or_else(|| pattern.to_string(), |rest| format!("!{rest}")),
        ),
    }
}

impl PathFilter {
    /// Keeps every file
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether it keeps every file
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
            && self.include.is_empty()
            && self.exclude.is_empty()
            && self.extensions.is_empty()
    }

    /// A file, or a folder (its files and subfolders'); a trailing `/` is optional
    pub fn path(mut self, path: &str) -> Self {
        let path = path.trim_end_matches('/');
        self.paths.push(path.to_string());
        self
    }

    /// A glob (`!` excludes), like ripgrep's `--glob`; `ignore_case` like its `--iglob`
    pub fn glob(mut self, pattern: &str, ignore_case: bool) -> Result<Self, PathError> {
        let (exclude, glob) = negated(pattern);
        let error = |e: globset::Error| PathError::Glob {
            pattern: pattern.to_string(),
            message: e.kind().to_string(),
        };
        // A trailing `/` only says it's a folder, and a leading one anchors it at the root
        let trimmed = glob.trim_end_matches('/');
        let anywhere = !trimmed.contains('/');
        let matcher = GlobBuilder::new(trimmed.trim_start_matches('/'))
            .case_insensitive(ignore_case)
            .literal_separator(true)
            .build()
            .map_err(error)?
            .compile_matcher();
        self.add(
            exclude,
            Pattern::Glob {
                glob: matcher,
                anywhere,
            },
        );
        Ok(self)
    }

    /// A regular expression matched anywhere in the path (`!` excludes; `(?i)` ignores case)
    pub fn regex(mut self, pattern: &str) -> Result<Self, PathError> {
        let (exclude, source) = negated(pattern);
        let regex = Regex::new(&source).map_err(|e| PathError::Regex {
            pattern: pattern.to_string(),
            message: e.to_string(),
        })?;
        self.add(exclude, Pattern::Regex(regex));
        Ok(self)
    }

    /// An extension to keep, without the dot (any case)
    pub fn extension(mut self, extension: &str) -> Self {
        self.extensions
            .push(extension.trim_start_matches('.').to_lowercase());
        self
    }

    fn add(&mut self, exclude: bool, pattern: Pattern) {
        if exclude {
            self.exclude.push(pattern);
        } else {
            self.include.push(pattern);
        }
    }

    /// Whether the filter keeps a file
    pub fn keeps(&self, path: &str) -> bool {
        let under = |folder: &String| {
            folder.is_empty()
                || path == folder
                || path
                    .strip_prefix(folder.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        };
        (self.paths.is_empty() || self.paths.iter().any(under))
            && (self.include.is_empty() || self.include.iter().any(|p| p.matches(path)))
            && !self.exclude.iter().any(|p| p.matches(path))
            && (self.extensions.is_empty()
                || self.extensions.contains(&crate::index::extension(path)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kept(filter: &PathFilter) -> Vec<&'static str> {
        [
            "a.md",
            "Notes/b.md",
            "Notes/drafts/c.md",
            "Notes/d.TXT",
            "Sermons/2025/e.md",
            "Sermons/2025/f.srt",
            "Sermons/2024/g.md",
            "!odd.md",
        ]
        .into_iter()
        .filter(|path| filter.keeps(path))
        .collect()
    }

    #[test]
    fn files_and_folders() {
        assert_eq!(kept(&PathFilter::new()).len(), 8);
        let filter = PathFilter::new()
            .path("Notes/")
            .path("Sermons/2025/f.srt")
            .path("Sermons/20");
        assert_eq!(
            kept(&filter),
            [
                "Notes/b.md",
                "Notes/drafts/c.md",
                "Notes/d.TXT",
                "Sermons/2025/f.srt"
            ]
        );
    }

    #[test]
    fn globs_like_ripgrep() -> Result<(), PathError> {
        // A glob without a folder matches at any depth
        assert_eq!(
            kept(&PathFilter::new().glob("*.srt", false)?),
            ["Sermons/2025/f.srt"]
        );
        // A glob matching a folder keeps (or drops) all of it
        let drafts = PathFilter::new()
            .glob("Notes", false)?
            .glob("!**/drafts", false)?;
        assert_eq!(kept(&drafts), ["Notes/b.md", "Notes/d.TXT"]);
        // A name without a `/` matches at any depth; with one, from the root
        assert_eq!(kept(&PathFilter::new().glob("!drafts/", false)?).len(), 7);
        assert!(kept(&PathFilter::new().glob("/drafts", false)?).is_empty());
        // `*` stays within a folder; `**` crosses them
        assert!(kept(&PathFilter::new().glob("Sermons/*.md", false)?).is_empty());
        assert_eq!(
            kept(&PathFilter::new().glob("Sermons/**/*.md", false)?),
            ["Sermons/2025/e.md", "Sermons/2024/g.md"]
        );
        assert_eq!(
            kept(&PathFilter::new().glob("Sermons/202[4]/**", false)?),
            ["Sermons/2024/g.md"]
        );
        assert_eq!(
            kept(&PathFilter::new().glob("*.txt", false)?),
            Vec::<&str>::new()
        );
        assert_eq!(
            kept(&PathFilter::new().glob("*.txt", true)?),
            ["Notes/d.TXT"]
        );
        // Only exclusions: everything else
        assert_eq!(kept(&PathFilter::new().glob("!Sermons", false)?).len(), 5);
        assert_eq!(kept(&PathFilter::new().glob("\\!*", false)?), ["!odd.md"]);
        assert!(matches!(
            PathFilter::new().glob("a[", false),
            Err(PathError::Glob { .. })
        ));
        Ok(())
    }

    #[test]
    fn regular_expressions_and_extensions() -> Result<(), PathError> {
        assert_eq!(
            kept(&PathFilter::new().regex(r"20\d\d/[ef]")?),
            ["Sermons/2025/e.md", "Sermons/2025/f.srt"]
        );
        assert_eq!(
            kept(&PathFilter::new().regex(r"(?i)notes/d\.")?),
            ["Notes/d.TXT"]
        );
        let not_notes = PathFilter::new().regex("!^Notes/")?.extension(".MD");
        assert_eq!(
            kept(&not_notes),
            ["a.md", "Sermons/2025/e.md", "Sermons/2024/g.md", "!odd.md"]
        );
        assert!(matches!(
            PathFilter::new().regex("("),
            Err(PathError::Regex { .. })
        ));
        // Paths, patterns, and extensions all have to agree
        let all = PathFilter::new()
            .path("Sermons")
            .glob("!**/2024", false)?
            .extension("md");
        assert_eq!(kept(&all), ["Sermons/2025/e.md"]);
        Ok(())
    }
}
