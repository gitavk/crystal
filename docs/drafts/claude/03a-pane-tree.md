# Step 3.1 — Pane Tree Data Structure

> `feat(tui): implement pane tree data structure with split/close/resize`

## Goal

Create the binary tree that represents the pane layout. Every future pane
operation (split, close, resize, focus, render) depends on this structure.

## Files

| File | Action |
|------|--------|
| `crates/crystal-tui/src/pane.rs` | NEW — PaneNode, PaneId, SplitDirection, ViewType |
| `crates/crystal-tui/src/layout.rs` | REWRITE — use PaneNode for rect calculation |

## Data Structures

```rust
// crates/crystal-tui/src/pane.rs

pub type PaneId = u32;

/// A node in the pane layout tree
pub enum PaneNode {
    /// A leaf pane that hosts a view
    Leaf {
        id: PaneId,
        view: ViewType,
    },
    /// A split container holding two children
    Split {
        direction: SplitDirection,
        ratio: f32,          // 0.0..1.0, position of the divider
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

pub enum SplitDirection {
    Horizontal, // top/bottom
    Vertical,   // left/right
}

pub enum ViewType {
    ResourceList(ResourceKind),
    Detail(ResourceKind, String),  // kind + resource name
    Terminal,
    Logs(String),   // pod name
    Exec(String),   // pod name
    Help,
    Empty,
    Plugin(String), // plugin name
}
```

## Operations

```rust
impl PaneNode {
    /// Split the pane with given ID.
    /// The target leaf becomes a Split node; the original leaf becomes `first`,
    /// a new leaf with `new_view` becomes `second`.
    pub fn split(
        &mut self,
        target: PaneId,
        direction: SplitDirection,
        new_view: ViewType,
    ) -> PaneId { /* ... */ }

    /// Close a pane, promoting its sibling to take the parent's place.
    pub fn close(&mut self, target: PaneId) -> bool { /* ... */ }

    /// Resize: adjust the ratio of the split containing the target pane.
    /// delta is clamped so ratio stays within 0.1..0.9.
    pub fn resize(&mut self, target: PaneId, delta: f32) { /* ... */ }

    /// Get all leaf pane IDs in depth-first order (for focus cycling).
    pub fn leaf_ids(&self) -> Vec<PaneId> { /* ... */ }

    /// Calculate the Rect for each pane given the total area.
    /// Splits divide the area according to direction and ratio.
    pub fn layout(&self, area: Rect) -> Vec<(PaneId, Rect)> { /* ... */ }

    /// Find pane by ID.
    pub fn find(&self, id: PaneId) -> Option<&PaneNode> { /* ... */ }
}
```

## Tree Semantics

- A single pane is a `Leaf` at the root
- Splitting a leaf replaces it with a `Split` node containing two `Leaf` children
- Closing a leaf promotes its sibling to replace the parent `Split`
- Closing the last pane is not allowed (always at least one leaf)
- `ratio` defaults to 0.5 on split, adjustable via resize

Example after two splits:

```
        Split(V, 0.5)
       /             \
  Leaf(pods)    Split(H, 0.5)
                /            \
           Leaf(logs)    Leaf(help)
```

## Tests

- `split()` on a single leaf produces correct Split with two children
- `split()` on a nested leaf targets the right node
- `close()` promotes sibling correctly
- `close()` on last pane returns false
- `layout()` divides area correctly for vertical split
- `layout()` divides area correctly for horizontal split
- `layout()` handles nested splits
- `leaf_ids()` returns depth-first order
- `resize()` clamps ratio to valid range

## Demo

- [ ] Create a single-leaf tree, call `layout()` → full area returned
- [ ] Split it vertically → two rects side by side, each ~50% width
- [ ] Split right pane horizontally → three rects in correct positions
- [ ] Close one pane → sibling expands to fill parent area
