use crate::htmloc::{FragmentEngine, GenerateOptions, TextFragment};
use itertools::Itertools;

use topos_bible::matcher::{BibleMatch, BibleMatcher, LineColLocation, LineIndex};

use crate::{Format, FormatError};

#[derive(thiserror::Error, Debug)]
pub enum HTMLMatchError {
    #[error("Failed to generate Text Fragment")]
    FailedToGenerate,
}

/**
Where a match is in an HTML document
- Search runs on the text a browser shows: tags, `<script>`, and `<style>` are left out,
  character references are decoded, and block elements are on their own lines
*/
#[derive(Clone, Debug)]
pub struct HTMLLocation {
    /// The match's lines, columns, and bytes in the HTML source
    pub line_col: LineColLocation,
    /// A text fragment (`#:~:text=`) that a browser highlights
    pub text_fragment: TextFragment,
}

impl Format for HTMLLocation {
    type Input<'a> = &'a str;

    fn search(
        matcher: &BibleMatcher,
        input: Self::Input<'_>,
    ) -> Result<Vec<BibleMatch<Self>>, FormatError> {
        let doc = FragmentEngine::from_html(input);
        let source = LineIndex::new(input);

        let results = matcher.search(doc.plain_text());

        results
            .into_iter()
            .map(|m| {
                let bytes = m.location.bytes;
                let text_fragment = doc
                    .generate_for_plain_text(
                        bytes.start,
                        bytes.end,
                        Some(GenerateOptions::default()),
                    )
                    .ok_or(HTMLMatchError::FailedToGenerate)?;
                // Where the match is in the HTML (between tags, not in the extracted text)
                let start = doc.doc().source_byte(bytes.start);
                let end = doc.doc().source_end_byte(bytes.end);
                let (Some(start), Some(end)) = (start, end) else {
                    return Err(HTMLMatchError::FailedToGenerate.into());
                };
                Ok(m.map_loc(|_| HTMLLocation {
                    text_fragment,
                    line_col: LineColLocation::new(&source, start, end),
                }))
            })
            .try_collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::htmloc::FragmentEngine;

    fn fragments(html: &str) -> Vec<String> {
        HTMLLocation::search(&BibleMatcher::default(), html)
            .unwrap()
            .iter()
            .map(|m| m.location.text_fragment.to_hash_string())
            .collect()
    }

    #[test]
    fn tags_before_a_match() {
        // Positions used to be read as HTML positions, which failed or moved the fragment
        let html = r#"<div class="topos"><span class="topos-w" data-strong="G3756">Not</span> as <i>John</i> 3:16 says</div>"#;
        assert_eq!(fragments(html), ["#:~:text=as-,John%203%3A16,-says"]);
        let found = HTMLLocation::search(&BibleMatcher::default(), html).unwrap();
        let bytes = found[0].location.line_col.bytes;
        assert_eq!(&html[bytes.start..bytes.end], "John</i> 3:16");
    }

    #[test]
    fn matches_near_the_end_need_context() {
        // This used to loop forever: no text after the match, and the prefix wasn't unique
        let html = "<p>John 3:16 and John 3:16</p>";
        assert_eq!(fragments(html).len(), 2);
    }

    #[test]
    fn text_as_a_browser_shows_it() {
        let html = "<h1>Faith &amp; Love</h1><p>See&nbsp;Rom 5:8&#8212;</p><script>let x = 'John 3:16';</script><style>p{}</style><div>Gal 2:20</div>";
        let engine = FragmentEngine::from_html(html);
        assert_eq!(
            engine.plain_text(),
            "Faith & Love\nSee\u{a0}Rom 5:8\u{2014}\nGal 2:20\n"
        );
        let found = HTMLLocation::search(&BibleMatcher::default(), html).unwrap();
        let written: Vec<&str> = found
            .iter()
            .map(|m| &html[m.location.line_col.bytes.start..m.location.line_col.bytes.end])
            .collect();
        assert_eq!(written, ["Rom 5:8", "Gal 2:20"]);
        // Each fragment finds its match again
        for m in &found {
            let selection = engine.resolve_fragment(&m.location.text_fragment).unwrap();
            assert_eq!(
                selection.start,
                crate::htmloc::Position::new(
                    m.location.line_col.start.line,
                    m.location.line_col.start.column
                )
            );
        }
    }
}
