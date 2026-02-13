# Step 5.2 — VT100 Screen to Ratatui Renderer

> `feat(terminal): implement VT100 screen to ratatui renderer`

## Goal

Convert `vt100::Screen` state into ratatui widgets for display. This is the
bridge between the terminal emulation layer and the TUI — a pure function that
takes screen state and produces styled spans.

## Files

| File | Action |
|------|--------|
| `crates/crystal-terminal/src/vt.rs` | NEW — VT parser wrapper |
| `crates/crystal-terminal/src/renderer.rs` | NEW — screen → ratatui conversion |

## VT Parser Wrapper

```rust
// crates/crystal-terminal/src/vt.rs

pub struct VtParser {
    parser: vt100::Parser,
}

impl VtParser {
    pub fn new(rows: u16, cols: u16) -> Self { /* ... */ }

    pub fn process(&mut self, bytes: &[u8]) { /* ... */ }

    pub fn screen(&self) -> &vt100::Screen { /* ... */ }

    pub fn resize(&mut self, rows: u16, cols: u16) { /* ... */ }
}
```

## Renderer

```rust
// crates/crystal-terminal/src/renderer.rs

use ratatui::prelude::*;
use vt100::Screen;

/// Convert vt100::Screen state to ratatui drawable content.
/// This is a pure function — takes screen state, produces widgets.
pub fn render_terminal_screen(
    screen: &Screen,
    area: Rect,
    frame: &mut Frame,
) {
    // For each visible row in the screen:
    //   1. Get the row's cells via screen.rows()
    //   2. Convert each cell's attributes to ratatui::style::Style:
    //      - fg/bg color (8-color, 256-color, truecolor)
    //      - bold, italic, underline, inverse
    //   3. Build Vec<Span> for the row
    //   4. Render as Line widgets
    //
    // Handle:
    //   - Cursor position and style (block/underline/bar)
    //   - Scrollback buffer offset
    //   - 256-color and truecolor mapping
}

/// Map vt100::Color to ratatui::style::Color
fn convert_color(color: vt100::Color) -> Color {
    // vt100::Color::Default → Color::Reset
    // vt100::Color::Idx(n) → Color::Indexed(n)
    // vt100::Color::Rgb(r, g, b) → Color::Rgb(r, g, b)
}
```

## Color Mapping

| vt100 | ratatui |
|-------|---------|
| `Color::Default` | `Color::Reset` |
| `Color::Idx(0..7)` | `Color::Indexed(n)` |
| `Color::Idx(8..255)` | `Color::Indexed(n)` |
| `Color::Rgb(r,g,b)` | `Color::Rgb(r,g,b)` |

## Attribute Mapping

| vt100 attribute | ratatui modifier |
|-----------------|-----------------|
| `bold` | `Modifier::BOLD` |
| `italic` | `Modifier::ITALIC` |
| `underline` | `Modifier::UNDERLINED` |
| `inverse` | `Modifier::REVERSED` |

## Cursor Rendering

- If the screen has a visible cursor, render it as a highlighted cell
- Cursor position: `screen.cursor_position()` returns `(row, col)`
- In Normal mode (pane not in Insert), cursor is hidden

## Tests

- Empty screen renders all blank spans
- Single line of text at row 0 produces correct `Span` content
- Bold + red foreground maps to `Style::new().fg(Color::Red).add_modifier(Modifier::BOLD)`
- 256-color index maps correctly
- Truecolor RGB maps to `Color::Rgb`
- Inverse attribute swaps fg/bg
- Cursor position renders as highlighted cell at correct location

## Demo

- [ ] Feed `"\x1b[31mHello\x1b[0m World"` → renders "Hello" in red, " World" in default
- [ ] Feed bold + underline escape → correct modifiers applied
- [ ] Cursor at (0, 5) renders highlight at that position
