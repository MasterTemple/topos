use clap::Parser;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use topos_lib::matcher::matcher::BibleMatcher;

use ignore::{WalkBuilder, WalkState};

use crate::args::Args;

pub mod args;

pub fn main() {
    let args = Args::parse();
    dbg!(&args);

    // let walk = WalkBuilder::new(".").build_parallel();
    let walk = WalkBuilder::new(".").build();

    // let results: Arc<Mutex<Vec<_>>> = Arc::new(Mutex::new(Vec::new()));
    let matcher = BibleMatcher::default();
    dbg!(&matcher);

    for entry in walk {
        dbg!(&entry);
        let entry = entry.unwrap();
        // multiple ways to check if is dir
        // entry.metadata().unwrap().is_dir()
        // entry.file_type().unwrap().is_dir();
        // entry.path().is_dir()
        if entry.metadata().unwrap().is_dir() {
            // return WalkState::Continue;
            continue;
        }

        let Ok(contents) = &std::fs::read_to_string(entry.path()) else {
            // return WalkState::Continue;
            continue;
        };
        dbg!(&contents);
        let matches = matcher.search(&contents);
        dbg!(&matches);

        // results.lock().unwrap().extend(matches);
        // results.lock().unwrap().push(entry.path().to_path_buf());

        // println!("------------------------------");
        // println!("{:?}", entry.path());
        // println!("------------------------------");
        //
        // let file = File::open(entry.path()).unwrap();
        // let reader = BufReader::new(file);
        // for (idx, line) in reader.lines().enumerate() {
        //     println!("{}. {}", idx, line.unwrap());
        // }
        // println!("------------------------------");
        // WalkState::Continue
    }

    // walk.run(|| {
    //     Box::new(|entry| match entry {
    //         Ok(entry) => {
    //             // multiple ways to check if is dir
    //             // entry.metadata().unwrap().is_dir()
    //             // entry.file_type().unwrap().is_dir();
    //             // entry.path().is_dir()
    //             if entry.metadata().unwrap().is_dir() {
    //                 return WalkState::Continue;
    //             }
    //
    //             let Ok(contents) = &std::fs::read_to_string(entry.path()) else {
    //                 return WalkState::Continue;
    //             };
    //             let matches = matcher.search(&contents);
    //             dbg!(&matches);
    //
    //             results.lock().unwrap().extend(matches);
    //             // results.lock().unwrap().push(entry.path().to_path_buf());
    //
    //             // println!("------------------------------");
    //             // println!("{:?}", entry.path());
    //             // println!("------------------------------");
    //             //
    //             // let file = File::open(entry.path()).unwrap();
    //             // let reader = BufReader::new(file);
    //             // for (idx, line) in reader.lines().enumerate() {
    //             //     println!("{}. {}", idx, line.unwrap());
    //             // }
    //             // println!("------------------------------");
    //             WalkState::Continue
    //         }
    //         Err(err) => {
    //             eprintln!("Error: {}", err);
    //             WalkState::Continue
    //         }
    //     })
    // });

    // dbg!(results);
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
