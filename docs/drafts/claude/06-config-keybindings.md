# Stage 6 — Configuration & Custom Keybindings

## Goal

Expand crystal-config into a full TOML-based configuration system that controls
keybindings, theme colors, view columns, feature flags, and behavior. Users can
fully customize the experience without touching code. Config is hot-reloadable.

**Guiding principle — "Config over magic":** every tunable behavior must have an
explicit configuration knob. No hidden defaults, no magic auto-detection for
user-facing settings.

## Prerequisites

- Stage 5 complete (terminal, exec, logs working)
- Existing crystal-config crate with `Config`, `KeybindingsConfig`, and
  embedded `defaults.toml`
- Existing `KeybindingDispatcher` in crystal-app with `parse_key_string()`
- Existing hardcoded Catppuccin theme constants in crystal-tui `theme.rs`

## Current State (what already exists)

The foundation is partially built:

- **crystal-config/src/lib.rs** — `Config` struct with `tick_rate_ms` and
  `KeybindingsConfig` (3 sections: global, pane, resource)
- **crystal-config/src/defaults.toml** — embedded default keybindings
- **crystal-app/src/keybindings.rs** — `KeybindingDispatcher` with
  `parse_key_string()`, `InputMode` enum, dispatch routing
- **crystal-app/src/command.rs** — `Command` enum (all actions go through this)
- **crystal-tui/src/theme.rs** — hardcoded `const` colors (Catppuccin)

This stage promotes config from "keybindings only" to "everything configurable."

## Design Rules (from architecture spec)

These rules from the ChatGPT architecture doc constrain the config design:

- **UI never mutates state directly** — config changes flow through `AppEvent::ConfigReloaded`
- **All actions go through Commands** — keybindings map to `Command` enum variants, not raw functions
- **Config over magic** — prefer explicit configuration over implicit behavior
- **Small context windows** — each config section is a self-contained module

## YouTube Episodes

1. **"Making It Yours — Config System in Rust"**: TOML parsing, serde, defaults
2. **"Hot Reload & Custom Keybindings"**: file watcher, runtime rebinding

---

## New/Modified Files

```
crates/
├── crystal-config/
│   └── src/
│       ├── lib.rs                 # EXPAND — full AppConfig with all sections
│       ├── keybindings.rs         # NEW — keybinding config types + validation
│       ├── theme.rs               # NEW — theme/color config types
│       ├── general.rs             # NEW — general + terminal + feature flags
│       ├── views.rs               # NEW — per-resource column config
│       └── defaults.toml          # EXPAND — add theme, general, views, flags
├── crystal-app/
│   └── src/
│       ├── app.rs                 # Use AppConfig throughout
│       ├── keybindings.rs         # REFACTOR — 5 binding groups, (Command, bool) return
│       ├── config_watcher.rs      # NEW — file watcher for hot reload
│       └── event.rs               # ADD ConfigReloaded variant
├── crystal-tui/
│   └── src/
│       └── theme.rs               # REWRITE — build Theme from ThemeConfig
```

---

## Steps

| Step | File | Commit | Summary |
|------|------|--------|---------|
| 6.1 | [06a-config-schema.md](06a-config-schema.md) | `feat(config): expand Config into full AppConfig with general, terminal, and feature flags` | AppConfig, GeneralConfig, TerminalConfig, FeatureFlags |
| 6.2 | [06b-keybinding-groups.md](06b-keybinding-groups.md) | `feat(config): redesign keybindings into 5 semantic groups with modifier conventions` | Group design, modifier escalation, expanded defaults.toml |
| 6.3 | [06c-keybinding-dispatcher.md](06c-keybinding-dispatcher.md) | `feat(app): refactor KeybindingDispatcher for 5 groups with mutate confirmation` | KeybindingsConfig types, dispatcher 3→5 maps, (Command, bool) return |
| 6.4 | [06d-theme-config.md](06d-theme-config.md) | `feat(tui): replace hardcoded theme constants with config-driven Theme` | ThemeConfig, Theme::from_config, parse_color |
| 6.5 | [06e-hot-reload.md](06e-hot-reload.md) | `feat(app): add config file watcher with hot reload` | ConfigWatcher, notify crate, AppEvent::ConfigReloaded |
| 6.6 | [06f-cli-flags.md](06f-cli-flags.md) | `feat(app): add --init-config and --print-config CLI flags` | Config generation, effective config dump |
| 6.7 | [06g-view-columns.md](06g-view-columns.md) | `feat(config): add per-resource view column configuration` | ViewsConfig, ResourceViewConfig |

---

## Command Flow Diagram

```
Keyboard Input
   ↓
KeyEvent (crossterm)
   ↓
KeybindingDispatcher::dispatch(key)
   ↓                              ┌─────────────────────────┐
(Command, requires_confirm)       │ KeybindingsConfig (TOML) │
   ↓                              │ parsed at startup +      │
App::handle_command(cmd)          │ on ConfigReloaded event  │
   ↓                              └─────────────────────────┘
State Mutation / Side Effect
   ↓
RenderContext (borrowed)
   ↓
UI Re-render (Theme from ThemeConfig)
```

No module skips this flow. Config changes re-enter at the top via
`AppEvent::ConfigReloaded`.

## Migration Path

Since crystal-config already has `Config` and `KeybindingsConfig`:

1. Rename `Config` → `AppConfig`, add new sections with `#[serde(default)]`
   so existing `defaults.toml` still parses
2. **Migrate keybinding groups** from 3 (global/pane/resource) → 5
   (navigation/browse/tui/global/mutate). Redistribute existing bindings:
   - `pane` → split into `navigation` (scroll/select) and `tui` (split/resize)
   - `resource` → split into `browse` (yaml/logs/filter) and `mutate` (delete/scale/exec)
   - `global` → stays `global` but keys change (q → ctrl+q, ? → ctrl+h, etc.)
3. Update `KeybindingDispatcher` from 3 maps to 5, add `(Command, bool)` return
4. Add theme/general/views/features sections to `defaults.toml` incrementally
5. Replace hardcoded theme constants in crystal-tui one-by-one
6. Add `ConfigWatcher` as optional (feature-flagged) component

## Tests

- Unit: `parse_key_string("ctrl+d")` produces correct `KeyEvent` (already exists)
- Unit: `parse_color("#89b4fa")` produces correct `Color`
- Unit: `AppConfig::default()` has all expected fields populated
- Unit: Partial TOML (only `[theme]` section) merges with defaults correctly
- Unit: `validate_keybindings()` catches invalid key strings
- Unit: Feature flags default to expected values
- Unit: View column config round-trips through serde
- Integration: Config file change triggers hot reload and rebuilds dispatcher
- Integration: Invalid config produces toast, app continues with previous config

## Demo Checklist

- [ ] `crystal --init-config` generates `~/.config/crystal/config.toml`
- [ ] `crystal --print-config` dumps effective config
- [ ] Edit theme colors → app updates live (hot reload)
- [ ] Remap `j`/`k` to `n`/`p` → navigation changes immediately
- [ ] Invalid config → error toast, app continues with previous config
- [ ] Feature flag `port-forward = false` → port forward key does nothing
- [ ] Custom view columns → resource table reflects changes
- [ ] Show config file structure and explain sections
