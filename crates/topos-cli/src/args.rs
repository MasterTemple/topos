use clap::Parser;
use std::path::PathBuf;
use topos_lib::{
    data::genres::Genres,
    filter::{
        filter::{BibleFilter, IsFilter, Operation},
        filters::{book::BookFilter, genre::GenreFilter, testament::TestamentFilter},
    },
    matcher::matcher::BibleMatcher,
};

/**
- By positively specifying a testament/genre/book, you will implicitly telling the program to exclude the remaining items in that category.
- You may choose to exclude a subset from a larger inclusion (ex: book from a genre), however this must be specified **after** the inclusion (or else it will be re-added)
- You may combine multiple filters, and they will be joined with a logical OR
*/
#[derive(Parser, Debug)]
#[clap(
    name = "topos",
    about = "A Bible verse-aware search tool inspired by ripgrep",
    version = "0.1.0"
)]
pub struct Args {
    // Input source: text or file
    #[clap(
        long = "text",
        long = "input",
        // group = "input",
        // These have to be 2 separate groups, because I do want the user to be able to specify
        // files and directories
        group = "input_or_files",
        group = "input_or_dirs",
        help = "Text to search within (instead of files)"
    )]
    pub input: Option<String>,

    // File and directory paths
    #[clap(
        long = "file",
        short,
        // group = "input",
        group = "input_or_files",
        help = "One or more files to search"
    )]
    pub files: Vec<PathBuf>,

    #[clap(
        long = "dir",
        short,
        // group = "input",
        group = "input_or_dirs",
        help = "One or more directories to search recursively"
    )]
    pub dirs: Vec<PathBuf>,

    // Testament filters
    #[clap(
        long = "testament",
        short,
        help = "Include books from a specific testament (old/new)"
    )]
    pub testaments: Option<Vec<TestamentFilter>>,

    #[clap(
        long = "exclude-testament",
        help = "Exclude books from a specific testament"
    )]
    pub exclude_testaments: Option<Vec<TestamentFilter>>,

    // Genre filters
    #[clap(
        long = "genre",
        short,
        help = "Include books of a specific genre (e.g. epistles, gospels)"
    )]
    pub genres: Option<Vec<String>>,

    #[clap(long = "exclude-genre", help = "Exclude books of a specific genre")]
    pub exclude_genres: Option<Vec<String>>,

    // Book filters
    #[clap(long = "book", short, help = "Include specific books (e.g. John)")]
    pub books: Option<Vec<String>>,

    #[clap(long = "exclude-book", help = "Exclude specific books")]
    pub exclude_books: Option<Vec<String>>,

    // Verse range filters
    #[clap(
        long = "inside",
        short = 'i',
        help = "Limit search to a verse range (e.g. John 1:1-5)"
    )]
    pub inside: Option<Vec<String>>,

    // Verse range filters
    #[clap(
        long = "outside",
        short = 'o',
        help = "Forbid search from matching a verse range (e.g. John 1:1-5)"
    )]
    pub outside: Option<Vec<String>>,
    // // Verse match mode
    // #[clap(long = "matches", help = "Check if the input is a valid Bible verse")]
    // pub matches: Option<String>,
    //
    // // Boolean flags
    // #[clap(
    //     long = "check",
    //     group = "operation",
    //     help = "Return true/false (or 0/1) if a verse is present"
    // )]
    // pub check: bool,
    //
    // // Boolean flags
    // #[clap(long = "first", group = "operation", help = "Get first verse")]
    // pub first: bool,
    //
    // #[clap(
    //     long = "config",
    //     // parse(from_os_str),
    //     help = "Use a custom configuration file"
    // )]
    // pub config: Option<PathBuf>,
    //
    // #[clap(long = "igonre", help = "Ignore when non-real books/genres are given")]
    // pub ignore_non_existent: bool,
}

fn idk(args: Args) {
    // TODO: get alternate Bible/Genre data
    let mut filter = BibleFilter::default();

    if let Some(list) = args.testaments {
        filter.include_many(list);
    }

    if let Some(list) = args.genres {
        filter.include_many(list.into_iter().map(GenreFilter::new).collect());
    }

    if let Some(list) = args.books {
        filter.include_many(list.into_iter().map(BookFilter::new).collect());
    }

    if let Some(list) = args.exclude_testaments {
        filter.exclude_many(list);
    }

    if let Some(list) = args.exclude_genres {
        filter.exclude_many(list.into_iter().map(GenreFilter::new).collect());
    }

    if let Some(list) = args.exclude_books {
        filter.exclude_many(list.into_iter().map(BookFilter::new).collect());
    }

    if let Some(list) = args.inside {
        for value in list {
            filter.filter_inside(&value);
        }
    }

    if let Some(list) = args.outside {
        for value in list {
            filter.filter_outside(&value);
        }
    }

    let matcher = filter.create_matcher().unwrap();
    // matcher
}

impl<'a> TryFrom<Args> for BibleMatcher<'a> {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: Args) -> Result<Self, Self::Error> {
        todo!()
    }
}
