# Pane-Specific Help Overlay

KubeTile now has many pane types, each with its own set of keybindings that
vary by context (Normal mode, QueryEditor, QueryBrowse, Insert, etc.). The
existing `HelpPane` lists all global binding categories in a split-pane view —
but it doesn't know which pane is focused, mixes unrelated bindings together,
and grows harder to scan as features accumulate.

---

## Problem

A user in a `LogsPane` has to remember `f` (follow), `w` (wrap), `/` (filter)
among dozens of other keys that are irrelevant to logs. A user in the
`QueryPane` editor has no single place to check which key saves a query or
opens history.

The full HelpPane is useful as a reference but poor as a quick "what can I do
right now?" lookup.

---

## Solution

`?` opens a **floating modal overlay** showing only the keybindings relevant to
the currently focused pane. The overlay renders on top of all panes without
disturbing the layout. It closes with `Esc`, `q`, or `?`.

Each pane owns its help content by implementing `pane_help()` on the `Pane`
trait. The modal is pane-aware, not mode-aware — it shows when `?` is pressed
in Normal mode regardless of which pane is focused.

---

## Design Decisions

- **Modal, not split pane** — no layout disruption, no extra pane to close
- **Pane-owned content** — each pane defines what to advertise via
  `fn pane_help(&self) -> Vec<(String, String)>`; default returns empty vec
- **Normal mode only** — `?` in QueryEditor types `?`; user exits with Esc
  first. This is deliberate: text editors should not intercept `?`
- **`?` replaces old `ShowHelp` default** — `?` now opens the pane modal;
  the global `HelpPane` remains accessible if users rebind `help` in config
- **Two-column layout** — key (bold accent) on the left, description (dim) on
  the right, same visual style as HelpPane rows

---

## Architecture

### New file
- `crates/kubetile-tui/src/widgets/pane_help.rs` — `PaneHelpView<'a>` + `PaneHelpWidget`

### Modified files
- `crates/kubetile-tui/src/pane.rs` — add `pane_help()` default method to `Pane` trait
- `crates/kubetile-app/src/panes/*.rs` — implement `pane_help()` per pane
- `crates/kubetile-app/src/command.rs` — `ShowPaneHelp`, `ClosePaneHelp`
- `crates/kubetile-app/src/keybindings.rs` — `InputMode::PaneHelp`, dispatch arm
- `crates/kubetile-app/src/keybindings/commands.rs` — `"show_pane_help"` command
- `crates/kubetile-app/src/app.rs` — `pane_help_overlay` field
- `crates/kubetile-app/src/app/input.rs` — route commands
- `crates/kubetile-app/src/app/pane_ops.rs` — `show_pane_help()`, `close_pane_help()`
- `crates/kubetile-app/src/app/render.rs` — populate `pane_help` in `RenderContext`
- `crates/kubetile-tui/src/layout.rs` — render overlay in `render_body()`
- `kubetile-config` default config — bind `?` to `show_pane_help`

---

## Implementation Stage

### Stage 1 — Pane trait + per-pane content
**Goal**: Each pane reports its relevant keybindings.

- Add `fn pane_help(&self) -> Vec<(String, String)>` (default `vec![]`) to the `Pane` trait
- Implement for all pane types with their key → description pairs
- `QueryPane` returns `QueryEditor` keys (most user-relevant when opening from Normal)

Acceptance: calling `pane.pane_help()` on any pane returns a non-empty list
for user-facing panes and empty list for HelpPane/Empty.

Files: `kubetile-tui/src/pane.rs`, all files under `kubetile-app/src/panes/`

### Stage 2 — Command + InputMode + App state
**Goal**: App can track that the help overlay is open.

- `Command::ShowPaneHelp`, `Command::ClosePaneHelp`
- `InputMode::PaneHelp` — `dispatch()` returns `ClosePaneHelp` for `Esc`/`q`/`?`
- `"show_pane_help"` in global command map
- `pane_help_overlay: Option<Vec<(String, String)>>` on `App`

Files: `command.rs`, `keybindings.rs`, `keybindings/commands.rs`, `app.rs`

### Stage 3 — App methods + input routing
**Goal**: `ShowPaneHelp` collects entries from focused pane and opens modal.

- `show_pane_help()` — get `pane_help()` from focused pane, store in overlay field, set mode
- `close_pane_help()` — clear overlay, set mode to Normal
- Route both commands in `app/input.rs`

Files: `app/input.rs`, `app/pane_ops.rs`

### Stage 4 — Widget + render wiring
**Goal**: Overlay is visible in the TUI.

- `PaneHelpWidget` renders a centered floating box with `Clear` + border + two-column rows
- `PaneHelpView<'a>` holds `entries: &'a [(String, String)]` and a title string
- `RenderContext` gets `pane_help: Option<PaneHelpView<'a>>`
- `render_body()` renders it last (on top of other modals if needed)
- `app/render.rs` populates it when `InputMode::PaneHelp`

Files: `kubetile-tui/src/widgets/pane_help.rs`, `layout.rs`, `app/render.rs`

### Stage 5 — Config default rebind
**Goal**: `?` opens pane help out of the box.

- Change default `"help"` binding from `"?"` to `"show_pane_help": "?"`
- Keep `"help"` command available in global map for users who want the HelpPane

Files: `kubetile-config` default keybindings

---

## Future extensions

- **Scroll inside the modal** — for panes with many keys (e.g., QueryPane)
- **Section headers** — group keys inside the modal (Editor / Browse / History)
- **Status bar hint** — always-visible 1-line strip showing 4–6 keys for current
  mode (lazygit/helix style), complementing the `?` modal
- **`?` inside QueryEditor** — `Ctrl+?` chord to show help without leaving editor
