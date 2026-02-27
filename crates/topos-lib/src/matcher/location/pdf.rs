use line_col::LineColLookup;
use mupdf::{Document, Rect, TextPageOptions, pdf::PdfDocument};
use regex::Match;

use crate::matcher::{
    instance::BibleMatch,
    location::line_col::LineColLocation,
    matcher::{BibleMatcher, MatchResult, Matcher},
};

/**
- This is in the normal PDF coordinate space (origin is at the bottom left) used by PDF.js
- MuPDF coordinate space is at the top left: https://mupdf.readthedocs.io/en/latest/reference/common/coordinate-system.html
*/
// TODO: Do I want to store page bounds?
#[derive(Clone, Debug)]
pub struct PDFRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl PDFRect {
    pub fn from_rects(page: Rect, line: Rect) -> Self {
        // let x = line.x0;
        // let y = page.y1 - line.y1;
        // let w = line.x1;
        // let h = page.y1 - line.y0;
        let x = line.x0;
        let y = page.y1 - line.y1;
        let w = line.x1 - line.x0;
        let h = line.y1 - line.y0;
        Self { x, y, w, h }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PDFMatchError {
    #[error("Failed to read pages")]
    ReadPages,
    #[error("Failed to read Page {0}")]
    ReadPage(usize),
    #[error("Failed to read text on Page {0}")]
    ReadText(usize),
    #[error("Failed to page bounds for Page {0}")]
    PageBounds(usize),
}

#[derive(Clone, Debug)]
pub enum PDFLocation {
    Page(usize),
    // The Obsidian PDF++ selection (using PDF.js backend) is unstable
    // Selection,
    Rectangle { page: usize, rect: PDFRect },
    Rectangles { page: usize, rect: Vec<PDFRect> },
    Search { page: usize, query: String },
}

// impl Matcher for PDFLocation {
//     type Input<'a> = &'a Document;
//
//     fn search<'a>(
//         matcher: &BibleMatcher,
//         doc: Self::Input<'a>,
//     ) -> MatchResult<Vec<BibleMatch<Self>>> {
//         let mut matches = vec![];
//
//         for (idx, page) in doc
//             .pages()
//             .map_err(|_| PDFMatchError::ReadPages)?
//             .enumerate()
//         {
//             let page = page.map_err(|_| PDFMatchError::ReadPage(idx))?;
//
//             // If I just want page number
//             let page_number = true;
//             if page_number {
//                 let text = page.to_text().map_err(|_| PDFMatchError::ReadText(idx))?;
//
//                 let results = matcher.search::<LineColLocation>(&text)?;
//
//                 matches.extend(
//                     results
//                         .into_iter()
//                         .map(|m| m.map_loc(|_| PDFLocation::Page(idx))),
//                 );
//             }
//             // TODO: Even if I want the rect, I should still just do all matches on the big text block, but then find their rects (otherwise I have to find matches across rects, which is significantly harder)
//             else {
//                 let page_bounds = page.bounds().map_err(|_| PDFMatchError::PageBounds(idx))?;
//                 page.to_text_page(TextPageOptions::into_iter)
//                 let text_page = page
//                     .to_text_page(TextPageOptions::BLOCK_TEXT)
//                     .map_err(|_| PDFMatchError::ReadText(idx))?;
//
//                 for block in text_page.blocks() {
//                     // To understand bounds/rect: https://pymupdf.readthedocs.io/en/latest/rect.html
//                     for line in block.lines() {
//                         let line_text = line.chars().filter_map(|c| c.char()).collect::<String>();
//                         // TODO: Match it somehow
//                         if line_text.contains("Hebrews") {
//                             let rect = PDFRect::from_rects(page_bounds, line.bounds());
//                             let loc = PDFLocation::Rectangle { page: idx, rect };
//                             // TODO: finish
//                             // matches.push(value);
//                         }
//                     }
//                 }
//             }
//         }
//
//         Ok(matches)
//     }
// }

// impl Matcher for PDFLocation {
//     type Input<'a> = &'a Document;
//
//     fn search<'a>(
//         matcher: &BibleMatcher,
//         doc: Self::Input<'a>,
//     ) -> MatchResult<Vec<BibleMatch<Self>>> {
//         let mut matches = vec![];
//
//         for (idx, page) in doc
//             .pages()
//             .map_err(|_| PDFMatchError::ReadPages)?
//             .enumerate()
//         {
//             let page = page.map_err(|_| PDFMatchError::ReadPage(idx))?;
//
//             // If I just want page number
//             let text = page.to_text().map_err(|_| PDFMatchError::ReadText(idx))?;
//
//             let text_page = page
//                 .to_text_page(TextPageOptions::BLOCK_TEXT)
//                 .map_err(|_| PDFMatchError::ReadText(idx))?;
//
//             // for block in text_page.blocks() {
//             //     // To understand bounds/rect: https://pymupdf.readthedocs.io/en/latest/rect.html
//             //     for line in block.lines() {
//             //         let c = line.chars().next().unwrap();
//             //         c.quad().
//             //         let line_text = line.chars().filter_map(|c| c.char()).collect::<String>();
//             //         // TODO: Match it somehow
//             //         if line_text.contains("Hebrews") {
//             //             let rect = PDFRect::from_rects(page_bounds, line.bounds());
//             //             let loc = PDFLocation::Rectangle { page: idx, rect };
//             //             // TODO: finish
//             //             // matches.push(value);
//             //         }
//             //     }
//             // }
//
//             let results = matcher.search::<LineColLocation>(&text)?;
//
//             let page_bounds = page.bounds().map_err(|_| PDFMatchError::PageBounds(idx))?;
//
//             matches.extend(
//                 results
//                     .into_iter()
//                     // .map(|m| m.map_loc(|_| PDFLocation::Page(idx))),
//                     .map(|m| convert_match(idx, page_bounds, chars, lines, m)),
//             );
//         }
//
//         Ok(matches)
//     }
// }

struct PageChar {
    line: usize,
    column: usize,
    rect: Rect, // MuPDF rect or quad
}
struct LineIndex {
    line: usize,
    start_idx: usize,
    end_idx: usize,
}
fn merge_rects(rects: &[Rect]) -> Rect {
    let mut x0 = f32::MAX;
    let mut y0 = f32::MAX;
    let mut x1 = f32::MIN;
    let mut y1 = f32::MIN;

    for r in rects {
        x0 = x0.min(r.x0);
        y0 = y0.min(r.y0);
        x1 = x1.max(r.x1);
        y1 = y1.max(r.y1);
    }

    Rect { x0, y0, x1, y1 }
}
// fn convert_match(
//     page_idx: usize,
//     page_bounds: Rect,
//     chars: &[PageChar],
//     lines: &[LineIndex],
//     m: BibleMatch<LineColLocation>,
// ) -> BibleMatch<PDFLocation> {
//     let start = m.location.start;
//     let end = m.location.end;
//
//     let mut line_rects = vec![];
//
//     for line in start.line..=end.line {
//         let line_info = &lines[line];
//
//         let start_col = if line == start.line { start.column } else { 0 };
//
//         let end_col = if line == end.line {
//             end.column
//         } else {
//             line_info.end_idx - line_info.start_idx
//         };
//
//         let start_idx = line_info.start_idx + start_col;
//         let end_idx = line_info.start_idx + end_col;
//
//         let rect = merge_rects(
//             &chars[start_idx..end_idx]
//                 .iter()
//                 .map(|c| c.rect)
//                 .collect::<Vec<_>>(),
//         );
//
//         line_rects.push(PDFRect::from_rects(page_bounds, rect));
//     }
//
//     let loc = if line_rects.len() == 1 {
//         PDFLocation::Rectangle {
//             page: page_idx,
//             rect: line_rects.remove(0),
//         }
//     } else {
//         PDFLocation::Rectangles {
//             page: page_idx,
//             rect: line_rects,
//         }
//     };
//
//     m.map_loc(|_| loc)
// }

use unicode_normalization::UnicodeNormalization;

pub fn search_pdf_page(
    matcher: &BibleMatcher,
    page_idx: usize,
    page: &mupdf::Page,
) -> MatchResult<Vec<BibleMatch<PDFLocation>>> {
    // ------------------------------------------------------------
    // 1. Extract normalized plain text for matching
    // ------------------------------------------------------------

    let raw_text = page
        .to_text()
        .map_err(|_| PDFMatchError::ReadText(page_idx))?;

    // Normalize to NFC to match glyph iteration
    let normalized_text: String = raw_text.nfc().collect();

    let mut matches: Vec<BibleMatch<LineColLocation>> =
        matcher.search::<LineColLocation>(&normalized_text)?;

    if matches.is_empty() {
        return Ok(vec![]);
    }

    // Matches must be sorted
    matches.sort_by_key(|m| (m.location.start.line, m.location.start.column));

    // ------------------------------------------------------------
    // 2. Extract structured text (glyphs with geometry)
    // ------------------------------------------------------------

    let text_page = page
        // TODO: Should I preserve ligatures? What are they
        .to_text_page(TextPageOptions::PRESERVE_WHITESPACE)
        .map_err(|_| PDFMatchError::ReadText(page_idx))?;

    let page_bounds = page
        .bounds()
        .map_err(|_| PDFMatchError::PageBounds(page_idx))?;
    println!("{:#?}", page_bounds);

    let mut results = Vec::with_capacity(matches.len());

    let mut current_match_idx = 0;
    let mut current_match = &matches[current_match_idx];
    let mut current_match_str =
        &normalized_text[current_match.location.bytes.start..=current_match.location.bytes.end];
    let mut current_match_char_idx = 0;
    // let mut current_match_str = normalized_text.lines().nth(current_match.location.start.line)
    //     .map(|l| l.chars().skip(current_match.location.start.column).tak.collect())
    // ;
    // let mut current_match_str = "2 Tim. 3:2";
    dbg!(current_match_str);

    let mut current_line: usize = 0;
    let mut current_column: usize = 0;

    // Accumulator for multi-line rects
    let mut line_rects: Vec<PDFRect> = Vec::new();
    let mut current_line_char_rects: Vec<mupdf::Rect> = Vec::new();

    // ------------------------------------------------------------
    // 3. Single-pass glyph iteration
    // ------------------------------------------------------------

    for block in text_page.blocks() {
        for line in block.lines() {
            current_column = 0;

            for ch in line.chars() {
                if current_match_idx >= matches.len() {
                    break;
                }

                let pos_line = current_line;
                let pos_col = current_column;

                let start = current_match.location.start;
                let end = current_match.location.end;

                // ----------------------------------------------------
                // Start of match
                // ----------------------------------------------------
                if pos_line == start.line && pos_col == start.column {
                    current_line_char_rects.clear();
                    line_rects.clear();
                }

                // ----------------------------------------------------
                // If inside active match, accumulate glyph rect
                // ----------------------------------------------------
                let in_match = (pos_line > start.line
                    || (pos_line == start.line && pos_col >= start.column))
                    && (pos_line < end.line || (pos_line == end.line && pos_col < end.column));

                if in_match {
                    let q = ch.quad();
                    // if current_match_str
                    dbg!(ch.char());
                    // assert_eq!(q.ll, ch.origin());
                    // current_line_char_rects.push(ch.quad());
                    current_line_char_rects.push(mupdf::Rect {
                        x0: q.ll.x,
                        y0: q.ll.y,
                        x1: q.ur.x,
                        y1: q.ur.y,
                    });
                }

                // ----------------------------------------------------
                // End of match
                // ----------------------------------------------------
                if pos_line == end.line && pos_col == end.column {
                    if !current_line_char_rects.is_empty() {
                        let merged = merge_rects(&current_line_char_rects);

                        line_rects.push(PDFRect::from_rects(page_bounds, merged));
                    }

                    let location = if line_rects.len() == 1 {
                        PDFLocation::Rectangle {
                            page: page_idx,
                            rect: line_rects.remove(0),
                        }
                    } else {
                        PDFLocation::Rectangles {
                            page: page_idx,
                            rect: line_rects.clone(),
                        }
                    };

                    results.push(current_match.clone().map_loc(|_| location));

                    // Advance to next match
                    current_match_idx += 1;
                    if current_match_idx >= matches.len() {
                        break;
                    }

                    current_match = &matches[current_match_idx];
                    current_line_char_rects.clear();
                    line_rects.clear();
                }

                current_column += 1;
            }

            // If match spans lines, finalize this line’s rect
            if !current_line_char_rects.is_empty() {
                let merged = merge_rects(&current_line_char_rects);
                line_rects.push(PDFRect::from_rects(page_bounds, merged));
                current_line_char_rects.clear();
            }

            current_line += 1;
        }
    }

    for result in &results {
        match &result.location {
            PDFLocation::Page(_) => todo!(),
            PDFLocation::Rectangles { page, rect } => todo!(),
            PDFLocation::Search { page, query } => todo!(),
            PDFLocation::Rectangle { page, rect } => {
                println!(
                    "![[The Dorean Principle - by Conley Owens.pdf#page={}&rect={},{},{},{}&color=yellow|The Dorean Principle - by Conley Owens, p.iii]]",
                    page,
                    rect.x,
                    rect.y,
                    rect.x + rect.w,
                    rect.y + rect.h,
                )
            }
        }
    }

    Ok(results)
}

impl Matcher for PDFLocation {
    type Input<'a> = &'a Document;

    fn search<'a>(
        matcher: &BibleMatcher,
        doc: Self::Input<'a>,
    ) -> MatchResult<Vec<BibleMatch<Self>>> {
        let mut matches = vec![];

        for (idx, page) in doc
            .pages()
            .map_err(|_| PDFMatchError::ReadPages)?
            .enumerate()
        {
            let page_num = idx + 1;
            let page = page.map_err(|_| PDFMatchError::ReadPage(page_num))?;
            // TODO: remove this later, it is just for testing
            if matches.len() > 0 {
                break;
            }
            matches.extend(search_pdf_page(matcher, page_num, &page)?);
        }

        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::AnyResult;

    use super::*;

    #[test]
    fn tdp_pdf() -> AnyResult<()> {
        let path = "/home/dgmastertemple/Dropbox/Apps/remotely-save/Dropbox Library/Books/PDF/The Dorean Principle - by Conley Owens.pdf";
        let doc = Document::open(path)?;
        let m = BibleMatcher::default();
        let result = m.search::<PDFLocation>(&doc)?;

        // println!("{:#?}", &result);
        // println!("{:#?}", &result[0]);

        Ok(())
    }
}
