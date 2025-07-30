use clap::Parser;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};
use topos_lib::matcher::{
    instance::BibleMatch,
    matcher::{BibleMatcher, ThreadableBibleMatcher},
};

use ignore::{WalkBuilder, WalkState};

use crate::args::{Args, InputType};

pub mod args;

// pub fn run_single_threaded(walk: WalkBuilder, matcher: BibleMatcherData) {
//     for entry in walk.build() {
//         let entry = entry.unwrap();
//         if entry.path().is_dir() {
//             continue;
//         }
//
//         let Ok(contents) = &std::fs::read_to_string(entry.path()) else {
//             continue;
//         };
//         // println!("{:?}", entry.path());
//         let matches = matcher.search(&contents);
//         if matches.is_empty() {
//             continue;
//         }
//         //     println!("------------------------------");
//         //     println!("{:?}", entry.path());
//         //     println!("------------------------------");
//         //     // dbg!(&matches);
//         //     for m in matches {
//         //         let psg = m.psg;
//         //         let start = m.location.start;
//         //         print!("[L{}:{}]: ", start.line, start.column);
//         //         println!(
//         //             "{} {}",
//         //             // *psg.book,
//         //             matcher
//         //                 .data()
//         //                 .books()
//         //                 .get_name(psg.book)
//         //                 .unwrap_or(&format!("Book {}", psg.book.0)),
//         //             psg.segments
//         //                 .iter()
//         //                 .map(|e| e.to_string())
//         //                 .collect::<Vec<_>>()
//         //                 .join("\n")
//         //         );
//         //     }
//         //     println!("------------------------------");
//     }
// }

// pub fn run_multi_threaded(walk: WalkBuilder, matcher: BibleMatcherData) {
//     let walk = walk.build_parallel();
//     let results: Arc<Mutex<Vec<_>>> = Arc::new(Mutex::new(Vec::new()));
//
//     walk.run(|| {
//         Box::new(|entry| match entry {
//             Ok(entry) => {
//                 if entry.path().is_dir() {
//                     return WalkState::Continue;
//                 }
//
//                 let Ok(contents) = &std::fs::read_to_string(entry.path()) else {
//                     return WalkState::Continue;
//                 };
//
//                 let matches = matcher.search(&contents);
//                 results
//                     .lock()
//                     .unwrap()
//                     .push((entry.path().to_path_buf(), matches));
//
//                 WalkState::Continue
//             }
//             Err(err) => {
//                 eprintln!("Error: {}", err);
//                 WalkState::Continue
//             }
//         })
//     });
//
//     // dbg!(results);
// }

pub fn run_multi_threaded_ref<'a>(walk: WalkBuilder, matcher: ThreadableBibleMatcher<'a>) {
    let walk = walk.build_parallel();
    let results: Arc<Mutex<Vec<_>>> = Arc::new(Mutex::new(Vec::new()));

    walk.run(|| {
        Box::new(|entry| match entry {
            Ok(entry) => {
                if entry.path().is_dir() {
                    return WalkState::Continue;
                }

                let Ok(contents) = &std::fs::read_to_string(entry.path()) else {
                    return WalkState::Continue;
                };

                let matches = matcher.search(&contents);
                results
                    .lock()
                    .unwrap()
                    .push((entry.path().to_path_buf(), matches));

                WalkState::Continue
            }
            Err(err) => {
                eprintln!("Error: {}", err);
                WalkState::Continue
            }
        })
    });

    let results = results.lock().unwrap().clone();
    // let total: usize = results.iter().map(|(_, list)| list.len()).sum();
    // println!("Matches Found: {}\n", total);
    print_qf_list(matcher, results);

    // dbg!(results);
}

pub fn print_qf_list<'a>(
    matcher: ThreadableBibleMatcher<'a>,
    results: Vec<(PathBuf, Vec<BibleMatch>)>,
) {
    for (file, matches) in results {
        for m in matches {
            let psg = m.psg;

            let Some(book) = matcher.data().books().get_name(psg.book) else {
                continue;
            };

            let segments = psg
                .segments
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let start = m.location.start;
            let psg = format!("{} {}", book, segments);
            println!(
                "{}:{}:{}: {}",
                file.to_string_lossy(),
                start.line,
                start.column,
                psg
            )
        }
    }
}

pub fn print_table<'a>(
    matcher: ThreadableBibleMatcher<'a>,
    results: Vec<(PathBuf, Vec<BibleMatch>)>,
) {
    println!("| File | Line | Col | Verse |");
    println!("| ---- | ---- | --- | ----- |");
    for (file, matches) in results {
        for m in matches {
            let psg = m.psg;

            let Some(book) = matcher.data().books().get_name(psg.book) else {
                continue;
            };

            let segments = psg
                .segments
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let start = m.location.start;
            println!(
                "| {} | {} | {} | {} {} |",
                file.to_string_lossy(),
                start.line,
                start.column,
                book,
                segments
            )
        }
    }
}

pub fn handle_text(text: String, matcher: BibleMatcher) {
    let results = matcher.as_threadable().search(&text);
    print_table(matcher.as_threadable(), vec![(PathBuf::new(), results)])
}

pub fn handle_file(path: PathBuf, matcher: BibleMatcher) {
    let text = std::fs::read_to_string(&path).unwrap();
    handle_text(text, matcher)
}

pub fn handle_dir(path: PathBuf, matcher: BibleMatcher) {
    let walk = WalkBuilder::new(path);
    run_multi_threaded_ref(walk, matcher.as_threadable());
}

pub fn handle_input(input: InputType, matcher: BibleMatcher) {
    match input {
        InputType::Directory(path) => handle_dir(path, matcher),
        InputType::File(path) => handle_file(path, matcher),
        InputType::TextInput(text) => handle_text(text, matcher),
    }
}

pub fn main() {
    let args = Args::parse();
    // dbg!(&args);
    let input = InputType::new(args.input.clone());
    // let walk = WalkBuilder::new(args.files.get(0).unwrap_or(&PathBuf::from(".")));
    let matcher = BibleMatcher::try_from(args).unwrap();

    handle_input(input, matcher);

    // let timer = Instant::now();
    // run_single_threaded(walk.clone(), matcher.clone());
    // println!("Single Threaded: {}ms", timer.elapsed().as_millis());
    //
    // let timer = Instant::now();
    // run_multi_threaded(walk.clone(), matcher.clone());
    // println!("Multi Threaded: {}ms", timer.elapsed().as_millis());
    // let timer = Instant::now();
    // run_multi_threaded_ref(walk, matcher.as_threadable());
    // println!("Multi Threaded: {}ms", timer.elapsed().as_millis());
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    use ignore::{WalkBuilder, WalkState};

    #[test]
    fn walk() {
        let walk = WalkBuilder::new(".").build_parallel();
        walk.run(|| {
            Box::new(|entry| match entry {
                Ok(entry) => {
                    println!("Found: {:?}", entry.path());
                    println!("------------------------------");
                    if entry.metadata().unwrap().is_dir() {
                        return WalkState::Continue;
                    }
                    // multiple ways to check if is dir
                    // entry.metadata().unwrap().is_dir()
                    // entry.file_type().unwrap().is_dir();
                    // entry.path().is_dir()

                    let file = File::open(entry.path()).unwrap();
                    let reader = BufReader::new(file);
                    for (idx, line) in reader.lines().enumerate() {
                        println!("{}. {}", idx, line.unwrap());
                    }
                    println!("------------------------------");
                    WalkState::Continue
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    WalkState::Continue
                }
            })
        });
    }
}
