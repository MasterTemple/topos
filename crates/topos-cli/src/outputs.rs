use std::time::Instant;

use clap::ValueEnum;
use topos_lib::matcher::matcher::BibleMatcher;

use crate::matches::PathMatches;

#[derive(Copy, Clone, Debug, Default, ValueEnum)]
pub enum OutputMode {
    #[value(alias = "find", help = "Find matches and print total time")]
    Count,
    #[value(alias = "j", help = "Output matches as JSON")]
    JSON,
    #[default]
    #[value(alias = "t", help = "Output matches as a table (unaligned)")]
    Table,
    #[value(alias = "qf", help = "A format for the Neovim Quickfix List")]
    Quickfix,
}

/// But what about static methods, for things like column headers
trait OutputEntryFormat {
    fn json(self) -> String;
    fn table(self) -> String;
    fn quickfix(self) -> String;
}

impl OutputMode {
    /**
    TODO: this should not return an iterator of a struct, but an iterator of a type
    This type should implement [`OutputEntryFormat`]
    This is so that I can do different kinds of JSON outputs for example, based on the verbosity that the user requests (like context, ..)
    */
    pub fn write(&self, matcher: &BibleMatcher, results: impl Iterator<Item = PathMatches>) {
        match self {
            OutputMode::Count => print_time(matcher, results),
            OutputMode::JSON => print_json(matcher, results),
            OutputMode::Table => print_table(matcher, results),
            OutputMode::Quickfix => print_qf_list(matcher, results),
        }
    }
}

fn print_time(matcher: &BibleMatcher, results: impl Iterator<Item = PathMatches>) {
    let start = Instant::now();
    let mut count = 0;
    for PathMatches { path, matches } in results {
        let path = path
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        count += matches.len();
    }
    println!("Matches: {}", start.elapsed().as_millis());
    println!("Elapsed: {}ms", start.elapsed().as_millis());
}

fn print_json(matcher: &BibleMatcher, results: impl Iterator<Item = PathMatches>) {
    for PathMatches { path, matches } in results {
        let path = path
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        println!(r#"{{ "type": "start", "path": "{}" }}"#, path);
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

            // println!("{}:{}:{}: {}", path, start.line, start.column, psg)
            println!(r#"{{ "type": "match" }}"#);
        }
        println!(r#"{{ "type": "end", "path": "{}" }}"#, path);
    }
}

fn print_qf_list<'a>(matcher: &BibleMatcher, results: impl Iterator<Item = PathMatches>) {
    for PathMatches { path, matches } in results {
        let path = path
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
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

            println!("{}:{}:{}: {}", path, start.line, start.column, psg)
        }
    }
}

fn print_table<'a>(matcher: &BibleMatcher, results: impl Iterator<Item = PathMatches>) {
    println!("| File | Line | Col | Verse |");
    println!("| ---- | ---- | --- | ----- |");
    for PathMatches { path, matches } in results {
        let path = path
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
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
                path, start.line, start.column, book, segments
            )
        }
    }
}
