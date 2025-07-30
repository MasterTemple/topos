use clap::Parser;

use crate::args::Args;

pub mod args;

pub fn main() {
    let args = Args::parse();
    dbg!(&args);
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
