# Step 3.5 — Tab Bar Widget & Status Bar

> `feat(tui): add tab bar widget and status bar with mode hints`

## Goal

Add the two chrome bars that frame the pane area: a tab bar at the top and a
zellij-style status bar at the bottom. Together they provide context (which tab,
which cluster) and discoverability (what keys do what in the current mode).

## Files

| File | Action |
|------|--------|
| `crates/crystal-tui/src/widgets/tab_bar.rs` | NEW — tab strip widget |
| `crates/crystal-tui/src/widgets/status_bar.rs` | NEW — mode-dependent hint bar |

## Tab Bar

```rust
// crates/crystal-tui/src/widgets/tab_bar.rs

/// Top bar showing tabs:
///  [1] Pods │ [2] Services │ [3] Terminal
///            ^^^^^^^^^^^^^ active tab is highlighted
pub struct TabBarWidget<'a> {
    tabs: &'a [Tab],
    active: usize,
}
```

**Layout:** single row at the top of the terminal.

**UX:**
- Each tab shows `[N] Name` where N is 1-indexed
- Active tab is highlighted with accent color
- Inactive tabs use dimmed color
- Tab bar scrolls horizontally if too many tabs to fit

## Status Bar

```rust
// crates/crystal-tui/src/widgets/status_bar.rs

/// Bottom status bar showing mode-dependent keybinding hints.
///
///  NORMAL │ <Alt+n> New Pane │ <Alt+h/v> Split │ <Alt+←→↑↓> Navigate │ ...
///
/// In different modes, different hints appear:
/// - NORMAL: navigation, splitting, tab switching
/// - PANE:   resize, move, close, fullscreen
/// - SEARCH: search controls
///
/// Right side shows cluster context:
///  ... │ minikube / default
pub struct StatusBar {
    pub mode: InputMode,
    pub cluster_info: String,
    pub namespace: String,
}
```

**Layout:** single row at the bottom of the terminal.

**UX rules:**
- Hints are derived from active keybindings config, not hardcoded
- When mode changes, hints update immediately
- Current mode label shown at left in distinct color
- Cluster + namespace shown at right
- Clear indicator when no cluster is available

## Screen Layout

```
┌─────────────────────────────────────────┐
│ [1] Pods │ [2] Services │ [3] Terminal  │  ← tab_bar (1 row)
├─────────────────────────────────────────┤
│                                         │
│            Pane area                    │  ← remaining space
│                                         │
├─────────────────────────────────────────┤
│ NORMAL │ <Alt+v> Split │ ...  │ ctx/ns  │  ← status_bar (1 row)
└─────────────────────────────────────────┘
```

## Tests

- Tab bar renders correct number of tabs with correct names
- Active tab is visually distinct
- Status bar shows correct hints for Normal mode
- Status bar shows correct hints for Pane mode
- Status bar shows cluster info on right side
- Status bar shows "No cluster" when disconnected

## Demo

- [ ] Tab bar visible at top with active tab highlighted
- [ ] Switch tabs → tab bar updates
- [ ] Status bar shows mode-specific keybinding hints
- [ ] Change mode → status bar hints update
- [ ] Cluster + namespace visible on the right
