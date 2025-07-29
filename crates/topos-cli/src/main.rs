use clap::Parser;

use crate::args::Args;

pub mod args;

pub fn main() {
    let args = Args::parse();
    dbg!(&args);
}
