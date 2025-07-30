use clap::Parser;
use crossbeam_channel::{Receiver, unbounded};
use itertools::Either;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};
use topos_lib::matcher::{instance::BibleMatch, matcher::BibleMatcher};

use ignore::{WalkBuilder, WalkState};

use crate::{args::Args, inputs::InputType, outputs::OutputMode};

pub mod args;
pub mod inputs;
pub mod matches;
pub mod outputs;

pub fn main() {
    let args = Args::parse();
    let input = InputType::new(args.input.clone());
    let output = args.mode;

    let matcher = BibleMatcher::try_from(args).unwrap();

    let results = input.search(matcher.clone());
    output.write(&matcher, results);
}
