use line_col::LineColLookup;
use mupdf::{Document, Rect};
use regex::Match;

use crate::matcher::{
    instance::BibleMatch,
    matcher::{BibleMatcher, Matcher},
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

    // TODO: Should I have a BibleLocater instead? Where the BibleMatcher deals only with plaintext
    // and the BibleLocater deals with file types?
    // Or perhaps a `FileMatcher` that takes a path...
    // **NO** because what if the user has an XML string or something?
    fn search<'a>(matcher: &BibleMatcher<Self>, input: Self::Input<'a>) -> Vec<BibleMatch<Self>> {
        let mut matches = vec![];

        for (idx, page) in doc.pages()?.enumerate() {
            let page = page?;
            let text = page.to_text()?;
            let results = matcher.search(&text);
            matches.extend(results.into_iter().map(|r| PDFBibleMatch {
                psg: r.psg,
                location: PDFLocation::Page(idx),
            }));
        }

        Ok(matches)
    }
}
