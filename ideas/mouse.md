# Mouse Integration Plan

Merged from `mouse_claude.md`, `mouse_codex.md`, `mouse_gemini.md`.

## Guiding Principles

- Mouse support is **opt-in and additive**. Default: `mouse.enabled = false`.
- Every mouse action must have a keyboard equivalent. Mouse never bypasses the command layer.
- Route all mouse events through the existing input dispatcher — no separate control paths.
- Enable `crossterm::event::EnableMouseCapture` on startup when configured; always `DisableMouseCapture`
  on exit.
- Add runtime capability checks and graceful fallback for terminals that do not report mouse events.
- Keep deterministic focus rules when keyboard and mouse compete.

```toml
[mouse]
enabled            = false
capture-in-terminal = false   # pass-through mouse in PTY/exec panes
double-click-action = "open-detail"  # "open-detail" | "exec" | "logs"
scroll-lines       = 3
copy-on-select     = false
right-click-menu   = true
```

---

## Feature Catalogue

### Phase A — Quick Wins (low effort, high value)

#### A1. Click-to-Focus Pane
`MouseEventKind::Down(Left, col, row)` → hit-test pane `Rect`s → `focus_pane(idx)`.
Works for any pane: resource list, logs, query editor, terminal.

#### A2. Click Tab Bar to Switch Tab
Store each tab's x-span after render. `Down(Left)` on tab bar row → `switch_tab(n)`.
Keyboard equivalent: `Alt+1..9`.

#### A3. Scroll Wheel in Scrollable Panes
`ScrollUp` / `ScrollDown` → hit-test pane under cursor → dispatch `Command::ScrollUp` /
`Command::ScrollDown` to that pane (hover-scroll, not just focused pane).
`Shift+Wheel` → horizontal scroll in wide tables / query results.
Applies to: resource lists, log view, query results, YAML/detail panes.

#### A4. Middle-Click to Close Pane / Tab
`Down(Middle)` in tab bar → `close_tab(tab_idx)`.
`Down(Middle)` inside pane body → `close_pane(pane_idx)` (prompt confirmation for active PTY panes).
Keyboard equivalent: `Ctrl+W`.

#### A5. Mouse-Hover Status Hints
`MouseEvent::Moved` → hit-test pane and row → set `status_bar_hint: Option<String>`.
Status bar renders the hint instead of default context while `hint` is `Some`.
Hint clears after 3 s or on any key event.
Useful for: truncated cell values (image tags, labels, error text), pane descriptions.

---

### Phase B — Selection UX (medium effort)

#### B1. Click Row to Select / Double-Click for Default Action
`Down(Left)` in a resource list → compute `row − list_rect.y − header_height + scroll_offset` →
set `selected_index`.
Double-click (same cell, ≤ 400 ms) → fires configurable `double-click-action`
(`open-detail` / `exec` / `logs`).
Keyboard equivalent: `j/k` + `Enter`.

#### B2. Clickable Buttons in Dialogs
Store each button `Rect` when rendering a dialog/modal.
`Down(Left)` over a button → treat as `Enter` on that button.
Reusable `button_hit_test(mouse_pos, button_rects)` helper shared by all dialogs.

#### B3. Scrollbar Track / Thumb Click and Drag
Store scrollbar `Rect` (right-edge column, y-range) after render.
`Down(Left)` above thumb → page up; below thumb → page down.
`Down(Left)` on thumb → enter scroll-drag mode; `Drag` updates offset proportionally; `Up` exits.
Keyboard equivalent: `PageUp / PageDown`.

#### B4. Clickable Status-Bar / Breadcrumb Targets
Click namespace or context segment in the status bar → open selector overlay.
Click port-forward segment → open port-forward dialog.
Clickable breadcrumb trail in pane header (`Context > Namespace > Workload > Pod`):
click any segment to jump back to that level or open a selection dropdown.
Keyboard equivalent: dedicated shortcut per segment (e.g. `Ctrl+N` for namespace picker).

#### B5. Multi-Select with Shift/Ctrl+Click
`Ctrl+Click` a resource row → toggle it into a selection set.
`Shift+Click` → range-select rows between last selection and click target.
When ≥ 2 rows are selected, a "Batch Action" bar appears: Delete All, Restart All, etc.
Keyboard equivalent: `Space` to toggle selection, `Shift+j/k` for range.

---

### Phase C — Context Actions (depends on stable resource views, Stage 4+)

#### C1. Right-Click Context Menu on Resources
`Down(Right, col, row)` → select that row + open `ContextMenu` overlay anchored at `(col, row)`.
Actions: `Describe`, `Logs`, `Exec`, `Port-Forward`, `Delete` (with confirmation), `Copy name`,
`Query (Q)`. Each item shows its keyboard shortcut alongside.
`Up/Down/Enter/Esc` also navigate the menu; click outside closes it.

#### C2. Clickable Column Headers for Sorting / Resizing
Store column header x-ranges after render.
`Down(Left)` on header → `sort_by_column(col_idx)`, repeat toggles `▲/▼` direction.
`Drag` on column separator → resize column width (snap to min).
Keyboard equivalent: `s` / `S` to cycle / toggle sort.

---

### Phase D — Advanced (after keyboard clipboard and pane layout are solid)

