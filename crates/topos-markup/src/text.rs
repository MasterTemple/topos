//! Plain text, for terminals and editors.

use crate::{Node, filter::Filter, flatten::flatten};

/// Options for [`plain_text`] and [`Flattener`](crate::Flattener)
#[derive(Debug, Clone)]
pub struct TextOptions {
    /**
    Which elements are included; the default, [`Filter::text`], is the text to read (no notes,
    titles, or metadata)
    - Kept notes are in square brackets: `light [John 1:5]`
    - Kept titles are on their own line
    */
    pub filter: Filter,
    /// Append Strong's numbers after tagged words, like `beginning [H7225]`; default: false
    pub strongs: bool,
    /// Keep line and paragraph breaks as `\n`; otherwise they become spaces; default: true
    pub line_breaks: bool,
}

impl Default for TextOptions {
    fn default() -> Self {
        TextOptions {
            filter: Filter::text(),
            strongs: false,
            line_breaks: true,
        }
    }
}

/// Plain text, with runs of whitespace collapsed (see [`Flat`](crate::Flat))
pub fn plain_text(nodes: &[Node], opts: &TextOptions) -> String {
    flatten(nodes, opts).text
}
