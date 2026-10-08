//! References written in Bible markup (OSIS, ThML, GBF, TEI, USX), located in its tree.

use std::ops::Range;

use topos_bible::matcher::{BibleMatch, BibleMatcher};
use topos_bible_markup::{Flat, Point};

use crate::{Format, FormatError};

/**
Where a match is in parsed markup
- Search a [`Flat`] made with the [`Filter`](topos_bible_markup::Filter) you want searched (notes
  or not, titles or not); see [`Flattener`](topos_bible_markup::Flattener)
- A match can start and end in different elements (`See <hi>John</hi> 3:16`)
- Matches made only of added text (note brackets, Strong's numbers) are left out

```
use topos_bible::matcher::BibleMatcher;
use topos_bible_formats::{SearchFormat, markup::MarkupLocation};
use topos_bible_markup::{Markup, TextOptions, flatten::flatten, parse};

let nodes = parse(Markup::Osis, "See <hi type=\"italic\">John</hi> 3:16.");
let flat = flatten(&nodes, &TextOptions::default());
let found = BibleMatcher::default().search_format::<MarkupLocation>(&flat).unwrap();
let location = &found[0].location;
assert_eq!(&flat.text[location.range.clone()], "John 3:16");
assert_eq!(location.start.path.resolve(&nodes).unwrap().as_text(), Some("John"));
assert_eq!((location.end.path.to_string(), location.end.offset), ("/2".to_string(), 5));
```
*/
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkupLocation {
    /// Where the match starts: a text node and a byte offset in it
    pub start: Point,
    /// Where the match ends (exclusive)
    pub end: Point,
    /// The match's byte range in [`Flat::text`]
    pub range: Range<usize>,
}

impl Format for MarkupLocation {
    type Input<'a> = &'a Flat;

    fn search(
        matcher: &BibleMatcher,
        input: Self::Input<'_>,
    ) -> Result<Vec<BibleMatch<Self>>, FormatError> {
        Ok(matcher
            .search(&input.text)
            .into_iter()
            .filter_map(|m| {
                let range = m.location.bytes.start..m.location.bytes.end;
                let (start, end) = input.source_range(range.clone())?;
                Some(m.map_loc(|_| MarkupLocation { start, end, range }))
            })
            .collect())
    }
}
