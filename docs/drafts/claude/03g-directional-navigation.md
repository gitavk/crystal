# Step 3.7 — Directional Pane Focus Navigation

> `feat(app): add directional pane focus navigation`

## Goal

Implement spatial navigation between panes using Alt+arrow keys. Given the
current pane and a direction, find the best neighboring pane to focus.

## Files

| File | Action |
|------|--------|
| `crates/crystal-tui/src/pane.rs` | UPDATE — add `find_pane_in_direction()` |

## Algorithm

```rust
/// Given current pane Rect and all pane Rects, find the best pane
/// in the given direction (up/down/left/right).
///
/// Algorithm:
/// 1. Filter panes in the correct relative direction
///    - Right: candidate.x >= current.right()
///    - Left:  candidate.right() <= current.x
///    - Down:  candidate.y >= current.bottom()
///    - Up:    candidate.bottom() <= current.y
/// 2. Score by overlap on the perpendicular axis
///    - For left/right: overlap on Y axis
///    - For up/down: overlap on X axis
/// 3. Among candidates with overlap > 0, pick the closest
/// 4. If no overlap candidates, pick the nearest by center distance
pub fn find_pane_in_direction(
    current: (PaneId, Rect),
    all: &[(PaneId, Rect)],
    direction: Direction,
) -> Option<PaneId> { /* ... */ }
```

## Focus Cycling

In addition to directional navigation, Tab/Shift+Tab cycles focus through
all panes in depth-first order (using `PaneNode::leaf_ids()`):

- `FocusNextPane`: move to next leaf ID, wrap to first
- `FocusPrevPane`: move to previous leaf ID, wrap to last

## Edge Cases

- Single pane: all navigation is a no-op
- No pane in requested direction: focus stays on current pane
- Multiple candidates at same distance: prefer the one with most overlap
- Fullscreen mode: navigation disabled (only one visible pane)

## Tests

- Two panes side by side: Right from left → right pane, Left from right → left pane
- Two panes stacked: Down from top → bottom, Up from bottom → top
- Three panes (L-shape): navigation finds correct neighbor at each position
- No neighbor in direction → returns None (focus unchanged)
- Focus cycling wraps around correctly
- Single pane → all navigation returns None

## Demo

- [ ] Split into 2x2 grid
- [ ] Alt+Right/Left/Up/Down moves focus to correct neighbor
- [ ] Tab cycles through all panes in order
- [ ] Shift+Tab cycles in reverse
- [ ] Focus stays put when no neighbor exists in a direction
