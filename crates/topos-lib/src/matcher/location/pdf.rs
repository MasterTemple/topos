use line_col::LineColLookup;
use mupdf::{Document, Rect, TextPageOptions, pdf::PdfDocument};
use regex::Match;

use crate::matcher::{
    instance::BibleMatch,
    location::line_col::LineColLocation,
    matcher::{BibleMatcher, MatchResult, Matcher},
};

/**
- This is in the normal PDF coordinate space
- MuPDF coordinate space: https://mupdf.readthedocs.io/en/latest/reference/common/coordinate-system.html
*/
#[derive(Clone, Debug)]
pub struct PDFRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl PDFRect {
    pub fn from_rects(page: Rect, line: Rect) -> Self {
        let x = line.x0;
        let y = page.y1 - line.y1;
        let w = line.x1;
        let h = page.y1 - line.y0;
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
            let page = page.map_err(|_| PDFMatchError::ReadPage(idx))?;

            // If I just want page number
            let page_number = true;
            if page_number {
                let text = page.to_text().map_err(|_| PDFMatchError::ReadText(idx))?;

                let results = matcher.search::<LineColLocation>(&text)?;

                matches.extend(
                    results
                        .into_iter()
                        .map(|m| m.map_loc(|_| PDFLocation::Page(idx))),
                );
            }
            // TODO: Even if I want the rect, I should still just do all matches on the big text block, but then find their rects (otherwise I have to find matches across rects, which is significantly harder)
            else {
                let page_bounds = page.bounds().map_err(|_| PDFMatchError::PageBounds(idx))?;
                let text_page = page
                    .to_text_page(TextPageOptions::BLOCK_TEXT)
                    .map_err(|_| PDFMatchError::ReadText(idx))?;

                for block in text_page.blocks() {
                    // To understand bounds/rect: https://pymupdf.readthedocs.io/en/latest/rect.html
                    for line in block.lines() {
                        let line_text = line.chars().filter_map(|c| c.char()).collect::<String>();
                        // TODO: Match it somehow
                        if line_text.contains("Hebrews") {
                            let rect = PDFRect::from_rects(page_bounds, line.bounds());
                            let loc = PDFLocation::Rectangle { page: idx, rect };
                            // TODO: finish
                            // matches.push(value);
                        }
                    }
                }
            }
        }

        Ok(matches)
    }
}
