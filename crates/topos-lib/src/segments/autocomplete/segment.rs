// /**
// This comes after complete segments
// */
// pub enum CompletionSegment {
//     /// `John ?`
//     Chapter,
//     /// `John 1:1,?`
//     ChapterOrVerse,
//     /// `John 1:1,2:?`
//     ChapterVerse { chapter: u8 },
//     /// `John 1:1,2:1-?`
//     ChapterVerseRange { chapter: u8, verse: u8 },
//     /// `John 1:1,2:1-3:?`
//     ChapterRange {
//         start_chapter: u8,
//         start_verse: u8,
//         end_chapter: u8,
//     },
// }
