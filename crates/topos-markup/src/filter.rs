//! Which elements are rendered, flattened and searched.

use std::sync::Arc;

use crate::{Element, Kind};

/// What happens to an element
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Render the element and its contents
    Keep,
    /// Leave out the element (no tag, class, or decoration) but keep its contents
    Unwrap,
    /// Leave out the element and its contents
    Drop,
}

type Rule = Arc<dyn Fn(&Element) -> Option<Action> + Send + Sync>;

/**
Decides, for each element, whether it is kept, unwrapped, or dropped

The first that applies wins:
1. Custom rules ([`Filter::rule`]), latest first
2. Note types ([`Filter::note_type`]), for [`Kind::Note`]
3. Kinds ([`Filter::kind`])
4. [`Action::Keep`]

```
use topos_bible_markup::{Action, Filter, Kind, Markup, TextOptions, parse, plain_text};

let nodes = parse(Markup::Osis, r#"<title>The Word</title>In the beginning<note type="crossReference">Gen 1:1</note><note type="explanation">Or, origin</note>"#);
// The default is the verse text: no notes or titles
assert_eq!(plain_text(&nodes, &TextOptions::default()), "In the beginning");
// Footnotes but not cross references
let filter = Filter::text().keep(Kind::Note).note_type("crossReference", Action::Drop);
let opts = TextOptions { filter, ..TextOptions::default() };
assert_eq!(plain_text(&nodes, &opts), "In the beginning [Or, origin]");
```
*/
#[derive(Clone, Default)]
pub struct Filter {
    /// The action set for each kind, at most one per kind
    kinds: Vec<(Kind, Action)>,
    /// Note types and their actions, in the order they were added
    note_types: Vec<(String, Action)>,
    /// Custom rules, in the order they were added
    rules: Vec<Rule>,
}

impl Filter {
    /// Everything is kept
    pub fn all() -> Self {
        Self::default()
    }

    /// The text to read: notes, titles, and metadata are dropped
    pub fn text() -> Self {
        Self::all()
            .drop(Kind::Note)
            .drop(Kind::Title)
            .drop(Kind::Metadata)
    }

    /// Sets what happens to a kind of element
    pub fn kind(mut self, kind: Kind, action: Action) -> Self {
        match self.kinds.iter_mut().find(|(k, _)| *k == kind) {
            Some(entry) => entry.1 = action,
            None => self.kinds.push((kind, action)),
        }
        self
    }

    pub fn keep(self, kind: Kind) -> Self {
        self.kind(kind, Action::Keep)
    }

    pub fn unwrap(self, kind: Kind) -> Self {
        self.kind(kind, Action::Unwrap)
    }

    pub fn drop(self, kind: Kind) -> Self {
        self.kind(kind, Action::Drop)
    }

    /// Sets what happens to notes of a type (`crossReference`, `footnote`, `study`, ...; ignoring
    /// case), whatever is set for [`Kind::Note`]
    pub fn note_type(mut self, note_type: &str, action: Action) -> Self {
        self.note_types.push((note_type.to_string(), action));
        self
    }

    /// A custom rule, checked before everything else; [`None`] passes the element on
    pub fn rule(
        mut self,
        rule: impl Fn(&Element) -> Option<Action> + Send + Sync + 'static,
    ) -> Self {
        self.rules.push(Arc::new(rule));
        self
    }

    /// What happens to this element
    pub fn action(&self, element: &Element) -> Action {
        if let Some(action) = self.rules.iter().rev().find_map(|rule| rule(element)) {
            return action;
        }
        if element.kind == Kind::Note
            && let Some(note_type) = element.note_type()
            && let Some((_, action)) = self
                .note_types
                .iter()
                .rev()
                .find(|(t, _)| t.eq_ignore_ascii_case(note_type))
        {
            return *action;
        }
        self.kinds
            .iter()
            .find(|(k, _)| *k == element.kind)
            .map_or(Action::Keep, |(_, a)| *a)
    }
}

impl std::fmt::Debug for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Filter")
            .field("kinds", &self.kinds)
            .field("note_types", &self.note_types)
            .field("rules", &self.rules.len())
            .finish()
    }
}
