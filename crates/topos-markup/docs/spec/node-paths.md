# Node paths

`NodePath(pub Vec<usize>)` is a node's position: the index of each node on the way down from the
top-level list. `[2, 0]` is the first child of the third top-level node. The indexes count every
child, text nodes and elements alike, of the tree as parsed (filters never change paths).

| Operation | Behaviour |
|---|---|
| `Display` | `/` before each index: `[2, 0]` → `/2/0`, `[5]` → `/5`, `[]` → the empty string |
| `resolve(nodes)` | Follows the indexes: the first into `nodes`, each later one into the `children` of the element reached. `None` when an index is out of range, when an index would go into a text node, or for the empty path |
| `ancestors()` | The paths of the containing elements, outermost first: for `[2, 0, 1]`, `[2]` then `[2, 0]`. Empty for paths of length 0 or 1. It is an iterator borrowing the path |
| `Default` | the empty path |
| ordering (`Ord`) | lexicographic on the indexes; a proper prefix is less: `[1] < [1, 0] < [1, 2] < [2]`, which is document order |
| `Hash`, `Eq` | on the indexes |
| serde (feature) | a newtype struct around the list |

Paths appear in `Point::path` (a text node), `Span::path` (an element; the empty path for words of
Jesus continuing from an earlier entry), `TaggedReference::path` (an element), and `VerseSpan`
(`marker` is an element; `start` and `end` are places, which may be one past the last child and
so may not resolve).
