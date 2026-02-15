# Step 6.2 — Keybinding Group Design & Defaults

> `feat(config): redesign keybindings into 5 semantic groups with modifier conventions`

## Goal

Replace the current 3-section keybinding layout (global/pane/resource) with 5
semantic groups, each with a consistent modifier key convention. The modifier
escalation signals danger level to the user.

## Files

| File | Action |
|------|--------|
| `crates/crystal-config/src/defaults.toml` | REWRITE keybinding sections — 3 groups → 5 |

## Group Design

| Group | Modifier | Purpose | Danger level |
|---|---|---|---|
| **navigation** | _(bare)_ | Cursor movement, selection, scrolling | None |
| **browse** | _(bare)_ | Read-only resource inspection | None |
| **tui** | `alt+` | Pane/tab layout management | Low (reversible) |
| **global** | `ctrl+` | App-wide actions, mode switches | Medium |
| **mutate** | `ctrl+alt+` | Destructive resource operations | High (confirm dialog) |

### Why this works

- **Modifier escalation matches danger:** bare → alt → ctrl → ctrl+alt
- Users build muscle memory around *groups*, not individual keys
- The help overlay can display bindings grouped by modifier
- `mutate` group is automatically gated by confirmation dialogs
- A few high-frequency exceptions (tab switching `1-9`, `tab`/`shift+tab`
  for focus) stay as bare keys in the tui group for ergonomics

### Dispatch priority (first match wins in Normal mode)

`global → mutate → browse → navigation → tui`

### Migration from old groups

| Old section | New group(s) | What moved |
|---|---|---|
| `[keybindings.pane]` | `navigation` + `tui` | scroll/select → navigation; split/resize → tui |
| `[keybindings.resource]` | `browse` + `mutate` | yaml/logs/filter → browse; delete/scale/exec → mutate |
| `[keybindings.global]` | `global` + `tui` | quit/help/selectors → global; pane/tab ops → tui |

## Expanded defaults.toml (keybinding sections)

```toml
# ── Keybindings ──────────────────────────────────────────────────────────
# 5 groups, each with a default modifier ("super key"):
#   navigation = bare     browse = bare     tui = alt+
#   global = ctrl+        mutate = ctrl+alt+

# Navigation — bare keys, cursor/selection movement
[keybindings.navigation]
scroll_up = "k"
scroll_down = "j"
select_prev = "up"
select_next = "down"
select = "enter"
back = "esc"
go_to_top = "g"
go_to_bottom = "G"
page_up = "pageup"
page_down = "pagedown"

# Browse — bare keys, read-only resource inspection
[keybindings.browse]
view_yaml = "y"
view_describe = "d"
view_logs = "l"
filter = "/"
resource_switcher = ":"
sort_column = "s"
toggle_all_namespaces = "a"
toggle_follow = "f"

# TUI — alt+ prefix, pane/tab layout management
[keybindings.tui]
split_vertical = "alt+v"
split_horizontal = "alt+h"
close_pane = "alt+w"
toggle_fullscreen = "alt+f"
focus_up = "alt+up"
focus_down = "alt+down"
focus_left = "alt+left"
focus_right = "alt+right"
resize_grow = "alt+]"
resize_shrink = "alt+["
new_tab = "alt+t"
close_tab = "alt+c"
open_terminal = "alt+enter"
# Exceptions: high-frequency tab ops stay bare for ergonomics
focus_next = "tab"
focus_prev = "shift+tab"
goto_tab_1 = "1"
goto_tab_2 = "2"
goto_tab_3 = "3"
goto_tab_4 = "4"
goto_tab_5 = "5"
goto_tab_6 = "6"
goto_tab_7 = "7"
goto_tab_8 = "8"
goto_tab_9 = "9"

# Global — ctrl+ prefix, app-wide actions and mode switches
[keybindings.global]
quit = "ctrl+q"
help = "ctrl+h"
app_logs = "ctrl+l"
enter_insert = "ctrl+i"
namespace_selector = "ctrl+n"
context_selector = "ctrl+o"

# Mutate — ctrl+alt+ prefix, destructive operations (always confirm)
[keybindings.mutate]
delete = "ctrl+alt+d"
scale = "ctrl+alt+s"
restart_rollout = "ctrl+alt+r"
exec = "ctrl+alt+e"
port_forward = "ctrl+alt+p"
```

## Notes

- The group name itself is the safety policy: anything in `mutate` auto-triggers
  a confirmation dialog, no per-command annotation needed.
- `browse` and `navigation` both use bare keys but don't conflict because
  they bind different command names — the dispatcher maps command names to
  `Command` enum variants.
- The `tui` exceptions (`1-9`, `tab`, `shift+tab`) are documented inline so
  users understand why they break the `alt+` convention.

## Tests

- All key strings in every group are parseable by `parse_key_string()`
- No two commands within the same group share the same key
- No key collisions across groups (same KeyEvent in two groups)
