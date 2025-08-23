// use topos_parser::spanned_length::{SpanLen, VerboseDelimeter, VerboseNumber, VerboseSpace};
//
// pub enum FormattableToken {
//     Delimeter(VerboseDelimeter),
//     Number(VerboseNumber),
//     Space(VerboseSpace),
// }
//
// impl SpanLen for FormattableToken {
//     fn span_len(&self) -> usize {
//         match self {
//             FormattableToken::Delimeter(t) => t.span_len(),
//             FormattableToken::Number(t) => t.span_len(),
//             FormattableToken::Space(t) => t.span_len(),
//         }
//     }
// }
//
// impl FormattableToken {
//     pub fn get_contents<'a>(&'_ self, s: &'a str, start: usize) -> &'a str {
//         let span = self.as_span(start);
//         &s[span.start..span.end]
//     }
// }
//
// // impl FormattableToken {
// //     pub fn format(
// //         self,
// //         input: &str,
// //         cx: &mut FullFormatContext,
// //         options: &FormatOptions,
// //     ) -> String {
// //         let actual = self.get_contents(input, cx.start);
// //         cx.start += self.span_len();
// //         match self {
// //             FormattableToken::Delimeter(token) => {
// //                 match token.parsed {
// //                     Delimeter::Segment => {
// //                         cx.is_after_range = false;
// //                         match &options.delim {
// //                             DelimeterOptions::DontTouch => token.actual.to_string(),
// //                             // BUG: I need to know upcoming / current segment type, so I can
// //                             // specify ',' or ';'
// //                             DelimeterOptions::Normalize => ",".to_string(),
// //                             DelimeterOptions::NormalizeWith { verse_segment, .. } => {
// //                                 verse_segment.clone().unwrap_or(String::from(","))
// //                             }
// //                         }
// //                     }
// //                     Delimeter::Chapter => match &options.delim {
// //                         DelimeterOptions::DontTouch => token.actual.to_string(),
// //                         DelimeterOptions::Normalize => todo!(),
// //                         DelimeterOptions::NormalizeWith { chapter, range, .. } => todo!(),
// //                     },
// //                     Delimeter::Range => {
// //                         cx.is_after_range = true;
// //                         todo!()
// //                     }
// //                 }
// //             }
// //             FormattableToken::Number(token) => todo!(),
// //             FormattableToken::Space(token) => match options.general_spacing {
// //                 SpaceOptions::DontTouch => actual.to_string(),
// //                 SpaceOptions::RemoveAll => String::new(),
// //                 SpaceOptions::Normalize => String::from(" "),
// //             },
// //         }
// //     }
// // }
