# Step 4.4 — Commands and Input Modes

> `feat(app): extend commands and input modes for resource actions`

## Goal

Extend the existing `Command` enum, `PaneCommand` enum, and `InputMode` enum
with all the variants needed for resource views. This step defines the
vocabulary of actions — no implementation yet, just the types that steps
4.5–4.10 will handle.

## Files

| File | Action |
|------|--------|
| `crates/crystal-app/src/command.rs` | UPDATE — add resource action commands |
| `crates/crystal-tui/src/pane.rs` | UPDATE — add PaneCommand variants |
| `crates/crystal-app/src/keybindings.rs` | UPDATE — add InputMode variants, new bindings |
| `crates/crystal-config/src/lib.rs` | UPDATE — add resource keybinding config section |

## Existing Command Enum

```rust
// crates/crystal-app/src/command.rs — current state
pub enum Command {
    Quit, ShowHelp,
    FocusNextPane, FocusPrevPane, FocusDirection(Direction),
    SplitVertical, SplitHorizontal, ClosePane,
    NewTab, CloseTab, NextTab, PrevTab, GoToTab(usize),
    ToggleFullscreen, ResizeGrow, ResizeShrink,
    EnterMode(InputMode), ExitMode,
    NamespaceConfirm, NamespaceInput(char), NamespaceBackspace,
    Pane(PaneCommand),
}
```

## Extended Command Enum

```rust
pub enum Command {
    // --- existing variants (unchanged) ---
    Quit, ShowHelp,
    FocusNextPane, FocusPrevPane, FocusDirection(Direction),
    SplitVertical, SplitHorizontal, ClosePane,
    NewTab, CloseTab, NextTab, PrevTab, GoToTab(usize),
    ToggleFullscreen, ResizeGrow, ResizeShrink,
    EnterMode(InputMode), ExitMode,
    NamespaceConfirm, NamespaceInput(char), NamespaceBackspace,
    Pane(PaneCommand),

    // --- NEW: resource actions ---
    ViewYaml,                    // 'y' — open YAML view in split pane
    ViewDescribe,                // 'd' — open describe view in split pane
    DeleteResource,              // Ctrl+d — show confirmation dialog
    ScaleResource,               // 'S' — prompt for replica count
    RestartRollout,              // 'R' — deployments only
    ViewLogs,                    // 'l' — pods only
    ExecInto,                    // 'e' — pods only
    ToggleAllNamespaces,         // 'a' — toggle all-ns scope for current view

    // --- NEW: resource switcher ---
    EnterResourceSwitcher,       // ':' — open command palette
    ResourceSwitcherInput(char), // typing in the palette
    ResourceSwitcherBackspace,   // backspace in the palette
    ResourceSwitcherConfirm,     // Enter — switch to selected resource

    // --- NEW: confirmation dialog ---
    ConfirmAction,               // 'y' — confirm destructive action
    DenyAction,                  // 'n' / Esc — cancel destructive action

    // --- NEW: sort ---
    SortByColumn,                // 's' — cycle sort column
}
```

## Extended PaneCommand

```rust
// crates/crystal-tui/src/pane.rs — extend existing enum
pub enum PaneCommand {
    // --- existing ---
    ScrollUp, ScrollDown,
    SelectNext, SelectPrev,
    Select, Back, ToggleFollow,
    SendInput(String),
    SearchInput(char), SearchConfirm, SearchClear,

    // --- NEW ---
    Filter(String),       // set filter text (fuzzy match by name)
    ClearFilter,          // clear active filter
    SortByColumn(usize),  // sort by specific column index
    ToggleSortOrder,      // flip ascending/descending
}
```

## Extended InputMode

```rust
// crates/crystal-app/src/keybindings.rs — extend existing enum
pub enum InputMode {
    // --- existing ---
    Normal,
    Pane,
    Tab,
    Search,
    Command,
    Insert,
    NamespaceSelector,

    // --- NEW ---
    ResourceSwitcher,    // ':' command palette — typing filters resource types
    ConfirmDialog,       // 'y'/'n' only — modal confirmation
    FilterInput,         // '/' in list view — typing filters rows
}
```

## Keybinding Config Extension

```toml
# Added to defaults.toml

[keybindings.resource]
view_yaml = "y"
view_describe = "d"
delete = "ctrl+d"
scale = "shift+s"
restart = "shift+r"
view_logs = "l"
exec = "e"
toggle_all_namespaces = "a"
sort = "s"
filter = "/"
resource_switcher = ":"
```

## Dispatcher Routing Rules

The dispatcher needs to know when resource keybindings are active:

| Mode | Active Bindings | Resource Bindings Active? |
|------|----------------|--------------------------|
| Normal | global | Yes (if focused pane is ResourceList) |
| Pane | global + pane | Yes (if focused pane is ResourceList) |
| FilterInput | filter-specific only | No |
| ResourceSwitcher | switcher-specific only | No |
| ConfirmDialog | y/n/Esc only | No |
| Insert | global only, rest forwarded | No |

Resource keybindings are context-sensitive: they are only active when the
focused pane's `ViewType` is `ResourceList(_)` or `Detail(_, _)`. The
dispatcher checks the focused pane's view type before matching resource
bindings.

## Command Name Mapping (for config)

```
"view_yaml"              → Command::ViewYaml
"view_describe"          → Command::ViewDescribe
"delete"                 → Command::DeleteResource
"scale"                  → Command::ScaleResource
"restart"                → Command::RestartRollout
"view_logs"              → Command::ViewLogs
"exec"                   → Command::ExecInto
"toggle_all_namespaces"  → Command::ToggleAllNamespaces
"sort"                   → Command::SortByColumn
"filter"                 → Command::EnterMode(InputMode::FilterInput)
"resource_switcher"      → Command::EnterResourceSwitcher
```

## Tests

- Each new Command variant can be constructed (compile-time check)
- New keybinding config strings map to correct Command variants
- Resource keybindings are inactive when focused pane is not a resource view
- ResourceSwitcher mode only accepts input/backspace/confirm/Esc
- ConfirmDialog mode only accepts y/n/Esc
- FilterInput mode forwards characters and responds to Esc/Enter
