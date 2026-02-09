# Step 3.6 — Config-Driven Keybinding Dispatcher

> `feat(app): implement config-driven keybinding dispatcher`

## Goal

Replace all hardcoded key handling with a config-driven dispatcher. Keybindings
are defined in TOML, loaded at startup, and mapped to Commands through a single
`KeybindingDispatcher`. No keyboard strings exist outside config.

## Files

| File | Action |
|------|--------|
| `crates/crystal-app/src/keybindings.rs` | NEW — InputMode, KeybindingDispatcher |
| `crates/crystal-config/src/lib.rs` | UPDATE — keybinding TOML parsing |

## Input Modes

```rust
// crates/crystal-app/src/keybindings.rs

pub enum InputMode {
    Normal,     // default: navigate resources, basic shortcuts
    Pane,       // Alt+p: pane management (split, resize, close)
    Tab,        // Alt+t: tab management
    Search,     // /: filter/search within view
    Command,    // :: command palette
    Insert,     // terminal/exec pane: all keys forwarded to PTY
}
```

## Dispatcher

```rust
pub struct KeybindingDispatcher {
    mode: InputMode,
    global_bindings: HashMap<KeyEvent, Command>,
    pane_bindings: HashMap<KeyEvent, PaneCommand>,
}

impl KeybindingDispatcher {
    /// Returns a Command based on current mode + key event.
    /// Global bindings always checked first.
    /// In Insert mode, only global shortcuts are checked;
    /// all other keys become SendInput.
    pub fn dispatch(&self, key: KeyEvent) -> Option<Command> { /* ... */ }

    pub fn set_mode(&mut self, mode: InputMode) { /* ... */ }
    pub fn mode(&self) -> &InputMode { /* ... */ }
}
```

## TOML Config

```toml
[keybindings.global]
quit = "q"
help = "?"
focus_next = "tab"
focus_prev = "shift+tab"
split_vertical = "alt+v"
split_horizontal = "alt+h"
close_pane = "alt+w"
new_tab = "alt+t"
toggle_fullscreen = "alt+f"
focus_up = "alt+up"
focus_down = "alt+down"
focus_left = "alt+left"
focus_right = "alt+right"
resize_grow = "alt+]"
resize_shrink = "alt+["

[keybindings.pane]
scroll_up = "k"
scroll_down = "j"
select_next = "j"
select_prev = "k"
select = "enter"
back = "esc"
toggle_follow = "f"
```

## Config Loading Order

1. Load default config embedded in binary
2. Override with user config (`~/.config/crystal/config.toml`)
3. Invalid bindings are ignored with a warning log
4. App must still start — never crash on config errors

## Command Name Mapping

One place only — the mapping from config string to Command enum:

```
"quit"              → Command::Quit
"help"              → Command::ShowHelp
"focus_next"        → Command::FocusNextPane
"split_vertical"    → Command::SplitVertical
...
```

## Key String Parsing

Key strings in config map to crossterm `KeyEvent`:

| Config string | KeyEvent |
|---------------|----------|
| `"q"` | KeyCode::Char('q'), no modifiers |
| `"alt+v"` | KeyCode::Char('v'), Alt modifier |
| `"shift+tab"` | KeyCode::BackTab |
| `"enter"` | KeyCode::Enter |
| `"alt+up"` | KeyCode::Up, Alt modifier |
| `"alt+["` | KeyCode::Char('['), Alt modifier |

## Default Keybindings Table

| Key           | Mode   | Action              |
|---------------|--------|---------------------|
| Alt+n         | Normal | New pane (vertical)  |
| Alt+h         | Normal | Split horizontal     |
| Alt+v         | Normal | Split vertical       |
| Alt+←→↑↓     | Normal | Focus pane direction |
| Alt+[ / Alt+] | Normal | Resize focused pane  |
| Alt+f         | Normal | Toggle fullscreen    |
| Alt+w         | Normal | Close pane           |
| Alt+t         | Normal | New tab              |
| 1-9           | Normal | Go to tab N          |
| Tab           | Normal | Next tab             |
| Shift+Tab     | Normal | Previous tab         |
| /             | Normal | Enter search mode    |
| Esc           | Any    | Return to Normal     |
| j/k or ↑/↓   | Normal | Navigate list        |
| Enter         | Normal | Select/open          |
| q             | Normal | Quit (with confirm)  |

## Tests

- Dispatcher maps configured keys to correct Commands
- Dispatcher respects global-over-pane precedence
- Config loading merges defaults + user overrides correctly
- Invalid config key string is skipped with warning
- Missing config file → defaults still work
- Mode switch changes which bindings are active
- Insert mode forwards non-global keys as SendInput

## Demo

- [ ] Change a keybinding in config → behavior changes without code change
- [ ] Remove a keybinding from config → that shortcut is disabled
- [ ] Help screen reflects active (possibly customized) keybindings
