# Step 3.2 — Pane Rendering Contract & Command Scoping

> `feat(tui): define pane rendering contract and command scoping`

## Goal

Define what every pane must do (the rendering contract) and how keyboard input
is split into global commands vs pane-local commands. This is the boundary that
makes the whole system scalable — plugins, terminals, and resource views all
follow the same rules.

## Files

| File | Action |
|------|--------|
| `crates/crystal-tui/src/pane.rs` | UPDATE — add pane trait/contract |
| `crates/crystal-app/src/command.rs` | NEW — Command + PaneCommand enums |
| `crates/crystal-app/src/app.rs` | UPDATE — command routing logic |

## Pane Rendering Contract

Every pane type must satisfy this contract:

```rust
/// What every pane must do:
/// - Render itself within a given Rect
/// - React to focus state (styling only — no behavior change)
/// - Accept PaneCommands and update internal state
/// - Never affect other panes or access global state directly
```

Pane responsibilities grow across stages:

| Version | Capabilities |
|---------|-------------|
| v0      | Render within Rect, react to focus |
| v1      | + receive PaneCommand, update internal state |
| v2      | + context-aware (cluster/namespace/pod via App Core) |

**Rules (non-negotiable):**
- Panes render in isolation — no pane knows about other panes
- Root UI composes pane output into the final frame
- Panes never access global input directly
- Help is a pane, not a modal overlay hack

## Command Enums

```rust
// crates/crystal-app/src/command.rs

/// Global commands — always handled by App Core first
pub enum Command {
    Quit,
    ShowHelp,
    FocusNextPane,
    FocusPrevPane,
    FocusDirection(Direction),
    SplitVertical,
    SplitHorizontal,
    ClosePane,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    GoToTab(usize),
    ToggleFullscreen,
    EnterMode(InputMode),
    ExitMode,
    Pane(PaneCommand),  // dispatched to focused pane
}

/// Pane-local commands — routed to focused pane only
pub enum PaneCommand {
    ScrollUp,
    ScrollDown,
    SelectNext,
    SelectPrev,
    Select,           // Enter/confirm
    Back,
    ToggleFollow,     // logs pane
    SendInput(String), // terminal/exec pane
    SearchInput(char),
    SearchConfirm,
    SearchClear,
}
```

## Command Routing Rules

**Strict. No exceptions.**

1. Global commands handled first by App Core
2. `Command::Pane(cmd)` routed to focused pane only
3. Unhandled pane commands are silently ignored
4. No bubbling. No fallback magic.

**Consequences:**
- Pane-local shortcuts are inactive when pane is unfocused
- Same key can map differently in global vs pane scope
- A key bound globally takes precedence over the same key at pane level

## Config Scopes

```toml
[keybindings.global]
quit = "q"
help = "?"
focus_next = "tab"
split_vertical = "alt+v"
split_horizontal = "alt+h"

[keybindings.pane]
scroll_up = "k"
scroll_down = "j"
select = "enter"
```

## Help Pane Integration

Help is a first-class pane that:
- Can be split alongside other panes
- Respects focus like any other pane
- Shows **global** shortcuts plus shortcuts for the **previously focused** pane
- Updates content when focus changes

## Tests

- Command routing delivers `PaneCommand` only to focused pane
- Unfocused pane receives no commands
- Global command takes precedence when same key is bound in both scopes
- Help pane content updates when focus changes

## Demo

- [ ] Press `j`/`k` in focused pane → list navigates
- [ ] Press `j`/`k` when pane is unfocused → nothing happens
- [ ] Press `q` (global) → works regardless of focus
- [ ] Open help pane → shows shortcuts for previously focused pane type
