//! A Bible book's text in markup (topos-bible-markup): its verses, and passages as HTML or plain
//! text. Behind the `markup` feature (on by default); topos itself never holds Bible text, the
//! caller brings it.

use boltffi::*;
use topos_bible_markup::{
    Filter, HtmlOptions, HtmlRenderer, Kind, Markup, Node, TextOptions, find_all, parse,
    plain_text,
    verses::{VerseSpan, chapter_verses, passage_ranges, slice, usx_chapter_verses, verses},
};
use topos_lib::segments::Passage as CorePassage;

use crate::{Passage, Topos, ToposError};

/// One verse of a [`ToposBook`]
#[data]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BookVerse {
    pub chapter: u8,
    pub verse: u8,
    /// The last verse of a bridge like `4-5` (else the same as `verse`)
    pub last: u8,
}

/// How [`ToposBook::html`] and [`ToposBook::text`] write a passage
#[data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BookTextOptions {
    /// Footnotes and cross references
    pub notes: bool,
    /// Section headings
    pub headings: bool,
    /// Prefix of every HTML class (`topos-` gives `topos-verse`); data attributes are never
    /// prefixed
    pub class_prefix: String,
}

/**
A Bible book's text, parsed once and read a passage at a time

- `markup` names the format: `usx`, or `osis` for a book with verse milestones or elements
- Each verse runs from the end of the one before it to its own end, so a heading belongs to the
  verse after it, and a passage keeps its paragraphs and poetry lines
- HTML uses topos-bible-markup's classes: verse and chapter markers are empty
  `<span class="topos-verse" data-n="16" data-osis-id="JHN 3:16">`s, which a stylesheet can number

```ts
const book = ToposBook.parse(topos, "usx", await read("JHN.usx"));
book.bookId(); // 43
const options = { notes: false, headings: true, classPrefix: "topos-" };
book.html(topos.parse("John 3:16-18")!, options); // one HTML string for each part of the reference
```
*/
pub struct ToposBook {
    nodes: Vec<Node>,
    spans: Vec<VerseSpan>,
    book_id: Option<u8>,
}

impl ToposBook {
    /// The book's tree for each part of a passage in this book
    fn parts(&self, passage: &Passage) -> Vec<Vec<Node>> {
        if self.book_id != Some(passage.book_id) {
            return Vec::new();
        }
        let passage = CorePassage::from(passage);
        passage_ranges(&self.spans, &passage)
            .into_iter()
            .map(|r| {
                let (first, last) = (&self.spans[r.start], &self.spans[r.end - 1]);
                slice(&self.nodes, &first.start, Some(&last.end))
            })
            .collect()
    }
}

fn filter(options: &BookTextOptions) -> Filter {
    let mut filter = Filter::all().drop(Kind::Metadata);
    if !options.notes {
        filter = filter.drop(Kind::Note);
    }
    if !options.headings {
        filter = filter.drop(Kind::Title);
    }
    filter
}

#[export]
impl ToposBook {
    /// Parses a whole book (this never fails on odd markup; only an unknown `markup` is an error,
    /// which is `null` in JavaScript)
    pub fn parse(topos: &Topos, markup: String, text: String) -> Result<Self, ToposError> {
        let format = Markup::from_name(&markup).ok_or_else(|| ToposError::InvalidMarkup {
            message: format!("unknown markup {markup:?} (usx, osis)"),
        })?;
        let nodes = parse(format, &text);
        let books = topos.matcher.data().books();
        // USX `<book code="JHN">`, OSIS `<div type="book" osisID="John">`
        let book_id = find_all(&nodes, Kind::Metadata)
            .into_iter()
            .find_map(|e| e.attr("code"))
            .and_then(|code| books.search_usfm(code))
            .or_else(|| {
                find_all(&nodes, Kind::Division)
                    .into_iter()
                    .filter(|e| e.attr("type") == Some("book"))
                    .find_map(|e| books.search_osis(e.attr("osisID")?))
            })
            .map(|id| id.0);
        let spans = verses(&nodes);
        Ok(Self {
            nodes,
            spans,
            book_id,
        })
    }

    /// A USX book's verse counts per chapter (like `chapterVerses`), from its chapter and verse
    /// markers alone: fast enough to count a whole Bible without parsing it
    pub fn usx_chapter_verses(usx: String) -> Vec<u8> {
        usx_chapter_verses(&usx)
    }

    /// The book (1 for Genesis through 66 for Revelation with the default data), if the markup
    /// names one the data knows
    pub fn book_id(&self) -> Option<u8> {
        self.book_id
    }

    /// Every verse, in order (verses the translation leaves out are missing)
    pub fn verses(&self) -> Vec<BookVerse> {
        self.spans
            .iter()
            .map(|s| BookVerse {
                chapter: s.chapter,
                verse: s.verse,
                last: s.last,
            })
            .collect()
    }

