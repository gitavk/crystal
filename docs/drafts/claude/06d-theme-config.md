# Step 6.4 — Config-Driven Theme

> `feat(tui): replace hardcoded theme constants with config-driven Theme`

## Goal

Replace the hardcoded `const` color values in crystal-tui `theme.rs` with a
`Theme` struct built from `ThemeConfig`. The existing Catppuccin Mocha palette
becomes the default in `defaults.toml` — users can override any color.

## Files

| File | Action |
|------|--------|
| `crates/crystal-config/src/theme.rs` | NEW — ThemeConfig struct (string color values) |
| `crates/crystal-config/src/defaults.toml` | EXPAND — add [theme] section |
| `crates/crystal-tui/src/theme.rs` | REWRITE — Theme struct + from_config + parse_color |

## Config Type (crystal-config)

```rust
// crates/crystal-config/src/theme.rs

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ThemeConfig {
    // Base colors
    pub accent: String,
    pub bg: String,
    pub fg: String,
    pub header_bg: String,
    pub header_fg: String,
    pub selection_bg: String,
    pub selection_fg: String,
    pub border: String,
    pub border_active: String,
    pub text_dim: String,
    pub overlay_bg: String,

    // Status colors
    pub status_running: String,
    pub status_pending: String,
    pub status_failed: String,
    pub status_unknown: String,

    // YAML syntax highlighting
    pub yaml_key: String,
    pub yaml_string: String,
    pub yaml_number: String,
    pub yaml_boolean: String,
    pub yaml_null: String,

    // Mode indicator
    pub insert_mode_bg: String,
    pub insert_mode_fg: String,
}
```

## defaults.toml additions

```toml
[theme]
# Catppuccin Mocha palette (existing defaults, now configurable)
accent = "#89b4fa"
bg = "default"
fg = "#cdd6f4"
header-bg = "#1e1e2e"
header-fg = "#cdd6f4"
selection-bg = "#45475a"
selection-fg = "#cdd6f4"
border = "#585b70"
border-active = "#89b4fa"
text-dim = "#6c7086"
overlay-bg = "#1e1e2e"

# Status colors
status-running = "#a6e3a1"
status-pending = "#f9e2af"
status-failed = "#f38ba8"
status-unknown = "#585b70"

# YAML syntax highlighting
yaml-key = "#89b4fa"
yaml-string = "#a6e3a1"
yaml-number = "#fab387"
yaml-boolean = "#cba6f7"
yaml-null = "#585b70"

# Mode indicator
insert-mode-bg = "#a6e3a1"
insert-mode-fg = "#1e1e2e"
```

## Theme Struct (crystal-tui)

```rust
// crates/crystal-tui/src/theme.rs

pub struct Theme {
    pub accent: Color,
    pub bg: Color,
    pub fg: Color,
    pub header: Style,
    pub status_bar: Style,
    pub selection: Style,
    pub border: Style,
    pub border_active: Style,
    pub text_dim: Style,
    pub overlay: Style,
    pub status_running: Style,
    pub status_pending: Style,
    pub status_failed: Style,
    pub yaml_key: Style,
    pub yaml_string: Style,
    pub yaml_number: Style,
    pub yaml_boolean: Style,
    pub yaml_null: Style,
    pub insert_mode: Style,
}

impl Theme {
    pub fn from_config(config: &ThemeConfig) -> Self {
        // Parse each color string → Color
        // Compose Style values from fg/bg pairs
    }
}

/// Parse color string into ratatui Color.
/// Supported formats:
///   "#89b4fa"           → Color::Rgb
///   "rgb(137,180,250)"  → Color::Rgb
///   "red", "blue", etc. → Color::Red, Color::Blue (named)
///   "default"           → Color::Reset
pub fn parse_color(s: &str) -> anyhow::Result<Color> { /* ... */ }
```

## Migration

1. Keep the old `const` values as a reference — they must match the
   `defaults.toml` hex values exactly.
2. Replace all call sites that use `theme::ACCENT`, `theme::BODY_BG`, etc.
   with `theme.accent`, `theme.border`, etc.
3. `Theme` is passed through `RenderContext` (already borrowed by panes).
4. Delete the old `const` block once all call sites are migrated.

## Notes

- `Theme` must be `Clone` since hot-reload replaces it atomically.
- `parse_color` should return a helpful error message including the invalid
  string and supported formats.
- The `"default"` color maps to `Color::Reset`, meaning the terminal's own
  background/foreground is used.

## Tests

- `parse_color("#89b4fa")` → `Color::Rgb(137, 180, 250)`
- `parse_color("rgb(137,180,250)")` → same
- `parse_color("red")` → `Color::Red`
- `parse_color("default")` → `Color::Reset`
- `parse_color("not-a-color")` → descriptive error
- `Theme::from_config(ThemeConfig::default())` matches old const values