#### D1. Text Selection in Log / Query Result Panes
`Down(Left)` → anchor `(col, row)`; `Drag` → update end, highlight selection with inverted bg;
`Up(Left)` → finalise, show "Press y to copy" hint in status bar.
`y` → copy plain text of selection to clipboard (strips borders and ANSI spans).
`Ctrl+Shift+C` → alternative copy shortcut for terminal environments that intercept `y`.
`Esc` → clear selection.
Keyboard equivalent: existing `Y` clipboard feature in Query Panel.

#### D2. Pane Border Drag to Resize
`MouseEvent::Moved` within 1 col/row of a border edge → highlight border.
`Down(Left)` on border → resize mode; `Drag` → update split ratio; `Up` → finalise.
Snap to min/max constraints to prevent broken layouts.
Keyboard equivalent: `Ctrl+Arrow` in resize mode (zellij-style).

#### D3. "Time-Travel" Log Scrubbing
Horizontal scrubber at bottom of log pane representing the full log buffer timeline.
Click or drag the scrubber thumb to jump to a specific timestamp range.
Complements vertical scroll; keyboard equivalent: `g` / `G` for top/bottom.

#### D4. Visual Diff Selection
`Ctrl+Click` two resource rows (pods, deployment revisions, etc.) → opens split-pane Diff View
showing YAML differences side by side.
Keyboard equivalent: `d` with two rows marked via multi-select.

#### D5. Drag-and-Drop YAML Apply
Drag a `.yaml` file path (from another terminal pane or OS file manager, if terminal supports
drag-drop OSC sequences) into a KubeTile resource list pane → shows "Confirm Apply" dialog.
Keyboard equivalent: `:apply <path>` command.

---

### Phase E — Future Stages

#### E1. Interactive Resource Relationship Map (Stage 11 — XRay View)
In the XRay / dependency graph view:
- Click a node (Service, Pod, Endpoint) → highlight related nodes.
- Hover a connection line → show traffic metrics or latency (via Service Mesh plugin).
- Drag nodes to reorganise the view temporarily.
Keyboard equivalent: arrow navigation between nodes.

#### E2. Plugin Mouse Hooks (Stage 7 — WASM Plugin System)
Expose mouse events in the plugin API: `click`, `wheel`, `drag`, coordinates, modifiers.
Plugins declare interactive "hot-spot" regions on their custom views.
WASM guest handles events directly → enables custom dashboards, gauges, node graphs.

#### E3. AI "Explain This" via Right-Click (Stage 12 — AI Assistance)
Right-click a log line, event, or YAML field → context menu item "AI Explain".
Opens a side-pane overlay with an AI-generated explanation of the error, warning, or field.
Keyboard equivalent: `?` on a selected row.

---

## Implementation Phasing

| Phase | Features | Prerequisite |
|-------|----------|--------------|
| **A** | A1 click-focus, A2 tab click, A3 scroll wheel, A4 middle-close, A5 hover hints | Stable pane layout |
| **B** | B1 row click/double-click, B2 dialog buttons, B3 scrollbar, B4 breadcrumb/status-bar, B5 multi-select | Phase A done |
| **C** | C1 right-click menu, C2 column headers | Stage 4 resource views |
| **D** | D1 text selection+copy, D2 pane resize drag, D3 log scrubbing, D4 visual diff, D5 drag-drop YAML | Keyboard clipboard solid + stable layout |
| **E** | E1 XRay map, E2 plugin hooks, E3 AI explain | Stages 7, 11, 12 respectively |

---

## Technical Architecture

### Geometry Storage — `MouseMap`

After each render pass, populate a flat `Vec<(Rect, HitTarget)>`. Event handler iterates in
reverse paint order (topmost widget first) to find the first match.

```rust
enum HitTarget {
    Pane(usize),
    Tab(usize),
    Button(CommandId),
    ListRow { pane: usize, row: usize },
    ScrollbarTrack(usize),
    ScrollbarThumb(usize),
    ColumnHeader { pane: usize, col: usize },
    ColumnSeparator { pane: usize, col: usize },
    StatusBarSegment(StatusSegment),
    BreadcrumbSegment(usize),  // depth index
}
```

### Double-Click Detection

```rust
struct LastClick {
    button: MouseButton,
    col: u16,
    row: u16,
    at: Instant,
}
```

On `Down`: if previous click was same button, same cell, and `elapsed() < 400ms` → double-click.

### Opt-Out Guarantee

Wrap all mouse event processing behind `if self.config.mouse.enabled`.
All `HitTarget` arms must fire the same `Command` variants that keyboard shortcuts fire.

### crossterm Event Types Used

```
MouseEventKind::Down(MouseButton::Left)
MouseEventKind::Down(MouseButton::Right)
MouseEventKind::Down(MouseButton::Middle)
MouseEventKind::Up(MouseButton::Left)
MouseEventKind::Drag(MouseButton::Left)
MouseEventKind::Moved
MouseEventKind::ScrollUp
MouseEventKind::ScrollDown
```

### Testing

- Unit tests for `MouseMap` hit-testing (rect containment, reverse paint order).
- Integration tests for scroll behavior in core list/log widgets.
- Keyboard parity tests: for every mouse feature, assert the keyboard equivalent fires the same
  `Command` variant.
