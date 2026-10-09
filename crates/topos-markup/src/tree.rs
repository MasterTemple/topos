//! Building the tree from a stream of text, start, empty, and end events, shared by every markup
//! but plain text (which is also here, being only lines).

use crate::{Element, Kind, Markup, Milestone, Node, classify};

/// Appends the text of every text node below `nodes`, in document order
pub(crate) fn collect_text(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),
            Node::Element(e) => collect_text(&e.children, out),
        }
    }
}

/// Plain text: lines, with a line break element between each two
pub(crate) fn plain(text: &str) -> Vec<Node> {
    let mut nodes = Vec::new();
    if text.is_empty() {
        return nodes;
    }
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            nodes.push(Node::Element(Element {
                kind: Kind::LineBreak,
                name: String::from("br"),
                attrs: Vec::new(),
                children: Vec::new(),
                milestone: None,
                unclosed: false,
            }));
        }
        let line = line.strip_suffix('\r').unwrap_or(line);
        if !line.is_empty() {
            nodes.push(Node::Text(line.to_string()));
        }
    }
    nodes
}

/// The most elements open at once while parsing, so crafted input can't nest deep enough to
/// overflow the stack (parsing.md, "Depth limit")
const MAX_OPEN: usize = 1_000;

/// Builds a tree from events
pub(crate) struct Builder {
    markup: Markup,
    top: Vec<Node>,
    /// Elements opened and not yet closed, outermost first
    open: Vec<Element>,
}

impl Builder {
    pub(crate) fn new(markup: Markup) -> Self {
        Builder {
            markup,
            top: Vec::new(),
            open: Vec::new(),
        }
    }

    /// The node list new nodes go into
    fn container(&mut self) -> &mut Vec<Node> {
        match self.open.last_mut() {
            Some(e) => &mut e.children,
            None => &mut self.top,
        }
    }

    /// Adds text, joining it to a text node right before it
    pub(crate) fn text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let container = self.container();
        if let Some(Node::Text(last)) = container.last_mut() {
            last.push_str(text);
        } else {
            container.push(Node::Text(text.to_string()));
        }
    }

    /// Opens an element; at the depth limit it is added without children instead, marked
    /// unclosed, and what follows goes beside it
    pub(crate) fn start(&mut self, name: String, attrs: Vec<(String, String)>) {
        if self.open.len() >= MAX_OPEN {
            let mut e = Element {
                kind: Kind::Other,
                name,
                attrs,
                children: Vec::new(),
                milestone: None,
                unclosed: true,
            };
            classify::classify(self.markup, &mut e);
            self.container().push(Node::Element(e));
            return;
        }
        self.open.push(Element {
            kind: Kind::Other,
            name,
            attrs,
            children: Vec::new(),
            milestone: None,
            unclosed: false,
        });
    }

    /// Adds an element without children
    pub(crate) fn empty(&mut self, name: String, attrs: Vec<(String, String)>) {
        let mut e = Element {
            kind: Kind::Other,
            name,
            attrs,
            children: Vec::new(),
            milestone: None,
            unclosed: false,
        };
        classify::classify(self.markup, &mut e);
        self.container().push(Node::Element(e));
    }

    /// Closes the innermost open element with this name (ignoring ASCII case), or adds an orphan
    /// end
    pub(crate) fn end(&mut self, name: &str) {
        match self
            .open
            .iter()
            .rposition(|e| e.name.eq_ignore_ascii_case(name))
        {
            Some(index) => {
                while self.open.len() > index + 1 {
                    self.close_last(true);
                }
                self.close_last(false);
            }
            None => {
                let mut e = Element {
                    kind: Kind::Other,
                    name: name.to_string(),
                    attrs: Vec::new(),
                    children: Vec::new(),
                    milestone: Some(Milestone::End),
                    unclosed: false,
                };
                classify::classify(self.markup, &mut e);
                self.container().push(Node::Element(e));
            }
        }
    }

    /// Closes the innermost open element and adds it to its container
    fn close_last(&mut self, unclosed: bool) {
        if let Some(mut e) = self.open.pop() {
            e.unclosed = unclosed;
            classify::classify(self.markup, &mut e);
            self.container().push(Node::Element(e));
        }
    }

    /// Sets an attribute on the word before the current position, making the word if there is
    /// none (from the last word of the text before, or empty)
    pub(crate) fn tag_previous_word(&mut self, key: &str, value: &str) {
        let container = self.container();
        let is_word = matches!(container.last(), Some(Node::Element(e)) if e.kind == Kind::Word);
        if !is_word {
            let mut word = Element {
                kind: Kind::Word,
                name: String::from("w"),
                attrs: Vec::new(),
                children: Vec::new(),
                milestone: None,
                unclosed: false,
            };
            if let Some(Node::Text(text)) = container.last_mut()
                && !text.ends_with(char::is_whitespace)
            {
                let start = text
                    .char_indices()
                    .rev()
                    .find(|(_, c)| c.is_whitespace())
                    .map_or(0, |(i, c)| i + c.len_utf8());
                let last = text.split_off(start);
                if text.is_empty() {
                    container.pop();
                }
                if !last.is_empty() {
                    word.children.push(Node::Text(last));
                }
            }
            container.push(Node::Element(word));
        }
        if let Some(Node::Element(word)) = container.last_mut() {
            let existing = word.attrs.iter().position(|(k, _)| k == key).or_else(|| {
                word.attrs
                    .iter()
                    .position(|(k, _)| k.eq_ignore_ascii_case(key))
            });
            match existing.and_then(|i| word.attrs.get_mut(i)) {
                Some((_, v)) => {
                    v.push(' ');
                    v.push_str(value);
                }
                None => word.attrs.push((key.to_string(), value.to_string())),
            }
        }
    }

    /// Closes everything still open and gives the tree
    pub(crate) fn finish(mut self) -> Vec<Node> {
        while !self.open.is_empty() {
            self.close_last(true);
        }
        self.top
    }
}
