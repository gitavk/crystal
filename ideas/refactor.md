# Module Split Refactor Plan

Files targeted (non-test files above ~490 lines):

| File | Lines |
|------|-------|
| `crates/kubetile-app/src/app.rs` | 2411 |
| `crates/kubetile-tui/src/views/logs_view.rs` | 705 |
| `crates/kubetile-app/src/keybindings.rs` | 618 |
| `crates/kubetile-tui/src/pane.rs` | 491 |

Test files (`tests.rs`, `tests/mod.rs`) are excluded — large test files are normal.

---

## 1. `app.rs` → `app/` directory

**Strategy:** Convert `app.rs` into a module directory. Keep the `App` struct definition and `new()` + `run()` in `mod.rs`. Each functional concern becomes its own file with a dedicated `impl App` block.

### Target layout

```
crates/kubetile-app/src/app/
├── mod.rs            # App struct definition, new(), run(), re-exports
├── types.rs          # PendingAction, PendingConfirmation, PortForwardField,
│                     #   PendingPortForward, TabScope
├── input.rs          # handle_event(), handle_key(), handle_command()
├── watchers.rs       # start_watcher_for_pane() + spawn_bridge helper
├── tabs.rs           # new_tab(), close_tab(), toggle_*_tab(), switch_to_*(),
│                     #   sync_active_scope(), load_active_scope(), etc.
├── pane_ops.rs       # toggle_help(), focus_next/prev/direction(), set_focus(),
│                     #   split_focused(), close_focused(), close_pane(),
│                     #   toggle_fullscreen(), switch_resource(),
│                     #   handle_resource_update(), handle_resource_error(),
│                     #   with_pods_pane()
├── context.rs        # handle_namespace_*/context_*(), select_namespace/context(),
│                     #   filtered_namespaces/contexts(), refresh_namespaces(),
│                     #   apply_context_switch(), restart_watchers_for_active_panes()
├── logs_exec.rs      # open_detail_pane(), open_yaml_pane(), open_logs_pane(),
│                     #   find_logs_pane_*(), start_logs_stream_for_pane(),
│                     #   open_exec_pane(), attach_logs_*(), poll_runtime_panes()
├── port_forward.rs   # toggle_port_forward_for_selected(), open_port_forward_prompt(),
│                     #   confirm_port_forward(), attach_port_forward(),
│                     #   stop_all_port_forwards(), refresh_port_forwards_panes(),
│                     #   stop_selected_port_forward()
├── actions.rs        # initiate_delete(), initiate_save_logs(),
│                     #   execute_confirmed_action(), selected_resource_info(),
│                     #   focused_supports_insert_mode()
├── render.rs         # build_render_context(), mode_name(),
│                     #   update_active_tab_title(), active_namespace_label(),
│                     #   active_view_alias()
└── helpers.rs        # Free functions: resource_alias(), resource_kind_config_key(),
                      #   is_kubectl_available_with_logging(), kubectl_binary_candidates(),
                      #   best_port_for_pod(), EmptyPane struct + impl
```

### Steps

1. Create `crates/kubetile-app/src/app/` directory
2. Move `app.rs` → `app/mod.rs`, add `mod types; mod input; ...` declarations
3. For each submodule: create the file, cut the relevant methods/types from `mod.rs`, paste with `use super::*` or explicit imports
4. Fix `use` paths — methods reference each other via `self`, so `impl App` blocks across files work automatically
5. Run `cargo check` after each file to catch import issues
6. Run `cargo test` at the end

---

## 2. `logs_view.rs` → `logs_view/` directory

**Strategy:** The file has ~335 lines of real code and ~370 lines of inline tests. The render method is complex and uses several private helpers. Split render logic and helpers from the core struct.

### Target layout

```
crates/kubetile-tui/src/views/logs_view/
├── mod.rs        # LogLineRef, LogsView struct + all non-render impl methods
│                 #   (constructor, accessors, set_filter, set_container_filter,
│                 #    scroll_*, update())
├── render.rs     # impl LogsView { fn render() } + private render helpers:
│                 #   has_multiple_containers(), container_color(),
│                 #   truncate_str(), highlight_matches(), dim_area()
└── tests.rs      # existing inline test mod moved out
```

