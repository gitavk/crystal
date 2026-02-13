# Step 5.9 — Insert Mode for Terminal Input

> `feat(app): implement Insert mode for terminal input passthrough`

## Goal

Implement the Insert/Normal mode toggle that allows terminal and exec panes to
capture all keyboard input. In Insert mode, keystrokes are forwarded as raw
bytes to the active terminal session. Esc returns to Normal mode where pane
navigation and global shortcuts work.

## Files

| File | Action |
|------|--------|
| `crates/crystal-app/src/app.rs` | UPDATE — mode-aware key routing |
| `crates/crystal-tui/src/widgets/status_bar.rs` | UPDATE — show mode indicator |

## Input Mode Enum

```rust
// This likely already exists from Stage 3 (keybinding dispatcher).
// Stage 5 adds the Insert variant and routing logic.

pub enum InputMode {
    Normal,
    Insert,    // NEW — all keys → terminal/exec
    Command,   // existing — command palette
    Filter,    // existing — filter input
}
```

## Key Routing Rules

```
┌─────────────────────────────────────────────────────┐
│                   Key Event                         │
│                      ↓                              │
│              ┌───────────────┐                      │
│              │ InputMode?    │                      │
│              └───────┬───────┘                      │
│         ┌────────────┼────────────┐                 │
│         ↓            ↓            ↓                 │
│      Normal       Insert       Command              │
│         ↓            ↓            ↓                 │
│   KeybindingMap  Esc? ─→ Normal  Command palette    │
│         ↓         ↓ No                              │
│   Command enum   Convert key → bytes                │
│                  Command::TerminalInput              │
└─────────────────────────────────────────────────────┘
```

## Insert Mode Entry/Exit

**Enter Insert mode:**
- Focus a terminal/exec pane + press `i` or `Enter`
- Opening a new terminal pane auto-enters Insert mode
- Opening an exec session auto-enters Insert mode

**Exit Insert mode:**
- Press `Esc` → returns to Normal mode
- Terminal process exits → auto-exit to Normal mode
- Closing the terminal pane → auto-exit to Normal mode

## Key-to-Bytes Conversion

```rust
/// Convert a crossterm KeyEvent to raw terminal bytes
fn key_to_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    // Printable characters → UTF-8 bytes
    // Enter → \r
    // Backspace → \x7f
    // Tab → \t
    // Ctrl+c → \x03
    // Ctrl+d → \x04
    // Ctrl+z → \x1a
    // Arrow Up → \x1b[A
    // Arrow Down → \x1b[B
    // Arrow Right → \x1b[C
    // Arrow Left → \x1b[D
    // Home → \x1b[H
    // End → \x1b[F
    // Page Up → \x1b[5~
    // Page Down → \x1b[6~
    // Delete → \x1b[3~
    // F1-F12 → appropriate escape sequences
}
```

## Status Bar in Insert Mode

```
INSERT │ Esc → Normal mode │ [crystal:minikube/default] bash
```

The mode indicator is styled prominently (e.g., bold inverse) to make it
immediately obvious that keyboard input is going to the terminal.

## Normal Mode in Terminal Pane

When a terminal pane is focused but in Normal mode:

| Key | Action |
|-----|--------|
| `i` / `Enter` | Enter Insert mode |
| `[` / `]` | Scroll up/down in scrollback |
| `g` / `G` | Scroll to top/bottom of scrollback |
| `y` | Yank (copy) selected text |
| Standard pane nav | Alt+arrows, splits, close, etc. |

## Tests

- In Normal mode, `i` transitions to Insert mode
- In Insert mode, `Esc` transitions to Normal mode
- In Insert mode, printable key → `Command::TerminalInput` with correct bytes
- In Insert mode, Ctrl+c → `Command::TerminalInput` with `\x03`
- In Insert mode, arrow keys → correct escape sequences
- In Normal mode, arrow keys → pane navigation (not terminal input)
- Terminal pane focus + auto-enter: Insert mode activates
- Terminal exit → auto-exit to Normal mode
- Status bar shows "INSERT" when in Insert mode
- Status bar shows "NORMAL" when in Normal mode

## Demo

- [ ] Open terminal pane → auto-enters Insert mode, status bar shows INSERT
- [ ] Type commands → keystrokes go to terminal
- [ ] Press Esc → Normal mode, status bar updates
- [ ] Press `i` → back to Insert mode
- [ ] In Normal mode, Alt+arrow → navigate to another pane
- [ ] Ctrl+c in Insert mode → sends interrupt to terminal (not to app)
- [ ] Arrow keys in Insert mode → move cursor in terminal shell
