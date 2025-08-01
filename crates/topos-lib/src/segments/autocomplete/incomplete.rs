use crate::segments::parse::{ALL_DASHES, NON_SEGMENT_CHARACTERS, TRAILING_NON_DIGITS};

pub struct IncompleteSegment {}

// fn sanitize_input(input: &str) -> String {
//     // swap weird hyphens with normal dash
//     let input = &input.replace(ALL_DASHES, "-");
//
//     // swap period with colon (to support 'Jn1.1')
//     let input = &input.replace(".", ":");
//
//     // input now only contains the following characters: [\d,:;-]
//     let input = NON_SEGMENT_CHARACTERS.replace_all(&input, "").to_string();
//
//     // removing trailing non-digits (leading shouldn't exist)
//     let input = TRAILING_NON_DIGITS.replace_all(&input, "").to_string();
//
//     input
// }

impl IncompleteSegment {
    pub fn parse(input: &str) -> Self {
        Self {}
    }
}