### Steps

1. Create `crates/kubetile-tui/src/views/logs_view/` directory
2. Move `logs_view.rs` → `logs_view/mod.rs`
3. Extract `#[cfg(test)] mod tests { ... }` → `logs_view/tests.rs`, replace with `#[cfg(test)] mod tests;`
4. Extract `render()` method + helpers (`has_multiple_containers` through `dim_area`) → `logs_view/render.rs`
5. Add `mod render;` in `mod.rs`, add `use super::*` in `render.rs`
6. Update `views/mod.rs` if it re-exports `logs_view`
7. `cargo check && cargo test`

---

## 3. `keybindings.rs` → `keybindings/` directory

**Strategy:** The bulk of the file is two groups of repetitive boilerplate: key-string parsing utilities and command-name mapping functions. These can each live in their own file.

### Target layout

```
crates/kubetile-app/src/keybindings/
├── mod.rs        # InputMode enum, KeybindingDispatcher struct + impl
│                 #   (from_config(), dispatch(), mode(), set_mode(),
│                 #    key_for(), *_shortcuts(), all_shortcuts())
├── parsing.rs    # parse_key_string(), normalize_key_event(),
│                 #   format_key_display(), key_to_input_string()
└── commands.rs   # global_command_from_name/description(),
│                 #   mutate_command_from_name/description(),
│                 #   interact_command_from_name/description(),
│                 #   browse_command_from_name/description(),
│                 #   navigation_command_from_name/description(),
│                 #   tui_command_from_name/description()
└── tests.rs      # existing test mod (already separate file at keybindings/tests.rs)
```

Note: `keybindings/tests.rs` already exists — check if it's referenced via `mod tests;` inside `keybindings.rs` or is a sibling file. Adjust accordingly.

### Steps

1. Create `crates/kubetile-app/src/keybindings/` directory
2. Move `keybindings.rs` → `keybindings/mod.rs`
3. Extract key parsing functions → `keybindings/parsing.rs`
4. Extract command mapping functions → `keybindings/commands.rs`
5. In each new file add `use crate::command::Command;` and other needed imports
6. `cargo check && cargo test`

---

## 4. `pane.rs` → `pane/` directory

**Strategy:** `pane.rs` has four logically distinct groups: primitive types, resource types, the binary tree implementation, and the directional navigation algorithm.

### Target layout

```
crates/kubetile-tui/src/pane/
├── mod.rs          # Pane trait, PaneId type alias, re-exports of all submodules,
│                   #   pub use types::*; pub use resource::*; etc.
├── types.rs        # PaneId, Direction, PaneCommand, SplitDirection
├── resource.rs     # ResourceKind (+ impls), ViewType
├── tree.rs         # PaneNode, PaneTree + all split/close/resize/layout methods
│                   #   + split_rect() and other layout helpers
└── navigation.rs   # find_pane_in_direction(), perpendicular_overlap(),
                    #   edge_distance() helpers
```

Note: `pane/tests.rs` already exists at 367 lines — keep it as-is, already split.

### Steps

1. Create `crates/kubetile-tui/src/pane/` directory
2. Move `pane.rs` → `pane/mod.rs`
3. Extract types → `pane/types.rs`, resource kinds → `pane/resource.rs`
4. Extract `PaneNode` + `PaneTree` + split helpers → `pane/tree.rs`
5. Extract `find_pane_in_direction` + geometric helpers → `pane/navigation.rs`
6. In `mod.rs` add `pub use` re-exports so the public API is unchanged
7. `cargo check && cargo test`

---

## Execution Order

Safest order to avoid cascading import issues:

1. **`pane.rs`** — leaf crate (`kubetile-tui`), no app dependencies
2. **`logs_view.rs`** — also in `kubetile-tui`, isolated from app
3. **`keybindings.rs`** — in `kubetile-app`, no circular deps
4. **`app.rs`** — depends on everything above; do last

After each file: `cargo check` → fix imports → `cargo test`.
