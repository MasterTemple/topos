//! Builds a [`Node`] tree from a stream of tag/text events, tolerating the
//! unbalanced fragments SWORD entries contain.

use super::{Element, Kind, Milestone, Node};

pub(crate) struct TreeBuilder {
    root: Vec<Node>,
    stack: Vec<Element>,
    classify: fn(&mut Element),
}

impl TreeBuilder {
    pub(crate) fn new(classify: fn(&mut Element)) -> Self {
        TreeBuilder {
            root: Vec::new(),
            stack: Vec::new(),
            classify,
        }
    }

    fn children(&mut self) -> &mut Vec<Node> {
        match self.stack.last_mut() {
            Some(e) => &mut e.children,
            None => &mut self.root,
        }
    }

    pub(crate) fn text(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let kids = self.children();
        if let Some(Node::Text(t)) = kids.last_mut() {
            t.push_str(s);
        } else {
            kids.push(Node::Text(s.to_string()));
        }
    }

    pub(crate) fn start(&mut self, e: Element) {
        self.stack.push(e);
    }

    pub(crate) fn empty(&mut self, mut e: Element) {
        (self.classify)(&mut e);
        self.children().push(Node::Element(e));
    }

    pub(crate) fn end(&mut self, name: &str) {
        match self
            .stack
            .iter()
            .rposition(|e| e.name.eq_ignore_ascii_case(name))
        {
            Some(i) => {
                while self.stack.len() > i + 1 {
                    self.close_top(true);
                }
                self.close_top(false);
            }
            None => {
                let mut e = Element::new(name, Vec::new());
                e.milestone = Some(Milestone::End);
                (self.classify)(&mut e);
                self.children().push(Node::Element(e));
            }
        }
    }

    fn close_top(&mut self, unclosed: bool) {
        if let Some(mut e) = self.stack.pop() {
            e.unclosed = unclosed;
            (self.classify)(&mut e);
            self.children().push(Node::Element(e));
        }
    }

    pub(crate) fn finish(mut self) -> Vec<Node> {
        while !self.stack.is_empty() {
            self.close_top(true);
        }
        self.root
    }

    /// For ThML `<sync type="Strongs">` / GBF `<WG25>`, which *follow* the word they
    /// tag: wrap the preceding word in a synthetic `w` element (or extend the `w`
    /// created by a previous tag) and add the value to its `attr`.
    pub(crate) fn tag_previous_word(&mut self, attr: &str, value: String) {
        let kids = self.children();
        let need_new = !matches!(kids.last(), Some(Node::Element(e)) if e.kind == Kind::Word);
        if need_new {
            let mut w = Element::new("w", Vec::new());
            w.kind = Kind::Word;
            if let Some(Node::Text(t)) = kids.last_mut() {
                let trimmed = t.trim_end();
                if trimmed.len() == t.len() {
                    // text ends in a word: split it off
                    let start = t
                        .rfind(char::is_whitespace)
                        .map_or(0, |i| i + t[i..].chars().next().unwrap().len_utf8());
                    let word = t[start..].to_string();
                    t.truncate(start);
                    if t.is_empty() {
                        kids.pop();
                    }
                    w.children.push(Node::Text(word));
                }
            }
            kids.push(Node::Element(w));
        }
        if let Some(Node::Element(w)) = kids.last_mut() {
            let v = match w.attr(attr) {
                Some(old) => format!("{old} {value}"),
                None => value,
            };
            w.set_attr(attr, v);
        }
    }
}
