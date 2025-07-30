use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
use std::sync::Arc;

use crossbeam_channel::{Receiver, unbounded};
use ignore::{WalkBuilder, WalkState};
use itertools::Either;
use topos_lib::matcher::matcher::BibleMatcher;

use crate::matches::PathMatches;

#[derive(Clone, Debug)]
pub enum InputType {
    Directory(PathBuf),
    File(PathBuf),
    TextInput(String),
}

impl InputType {
    pub fn new(input: Option<String>) -> Self {
        match input {
            Some(input) => {
                let path = PathBuf::from(&input);
                if path.is_file() {
                    Self::File(path)
                } else if path.is_dir() {
                    Self::Directory(path)
                } else {
                    Self::TextInput(input)
                }
            }
            None => {
                if !io::stdin().is_terminal() {
                    // If no positional argument is given and stdin is being piped, read from stdin
                    let mut buffer = String::new();
                    io::stdin().read_to_string(&mut buffer).unwrap();
                    Self::TextInput(buffer.trim_end().to_string())
                } else {
                    Self::default()
                }
            }
        }
    }

    pub fn search(self, matcher: BibleMatcher) -> impl Iterator<Item = PathMatches> {
        match self {
            InputType::Directory(path) => Either::Left(handle_dir(path, matcher)),
            InputType::File(path) => {
                Either::Right(std::iter::once(PathMatches::from_file(path, &matcher)))
            }
            InputType::TextInput(text) => {
                Either::Right(std::iter::once(PathMatches::from_text(text, &matcher)))
            }
        }
    }
}

impl Default for InputType {
    fn default() -> Self {
        Self::Directory(PathBuf::from("."))
    }
}

fn handle_dir(path: PathBuf, matcher: BibleMatcher) -> impl Iterator<Item = PathMatches> {
    let walk = WalkBuilder::new(path);
    let receiver = run_multi_threaded_streaming(walk, matcher);
    receiver.into_iter()
}

fn run_multi_threaded_streaming<'a>(
    walk: WalkBuilder,
    matcher: BibleMatcher,
) -> Receiver<PathMatches> {
    let (sender, receiver) = unbounded();

    // Clone the matcher so it can be moved into the thread
    let matcher = Arc::new(matcher);
    let walk = walk.build_parallel();

    // Spawn the parallel walk in a background thread
    std::thread::spawn(move || {
        walk.run(|| {
            let sender = sender.clone();
            let matcher = Arc::clone(&matcher);

            Box::new(move |entry| {
                match entry {
                    Ok(entry) => {
                        if entry.path().is_dir() {
                            return WalkState::Continue;
                        }
                        let path = entry.path().to_path_buf();
                        let matches = PathMatches::from_file(path, matcher.as_ref());
                        // Send result to channel
                        if sender.send(matches).is_err() {
                            // Receiver was dropped, stop early
                            return WalkState::Quit;
                        }
                    }
                    Err(err) => {
                        eprintln!("Error: {}", err);
                    }
                }
                WalkState::Continue
            })
        });
    });

    receiver
}
