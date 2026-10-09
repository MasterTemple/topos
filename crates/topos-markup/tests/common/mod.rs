//! Helpers shared by the integration tests; public API only.

#![allow(dead_code)]

pub mod golden;

use topos_bible_markup::{Element, Kind, Markup, Node, find_all, parse};

/// Parses and returns the elements of a kind, cloned
pub fn elements(markup: Markup, input: &str, kind: Kind) -> Vec<Element> {
    let nodes = parse(markup, input);
    find_all(&nodes, kind).into_iter().cloned().collect()
}

/// The first element of the top level
pub fn first_element(nodes: &[Node]) -> &Element {
    nodes
        .iter()
        .find_map(Node::as_element)
        .expect("an element at the top level")
}

/// The tree in the golden files' textual form
pub fn tree(markup: Markup, input: &str) -> String {
    golden::dump_tree(&parse(markup, input))
}
