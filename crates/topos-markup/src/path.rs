//! Positions in a parsed tree.

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
        for index in rest {
            node = node.as_element()?.children.get(*index)?;
        }
        Some(node)
    }

    /// The paths of the elements containing this node, outermost first
    pub fn ancestors(&self) -> impl Iterator<Item = NodePath> + '_ {
        let len = self.0.len();
        (1..len).map(move |n| NodePath(self.0.get(..n).unwrap_or_default().to_vec()))
    }
}

impl std::fmt::Display for NodePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for index in &self.0 {
            write!(f, "/{index}")?;
        }
        Ok(())
    }
}
