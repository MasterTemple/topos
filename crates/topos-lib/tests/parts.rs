//! Where each segment of a found reference is written (`BibleMatcher::search_with_parts`)

use topos_bible::matcher::BibleMatcher;

/// Each match's parts as the text they cover
fn parts(text: &str) -> Vec<Vec<&str>> {
    let matcher = BibleMatcher::default();
    matcher
        .search_with_parts(text)
        .into_iter()
        .map(|(m, parts)| {
            assert_eq!(parts.len(), m.psg.segments.len(), "{text:?}");
            parts.iter().map(|p| &text[p.written.clone()]).collect()
        })
        .collect()
}

#[test]
fn each_segment_has_its_place() {
    assert_eq!(parts("See John 3:16 today"), [["John 3:16"]]);
    assert_eq!(parts("John 3:16; 4:5-7"), [["John 3:16", "4:5-7"]]);
    assert_eq!(parts("Rom 8:28, 31, 38-39."), [["Rom 8:28", "31", "38-39"]]);
    assert_eq!(
        parts("Ps 23, 1 Cor 13:4-7"),
        [vec!["Ps 23"], vec!["1 Cor 13:4-7"]]
    );
    // An old-style verse after a Roman numeral chapter is one segment with its chapter
    assert_eq!(parts("Matth. x, 8"), [["Matth. x, 8"]]);
    // Offsets are in the original text, before it is normalized
    assert_eq!(parts("“John 3:16–18; 4:1”"), [["John 3:16–18", "4:1"]]);
}

#[test]
fn a_range_says_where_its_end_is() {
    let text = "John 3:16-18; 4:5, 1-2";
    let matcher = BibleMatcher::default();
    let (_, parts) = matcher.search_with_parts(text).remove(0);
    let ends: Vec<_> = parts
        .iter()
        .map(|p| p.end.map(|end| &text[end..p.written.end]))
        .collect();
    assert_eq!(ends, [Some("18"), None, Some("2")]);
}

#[test]
fn a_segment_starts_and_ends_somewhere() {
    use topos_bible::segments::segment::Segment;
    let range = Segment::chapter_verse_range(4, 5, 7);
    assert_eq!(
        (range.first(), range.last()),
        (Segment::chapter_verse(4, 5), Segment::chapter_verse(4, 7))
    );
    let chapters = Segment::full_chapter_range(3, 4);
    assert_eq!(
        (chapters.first(), chapters.last()),
        (Segment::full_chapter(3), Segment::full_chapter(4))
    );
}
