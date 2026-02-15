# Step 6.3 — Keybinding Config Types & Dispatcher Refactor

> `feat(app): refactor KeybindingDispatcher for 5 groups with mutate confirmation`

## Goal

Move keybinding config types into crystal-config (data shape) and refactor the
`KeybindingDispatcher` in crystal-app from 3 binding maps to 5. The dispatcher
now returns `(Command, bool)` where the bool indicates whether the command
requires confirmation (true for anything from the `mutate` group).

## Files

| File | Action |
|------|--------|
| `crates/crystal-config/src/keybindings.rs` | NEW — KeybindingsConfig with 5 groups + validation |
| `crates/crystal-app/src/keybindings.rs` | REFACTOR — 5 maps, (Command, bool) dispatch return |
| `crates/crystal-app/src/app.rs` | UPDATE — handle confirmation bool from dispatch |

## Config Types (crystal-config)

```rust
// crates/crystal-config/src/keybindings.rs

/// 5 semantic groups, each with a default modifier convention.
/// Dispatch priority: global → mutate → browse → navigation → tui
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct KeybindingsConfig {
    pub navigation: HashMap<String, String>,  // bare keys — cursor movement
    pub browse: HashMap<String, String>,      // bare keys — read-only inspection
    pub tui: HashMap<String, String>,         // alt+ — layout management
    pub global: HashMap<String, String>,      // ctrl+ — app-wide actions
    pub mutate: HashMap<String, String>,      // ctrl+alt+ — destructive ops
}

/// Validate all key strings in a KeybindingsConfig are parseable.
/// Returns Vec of (group, command_name, error) for invalid entries.
pub fn validate_keybindings(config: &KeybindingsConfig) -> Vec<(String, String, String)> {
    // Iterate all 5 groups
    // Attempt parse_key_string() on every value
    // Collect errors with group name for context
}

/// Check for key collisions across groups.
/// Returns Vec of (key_string, group1, group2) for conflicts.
pub fn check_collisions(config: &KeybindingsConfig) -> Vec<(String, String, String)> {
    // Parse all keys, check for duplicates across groups
}
```

The config crate only owns the data shape. It does NOT depend on crossterm or
the `Command` enum.

## Dispatcher Changes (crystal-app)

```rust
// crates/crystal-app/src/keybindings.rs (updated)

pub struct KeybindingDispatcher {
    mode: InputMode,
    navigation_bindings: HashMap<KeyEvent, Command>,
    browse_bindings: HashMap<KeyEvent, Command>,
    tui_bindings: HashMap<KeyEvent, Command>,
    global_bindings: HashMap<KeyEvent, Command>,
    mutate_bindings: HashMap<KeyEvent, Command>,  // always confirm
}

impl KeybindingDispatcher {
    pub fn from_config(config: &KeybindingsConfig) -> Self {
        // Parse each group's HashMap<String, String> into HashMap<KeyEvent, Command>
        // command_name string → Command enum variant mapping
        // key string → KeyEvent via existing parse_key_string()
    }

    /// Dispatch returns (Command, requires_confirmation).
    /// If matched in mutate_bindings → (cmd, true).
    /// All others → (cmd, false).
    pub fn dispatch(&self, key: KeyEvent) -> Option<(Command, bool)> {
        // Priority: global → mutate → browse → navigation → tui
        if let Some(cmd) = self.global_bindings.get(&key) {
            return Some((cmd.clone(), false));
        }
        if let Some(cmd) = self.mutate_bindings.get(&key) {
            return Some((cmd.clone(), true));  // always confirm
        }
        if let Some(cmd) = self.browse_bindings.get(&key) {
            return Some((cmd.clone(), false));
        }
        if let Some(cmd) = self.navigation_bindings.get(&key) {
            return Some((cmd.clone(), false));
        }
        if let Some(cmd) = self.tui_bindings.get(&key) {
            return Some((cmd.clone(), false));
        }
        None
    }

    /// Build reverse lookup for help overlay: Vec<(group, key_display, command_name)>
    pub fn all_bindings(&self) -> Vec<(String, String, String)> {
        // Used by help screen to display bindings grouped by category
    }
}
```

## App Integration

```rust
// crates/crystal-app/src/app.rs — handle_key changes

fn handle_key(&mut self, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }
    if let Some((cmd, requires_confirm)) = self.dispatcher.dispatch(key) {
        if requires_confirm {
            // Route through confirmation dialog
            self.pending_confirmation = Some(PendingConfirmation::from_command(cmd));
        } else {
            self.handle_command(cmd);
        }
    }
}
```

## Notes

- The `all_bindings()` method enables the help overlay to display keybindings
  grouped by category (navigation, browse, tui, global, mutate) instead of
  a flat list.
- Mode-specific handling (Insert, Search, NamespaceSelector, etc.) stays
  unchanged — those modes intercept keys before dispatch reaches the 5 groups.
- The `check_collisions()` function runs at config load time. On collision,
  the app logs a warning toast but uses whichever group has higher priority.

## Tests

- `KeybindingDispatcher::from_config()` builds all 5 maps without error
- Dispatch from `mutate` group returns `(cmd, true)`
- Dispatch from other groups returns `(cmd, false)`
- Priority: global key shadows same key in lower-priority group
- `all_bindings()` returns entries for all 5 groups
- `validate_keybindings()` catches unparseable key strings
- `check_collisions()` detects duplicate keys across groups
