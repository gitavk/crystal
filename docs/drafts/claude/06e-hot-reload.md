# Step 6.5 — Config File Watcher (Hot Reload)

> `feat(app): add config file watcher with hot reload`

## Goal

Watch the user's config file for changes and reload the configuration at
runtime. Gated behind `features.hot_reload`. On successful reload, the app
rebuilds the keybinding dispatcher, theme, and tick rate. On parse error, the
app shows a toast and continues with the previous config.

## Files

| File | Action |
|------|--------|
| `crates/crystal-app/src/config_watcher.rs` | NEW — ConfigWatcher using notify crate |
| `crates/crystal-app/src/event.rs` | ADD — AppEvent::ConfigReloaded variant |
| `crates/crystal-app/src/app.rs` | UPDATE — handle ConfigReloaded event |
| `crates/crystal-app/Cargo.toml` | ADD — notify dependency |

## ConfigWatcher

```rust
// crates/crystal-app/src/config_watcher.rs

use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};

pub struct ConfigWatcher {
    path: PathBuf,
    tx: mpsc::UnboundedSender<AppEvent>,
    _watcher: RecommendedWatcher,
}

impl ConfigWatcher {
    pub fn start(path: PathBuf, tx: mpsc::UnboundedSender<AppEvent>) -> anyhow::Result<Self> {
        // Create notify watcher with 100ms debounce
        // Watch the config file (not directory) for Modify events
        // On change:
        //   1. Read and parse the file
        //   2. Ok  → tx.send(AppEvent::ConfigReloaded(Box::new(config)))
        //   3. Err → tx.send(AppEvent::Toast(ToastMessage::error(format!("Config error: {err}"))))
        //            App continues with previous config
    }
}
```

## AppEvent Addition

```rust
// crates/crystal-app/src/event.rs

pub enum AppEvent {
    // ... existing variants ...
    ConfigReloaded(Box<AppConfig>),
}
```

## Event Handling

```rust
// crates/crystal-app/src/app.rs — in the event loop

AppEvent::ConfigReloaded(config) => {
    // Rebuild keybinding dispatcher with new groups
    self.dispatcher = KeybindingDispatcher::from_config(&config.keybindings);

    // Rebuild theme (flows through RenderContext to TUI)
    self.theme = Theme::from_config(&config.theme);

    // Update tick rate
    self.tick_rate = Duration::from_millis(config.general.tick_rate_ms);

    // Store new config
    self.config = *config;

    // Notify user
    self.toasts.push(ToastMessage::info("Config reloaded"));
}
```

## Startup Integration

```rust
// In App::new() or main.rs startup

let config = AppConfig::load(&config_path)?;

let config_watcher = if config.features.hot_reload {
    Some(ConfigWatcher::start(config_path.clone(), event_tx.clone())?)
} else {
    None
};
```

## Notes

- The watcher is gated behind `features.hot_reload` — if disabled at startup,
  no watcher is created. Changing the flag in the config file has no effect
  until restart (chicken-and-egg).
- Debounce at 100ms prevents rapid re-parsing when editors write files in
  multiple steps (write temp, rename).
- The `Box<AppConfig>` in the event avoids a large enum variant.
- Config reload follows the architecture rule: file change → notify →
  `ConfigReloaded` event → App → state mutation → UI re-render. The UI never
  reads the config file directly.

## Tests

- Integration: Write config file → watcher sends ConfigReloaded event
- Integration: Write invalid TOML → watcher sends Toast error, no ConfigReloaded
- Unit: ConfigWatcher::start with nonexistent path → returns error
- Integration: Rapid successive writes → only one ConfigReloaded (debounce)
