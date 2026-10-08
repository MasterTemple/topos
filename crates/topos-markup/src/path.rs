//! Positions in a parsed tree.

use std::fmt;

use crate::Node;

/**
A node's position in a tree: the index of each node on the way down from the top-level list
- `[2, 0]` is the first child of the third top-level node
- Displayed as `/2/0`
*/
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NodePath(pub Vec<usize>);

impl NodePath {
    /// The node at this path
    pub fn resolve<'a>(&self, nodes: &'a [Node]) -> Option<&'a Node> {
        let (first, rest) = self.0.split_first()?;
        let mut node = nodes.get(*first)?;
        for &i in rest {
            node = match node {
                Node::Element(e) => e.children.get(i)?,
                Node::Text(_) => return None,
            };
        }
        Some(node)
    }

    /// The paths of the elements containing this node, outermost first
    pub fn ancestors(&self) -> impl Iterator<Item = NodePath> + '_ {
        (1..self.0.len()).map(|n| NodePath(self.0[..n].to_vec()))
    }

    pub(crate) fn child(&self, i: usize) -> NodePath {
        let mut path = self.0.clone();
        path.push(i);
        NodePath(path)
    }
}

impl fmt::Display for NodePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in &self.0 {
            write!(f, "/{i}")?;
        }
        Ok(())
    }
}
