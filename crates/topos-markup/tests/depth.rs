//! The depth limit (docs/spec/parsing.md, "Depth limit"): at most 1,000 open elements, so crafted
//! input can't overflow the stack. Run in a thread with a 2 MB stack, smaller than a main thread's

use topos_bible_markup::{
    HtmlRenderer, Markup, Node, TextOptions, flatten::flatten, parse, plain_text,
};

const LIMIT: usize = 1_000;
/// The cut elements sit inside the innermost open element: one level more
const LEVELS: usize = LIMIT + 1;
const DEEP: usize = 100_000;

/// The tree's depth and every element at the deepest level, without recursion
fn depth(nodes: &[Node]) -> usize {
    let mut deepest = 0;
    let mut stack: Vec<(&[Node], usize)> = vec![(nodes, 1)];
    while let Some((nodes, level)) = stack.pop() {
        for node in nodes {
            if let Node::Element(e) = node {
                deepest = deepest.max(level);
                stack.push((&e.children, level + 1));
            }
        }
    }
    deepest
}

/// The innermost element of a chain of first children
fn innermost(nodes: &[Node]) -> &[Node] {
    let mut nodes = nodes;
    while let Some(Node::Element(e)) = nodes.first() {
        nodes = &e.children;
    }
    nodes
}

fn on_small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deep_xml_is_cut_at_the_limit() {
    on_small_stack(|| {
        for markup in [Markup::Osis, Markup::Thml, Markup::Tei, Markup::Usx] {
            let input = format!("{}x", "<q>".repeat(DEEP));
            let nodes = parse(markup, &input);
            assert_eq!(depth(&nodes), LEVELS, "{markup:?}");
            // The innermost open `q` holds the rest as empty, unclosed elements, then the text
            let inner = innermost(&nodes);
            assert!(inner.is_empty());
            let mut chain = &nodes[..];
            for _ in 1..LIMIT {
                chain = &chain[0].as_element().unwrap().children;
            }
            let last = &chain[0].as_element().unwrap().children;
            assert_eq!(last.len(), DEEP - LIMIT + 1, "{markup:?}");
            for node in &last[..DEEP - LIMIT] {
                let e = node.as_element().unwrap();
                assert!(e.children.is_empty() && e.unclosed && e.name == "q");
            }
            assert_eq!(last[DEEP - LIMIT], Node::Text("x".into()));

            // Everything else works on it
            let copy = nodes.clone();
            assert_eq!(copy, nodes);
            let _ = format!("{nodes:?}");
            let _ = plain_text(&nodes, &TextOptions::default());
            let _ = flatten(&nodes, &TextOptions::default());
            let _ = HtmlRenderer::new().render(&nodes);
        }
    });
}

#[test]
fn deep_gbf_is_cut_at_the_limit() {
    on_small_stack(|| {
        let nodes = parse(Markup::Gbf, &format!("{}x", "<FR>".repeat(DEEP)));
        assert_eq!(depth(&nodes), LEVELS);
        let _ = HtmlRenderer::new().render(&nodes);
    });
}

#[test]
fn closing_tags_still_close_after_the_limit() {
    // Past the limit, a start tag and its end tag: the end closes the innermost open `q`
    let input = format!("{}a<q>b</q>c", "<q>".repeat(LIMIT));
    let nodes = parse(Markup::Osis, &input);
    assert_eq!(depth(&nodes), LEVELS);
    assert_eq!(
        plain_text(&nodes, &TextOptions::default()),
        "abc",
        "the text is all there"
    );
    // Shallow markup is unchanged
    let shallow = parse(Markup::Osis, "<q><q>a</q></q>");
    assert!(!shallow[0].as_element().unwrap().unclosed);
    assert_eq!(depth(&shallow), 2);
}
