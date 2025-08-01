#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SegmentJoiner {
    /// Joined by characters like `-`
    /// - There should not be 2 of these in a row, must be followed by `,` or `:` and then `,`
    Range,
    /// Joined by characters like `,` or `;`
    /// - There can be as many of these in a row as you would like
    Separate,
    /// Joined by characters like `:`
    /// - There should not be 2 of these in a row
    Chapter,
}
