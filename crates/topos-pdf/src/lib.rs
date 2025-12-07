use mupdf::Document;
use topos_lib::{
    error::AnyResult,
    matcher::{matcher::BibleMatcher, matches::ComplexFilter},
    segments::segments::Passage,
};

#[derive(Clone, Debug)]
pub enum PDFLocation {
    Page(usize),
    // Selection,
    // Rectangle,
}

#[derive(Clone, Debug)]
pub struct PDFBibleMatch {
    psg: Passage,
    location: PDFLocation,
}

#[derive(Clone, Debug)]
pub struct PDFBibleMatcher(BibleMatcher);
impl Default for PDFBibleMatcher {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl std::ops::Deref for PDFBibleMatcher {
    type Target = BibleMatcher;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PDFBibleMatcher {
    // TODO: For cross-page references I can use the first and last lines
    pub fn search(&self, doc: Document) -> AnyResult<Vec<PDFBibleMatch>> {
        let matcher = &self.0;

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

/**
Obsidian PDF++:

```no_run
> [!PDF|yellow] [[01. O Perfect Redemption! - by Mike Riccardi - 117201843484205.pdf#page=3&selection=223,2,226,1&color=yellow|01. O Perfect Redemption! - by Mike Riccardi - 117201843484205, p.3]]
> > Hebrews 9:26
```

```no_run
Page: 3
```

*/
#[cfg(test)]
mod tests {
    use mupdf::{Document, TextPageOptions};
    use topos_lib::matcher::matcher::BibleMatcher;

    use super::*;

    const PATH: &'static str = "/home/dgmastertemple/Dropbox/Apps/remotely-save/Dropbox Library/Papers/O Perfect Redemption/01. O Perfect Redemption! - by Mike Riccardi - 117201843484205.pdf";

    #[test]
    fn it_works() -> topos_lib::error::AnyResult<()> {
        let doc = Document::open(PATH)?;
        // let page = doc.load_page(3)?; // page 3?
        let page = doc.pages()?.nth(2).unwrap()?; // page 3

        let text = page.to_text()?;
        let matcher = BibleMatcher::default();

        // println!("```\n{}\n```", &text);

        let matches = matcher.search(&text);
        // dbg!(&matches);
        let heb = &matches[0];
        dbg!(&heb);

        let text_page = page.to_text_page(TextPageOptions::BLOCK_TEXT)?;
        dbg!(page.search("Hebrews", 3));
        // dbg!(&text_page);
        for block in text_page.blocks() {
            // to understand bounds/rect: https://pymupdf.readthedocs.io/en/latest/rect.html
            // dbg!(&block.bounds());
            for line in block.lines() {
                // dbg!(&line.bounds());
                let line_text = line.chars().filter_map(|c| c.char()).collect::<String>();
                if line_text.contains("Hebrews") {
                    dbg!(&block.bounds());
                    dbg!(&line);
                    dbg!(&line.bounds());

                    let bounds = line.bounds();
                    dbg!(block.ctm());

                    let x = bounds.x0;
                    let y = bounds.y0;

                    let x = bounds.origin().x;
                    let y = bounds.origin().y;

                    let w = bounds.width();
                    let h = bounds.height();
                    println!(
                        "![[01. O Perfect Redemption! - by Mike Riccardi - 117201843484205.pdf#page=3&rect={},{},{},{}&color=yellow|01. O Perfect Redemption! - by Mike Riccardi - 117201843484205, p.3]]",
                        x, y, w, h
                    );
                }
                // println!("{}", &line_text);
            }
        }

        Ok(())
    }
}
