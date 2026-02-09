# Step 3.3 — Zellij-Style Border Rendering

> `feat(tui): add zellij-style border rendering between panes`

## Goal

Render borders between panes using box-drawing characters. The focused pane
gets an accent-colored border; unfocused panes get dimmed borders. This gives
immediate visual feedback about which pane is active.

## Files

| File | Action |
|------|--------|
| `crates/crystal-tui/src/borders.rs` | NEW — border rendering logic |
| `crates/crystal-tui/src/theme.rs` | UPDATE — add border color variants |

## Visual Design

```
┌──────────┬──────────┐
│  Pane 1  │  Pane 2  │  ← active pane border in accent color
│          │          │
├──────────┼──────────┤
│  Pane 3             │
└─────────────────────┘
```

## Implementation

```rust
// crates/crystal-tui/src/borders.rs

/// Render borders between panes using box-drawing characters.
/// Active pane gets highlighted borders (like zellij's green border).
///
/// UX rules:
/// - Focused pane: visible border in accent color
/// - Unfocused panes: dimmed border
/// - Borders use box-drawing characters (─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼)
/// - Shared edges between adjacent panes merge into single lines
pub fn render_pane_borders(
    frame: &mut Frame,
    layout: &[(PaneId, Rect)],
    focused: PaneId,
    theme: &Theme,
) { /* ... */ }
```

## UX Rules

- Focused pane: border in theme accent color (Catppuccin-inspired)
- Unfocused panes: border in dimmed/surface color
- Single-pane mode: no borders (full screen usage)
- Border width is always 1 cell — pane Rects are inset by 1 from their allocated area

## Edge Cases

- Adjacent panes share border lines (no double-thick borders)
- Corner characters (┼, ├, ┤, etc.) chosen based on neighbor topology
- Fullscreen pane: borders hidden entirely
- Terminal resize: borders recalculated from pane tree layout

## Tests

- Single pane → no borders rendered
- Two panes side by side → shared vertical border
- Three panes (L-shape) → correct corner characters at intersection
- Focused pane border uses accent color
- Unfocused pane border uses dimmed color
- Borders render correctly at various terminal sizes (80x24, 120x40, 200x60)

## Demo

- [ ] Split into two panes → shared border appears
- [ ] Focus left pane → left border is accent-colored, right is dimmed
- [ ] Focus right pane → colors swap
- [ ] Toggle fullscreen → borders disappear
