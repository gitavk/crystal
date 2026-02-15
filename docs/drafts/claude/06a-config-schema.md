# Step 6.1 — AppConfig Schema, General Settings & Feature Flags

> `feat(config): expand Config into full AppConfig with general, terminal, and feature flags`

## Goal

Grow the existing `Config` struct into a full `AppConfig` that covers all
configurable sections. Add `GeneralConfig`, `TerminalConfig`, and `FeatureFlags`
as new sub-structs. Every new section uses `#[serde(default)]` so the existing
`defaults.toml` still parses without breaking.

## Files

| File | Action |
|------|--------|
| `crates/crystal-config/src/lib.rs` | EXPAND — rename Config → AppConfig, add new sections |
| `crates/crystal-config/src/general.rs` | NEW — GeneralConfig, TerminalConfig, FeatureFlags |
| `crates/crystal-config/src/defaults.toml` | EXPAND — add [general], [terminal], [features] sections |

## Data Structures

```rust
// crates/crystal-config/src/lib.rs

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub keybindings: KeybindingsConfig,  // existing — kept as-is for now
    pub theme: ThemeConfig,
    pub views: ViewsConfig,
    pub terminal: TerminalConfig,
    pub features: FeatureFlags,
}

impl Default for AppConfig {
    fn default() -> Self {
        toml::from_str(include_str!("defaults.toml")).unwrap()
    }
}

impl AppConfig {
    /// Load from file, falling back to defaults for missing fields
    pub fn load(path: &Path) -> anyhow::Result<Self> { /* ... */ }

    /// XDG config path: ~/.config/crystal/config.toml
    pub fn default_path() -> PathBuf { /* ... */ }

    /// Save current config to file (for --init-config)
    pub fn save(&self, path: &Path) -> anyhow::Result<()> { /* ... */ }

    /// Generate default config file if it doesn't exist
    pub fn init_default() -> anyhow::Result<PathBuf> { /* ... */ }
}
```

```rust
// crates/crystal-config/src/general.rs

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub tick_rate_ms: u64,            // existing field, moved here
    pub default_namespace: String,
    pub default_view: String,         // "pods", "deployments", etc.
    pub editor: String,               // $EDITOR fallback
    pub shell: String,                // $SHELL fallback
    pub log_tail_lines: u32,
    pub confirm_delete: bool,
    pub show_managed_fields: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TerminalConfig {
    pub scrollback_lines: u32,
    pub cursor_style: String,         // "block", "underline", "bar"
}

/// Feature flags — opt-in behavior toggles.
/// New experimental features get a flag here before becoming defaults.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FeatureFlags {
    pub hot_reload: bool,             // enable config hot-reload (default: true)
    pub command_palette: bool,        // enable ":" command palette (default: true)
    pub port_forward: bool,           // enable port-forward feature (default: true)
}
```

## defaults.toml additions

```toml
[general]
tick-rate-ms = 250
default-namespace = "default"
default-view = "pods"
editor = "$EDITOR"
shell = "$SHELL"
log-tail-lines = 1000
confirm-delete = true
show-managed-fields = false

[terminal]
scrollback-lines = 10000
cursor-style = "block"

[features]
hot-reload = true
command-palette = true
port-forward = true
```

## Notes

- `tick_rate_ms` moves from the top-level `Config` into `GeneralConfig`.
  The old field should be kept temporarily with `#[serde(alias)]` for
  backwards compatibility during migration.
- Feature flags follow the ChatGPT architecture's "config module owns feature
  flags" principle — they let users opt into experimental features without code
  changes.
- All new sub-structs use `#[serde(default)]` so a completely empty TOML file
  still produces valid config.

## Tests

- `AppConfig::default()` has all expected fields populated
- Partial TOML (only `[general]` section) merges with defaults correctly
- Feature flags all default to `true`
- `tick_rate_ms` reads from both old and new location
