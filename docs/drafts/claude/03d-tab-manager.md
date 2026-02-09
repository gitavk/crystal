# Step 3.4 — Tab Manager

> `feat(tui): implement tab manager with create/close/switch`

## Goal

Implement tab management. Each tab owns an independent pane tree and tracks
its own focused pane. The TabManager allocates globally unique pane IDs and
handles tab lifecycle.

## Files

| File | Action |
|------|--------|
| `crates/crystal-tui/src/tab.rs` | NEW — Tab + TabManager |

## Data Structures

```rust
// crates/crystal-tui/src/tab.rs

pub struct Tab {
    pub id: u32,
    pub name: String,
    pub root: PaneNode,
    pub focused_pane: PaneId,
}

pub struct TabManager {
    tabs: Vec<Tab>,
    active_tab: usize,
    next_pane_id: PaneId,
    next_tab_id: u32,
}

impl TabManager {
    pub fn new_tab(&mut self, name: &str, initial_view: ViewType) -> u32 { /* ... */ }
    pub fn close_tab(&mut self, id: u32) { /* ... */ }
    pub fn active(&self) -> &Tab { /* ... */ }
    pub fn active_mut(&mut self) -> &mut Tab { /* ... */ }
    pub fn switch_tab(&mut self, index: usize) { /* ... */ }
    pub fn next_tab(&mut self) { /* ... */ }
    pub fn prev_tab(&mut self) { /* ... */ }
    pub fn rename_tab(&mut self, id: u32, name: &str) { /* ... */ }
}
```

## Semantics

- App starts with one tab ("Main") containing a single pane
- New tab creates a fresh pane tree with one `Empty` leaf
- Closing the last tab is not allowed
- Tab switch preserves each tab's focus and pane state
- `next_pane_id` is global across all tabs — IDs never collide

## Pane ID Allocation

All pane IDs come from `TabManager.next_pane_id`:
- `split()` requests a new ID from TabManager before inserting
- This guarantees uniqueness across tabs
- IDs are never reused (monotonically increasing)

## Tests

- `new_tab()` creates tab with correct initial view
- `close_tab()` on last tab returns error / is no-op
- `switch_tab()` changes active tab, preserves previous tab's state
- `next_tab()` / `prev_tab()` wrap around
- Pane IDs are unique across tabs after multiple splits in different tabs

## Demo

- [ ] App starts with one tab "Main"
- [ ] Create new tab → tab bar shows two tabs
- [ ] Switch between tabs → each has independent pane layout
- [ ] Close a tab → removed from tab bar, previous tab activates
- [ ] Cannot close the last remaining tab