    /// The number of verses in each chapter, from chapter 1, as the book numbers them: its
    /// versification, for `ToposTranslations.chapterVerses`
    pub fn chapter_verses(&self) -> Vec<u8> {
        chapter_verses(&self.spans)
    }

    /// The chapters with verses, in order
    pub fn chapters(&self) -> Vec<u8> {
        let mut chapters: Vec<u8> = self.spans.iter().map(|s| s.chapter).collect();
        chapters.dedup();
        chapters
    }

    /// Each part of the passage (`John 3:16; 4` has two) as HTML, leaving out parts the book
    /// doesn't have; nothing for a passage in another book
    pub fn html(&self, passage: Passage, options: BookTextOptions) -> Vec<String> {
        // One renderer for the passage, so words of Jesus across verses stay marked
        let mut html = HtmlRenderer::with_options(HtmlOptions {
            class_prefix: options.class_prefix.clone().into(),
            filter: filter(&options),
        });
        self.parts(&passage)
            .iter()
            .map(|nodes| html.render(nodes))
            .collect()
    }

    /// Each part of the passage as plain text (notes in brackets when kept), like
    /// [`ToposBook::html`]
    pub fn text(&self, passage: Passage, options: BookTextOptions) -> Vec<String> {
        let text = TextOptions {
            filter: filter(&options),
            ..TextOptions::default()
        };
        self.parts(&passage)
            .iter()
            .map(|nodes| plain_text(nodes, &text).trim().to_string())
            .collect()
    }

    /// A whole chapter as HTML (its headings, and the verses the book has in it); empty if it has
    /// none
    pub fn chapter_html(&self, chapter: u8, options: BookTextOptions) -> String {
        let first = self.spans.iter().position(|s| s.chapter == chapter);
        let last = self.spans.iter().rposition(|s| s.chapter == chapter);
        let (Some(first), Some(last)) = (first, last) else {
            return String::new();
        };
        let nodes = slice(
            &self.nodes,
            &self.spans[first].start,
            Some(&self.spans[last].end),
        );
        HtmlRenderer::with_options(HtmlOptions {
            class_prefix: options.class_prefix.clone().into(),
            filter: filter(&options),
        })
        .render(&nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BookStyle;

    const USX: &str = r#"<usx version="3.0"><book code="JHN" style="id">x</book>
<chapter number="3" style="c" sid="JHN 3"/>
<para style="s1">Jesus and Nicodemus</para>
<para style="p"><verse number="16" style="v" sid="JHN 3:16"/>For God so loved the world<note caller="+" style="f"><char style="ft">Or so</char></note>.<verse eid="JHN 3:16"/>
<verse number="17" style="v" sid="JHN 3:17"/>For God did not send His Son.<verse eid="JHN 3:17"/></para>
<chapter eid="JHN 3"/>
<chapter number="4" style="c" sid="JHN 4"/>
<para style="p"><verse number="1" style="v" sid="JHN 4:1"/>Now Jesus learned<verse eid="JHN 4:1"/></para>
<chapter eid="JHN 4"/></usx>"#;

    fn options(notes: bool, headings: bool) -> BookTextOptions {
        BookTextOptions {
            notes,
            headings,
            class_prefix: "topos-".into(),
        }
    }

    #[test]
    fn passages_as_html_and_text() {
        let topos = Topos::new();
        let book = ToposBook::parse(&topos, "usx".into(), USX.into()).unwrap();
        assert_eq!(book.book_id(), Some(43));
        assert_eq!(book.chapters(), [3, 4]);
        assert_eq!(book.chapter_verses(), [0, 0, 17, 1]);
        assert_eq!(ToposBook::usx_chapter_verses(USX.into()), [0, 0, 17, 1]);
        assert_eq!(book.verses().len(), 3);
        let parse = |r: &str| topos.parse(r.into(), BookStyle::Name).unwrap();

        assert_eq!(
            book.text(parse("John 3:16; 4"), options(false, true)),
            [
                "Jesus and Nicodemus\nFor God so loved the world.",
                "Now Jesus learned"
            ]
        );
        assert_eq!(
            book.text(parse("John 3:16-17"), options(true, false)),
            ["For God so loved the world [Or so]. For God did not send His Son."]
        );
        let html = book.html(parse("John 3:17"), options(false, true));
        assert_eq!(
            html,
            [concat!(
                r#"<p class="topos-p">"#,
                "\n", // the line break before the verse in the USX
                r#"<span class="topos-verse" data-osis-id="JHN 3:17" data-n="17"></span>For God did not send His Son.</p>"#
            )]
        );
        // Other books and missing chapters give nothing
        assert!(
            book.html(parse("Romans 8"), options(false, true))
                .is_empty()
        );
        assert!(book.html(parse("John 9"), options(false, true)).is_empty());
        assert!(
            book.chapter_html(3, options(false, false))
                .contains("did not send")
        );
        assert!(
            !book
                .chapter_html(3, options(false, false))
                .contains("Nicodemus")
        );
        assert!(ToposBook::parse(&topos, "rtf".into(), String::new()).is_err());
    }
}
