# Step 5.5 — Terminal View (Pure Renderer)

> `feat(tui): implement terminal view as pure renderer over TerminalManager`

## Goal

Implement `TerminalView` — the TUI pane that displays a terminal session. This
is a **pure renderer**: it does not own a `PtySession` or `VtParser`. It receives
screen state from App Core's `TerminalManager` via `RenderContext` and draws it.

## Files

| File | Action |
|------|--------|
| `crates/crystal-tui/src/views/terminal_view.rs` | NEW — terminal pane view |

## Data Structures

```rust
// crates/crystal-tui/src/views/terminal_view.rs

pub struct TerminalView {
    session_id: SessionId,
    scrollback_offset: usize,
    title: String,
}

impl TerminalView {
    pub fn new(session_id: SessionId, title: String) -> Self { /* ... */ }

    /// Render the terminal screen state.
    /// `screen` is borrowed from TerminalManager via RenderContext.
    pub fn render(
        &self,
        screen: &vt100::Screen,
        frame: &mut Frame,
        area: Rect,
        focused: bool,
    ) {
        // 1. Draw title bar: "[crystal:cluster/ns] bash" or "[exec:pod/container]"
        // 2. Calculate content area (area minus title bar)
        // 3. Apply scrollback offset if user has scrolled up
        // 4. Call render_terminal_screen(screen, content_area, frame)
        // 5. If focused + Insert mode: show cursor
        // 6. If not focused: dim the output slightly
    }

    pub fn scroll_up(&mut self, lines: usize) { /* ... */ }
    pub fn scroll_down(&mut self, lines: usize) { /* ... */ }
    pub fn scroll_to_bottom(&mut self) { /* ... */ }
}
```

## Rendering Contract

This view satisfies the pane rendering contract from Stage 3:

| Capability | How |
|-----------|-----|
| Render within Rect | Uses `area` parameter, respects bounds |
| React to focus state | Dims content when unfocused, shows cursor when focused |
| Accept PaneCommands | Scroll up/down, scroll to bottom |
| No global state access | Reads screen from `RenderContext`, nothing else |

## Title Bar Format

- Shell: `[crystal:minikube/default] /bin/bash`
- Exec: `[exec:my-pod/nginx] /bin/sh`
- Exited: `[exited:1] /bin/bash — Press Enter to restart`

## Tests

- Renders title bar with correct session info
- Focused pane shows cursor at correct position
- Unfocused pane does not show cursor
- `scroll_up()` increases scrollback offset
- `scroll_down()` decreases scrollback offset (clamped at 0)
- `scroll_to_bottom()` resets offset to 0

## Demo

- [ ] Open a terminal pane, verify title bar shows cluster/namespace
- [ ] Type commands, see output rendered correctly
- [ ] Navigate away — terminal dims, cursor disappears
- [ ] Navigate back — terminal un-dims, cursor reappears
- [ ] Scroll up in Normal mode to see scrollback buffer
