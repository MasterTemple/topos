use once_cell::sync::Lazy;
use topos_lib::matcher::{instance::BibleMatch, matcher::BibleMatcher};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {} from Rust!", name)
}

static BIBLE: Lazy<BibleMatcher> = Lazy::new(|| BibleMatcher::default());

#[wasm_bindgen]
pub fn search(input: &str) -> Vec<String> {
    let m = &*BIBLE;
    m.search(input)
        .iter()
        .map(|r| {
            let name = m.data().books().get_name(r.psg.book).unwrap();
            let segments = r.psg.segments.to_string();
            let start = r.location.start;
            format!("[{}:{}] {name} {segments}", start.line, start.column)
        })
        .collect()
}
