# Query Panel

## Problem

Users running PostgreSQL inside Kubernetes pods face two friction points:
1. Copying query results from `psql` copies TUI border characters along with the data
2. Results that overflow the screen cannot be copied at all

## Solution

A first-class Query Panel inside KubeTile — a split pane with a multi-line SQL editor
(top ~30%) and a scrollable, border-free result table (bottom ~70%), with system clipboard
copy of rows or the full result set.

## Design Decisions

- **DB type**: PostgreSQL only (`psql --csv`) for initial release
- **Connection**: dialog pre-filled from pod env vars (`POSTGRES_DB`, `POSTGRES_USER`,
  `POSTGRES_PASSWORD`, `PGPORT`); user edits before confirming
- **Copy**: `arboard` crate for OS clipboard; graceful toast fallback on failure
- **Editor**: multi-line, `Ctrl+Enter` executes

## User Flow

1. Select a pod in resource list → press `Q`
2. Async task reads pod spec env vars from K8s API
3. `QueryDialog` modal appears with pre-filled connection fields
4. User reviews/edits Database / User / Password / Port, confirms with `Enter`
5. `QueryPane` opens (splits current pane horizontally)
6. Editor area is active — user types SQL
7. `Ctrl+Enter` executes → async `psql --csv` via kube exec
8. Result table renders (no borders, full data in memory regardless of viewport size)
9. `y` copies selected row as CSV; `Y` copies all rows with header
10. `j/k/PgUp/PgDn` scroll the full result set

## Architecture

### New files
- `crates/kubetile-core/src/query.rs` — `QueryConfig`, `QueryResult`, `execute_query()`, `read_postgres_env()`
- `crates/kubetile-tui/src/widgets/query_dialog.rs` — `QueryDialogWidget`, `QueryDialogField`
- `crates/kubetile-app/src/panes/query_pane.rs` — `QueryPane` implementing `Pane`
- `crates/kubetile-app/src/app/query.rs` — app-level query orchestration methods

### Key modifications
- `ViewType::Query(String)` added to kubetile-tui
- `QueryDialogView` added to `RenderContext` in layout.rs
- New `AppEvent` variants: `QueryPromptReady`, `QueryReady`, `QueryError`
- New `InputMode` variants: `QueryDialog`, `QueryEditor`
- New `Command` variants: `OpenQueryPane`, dialog input commands, editor input commands,
  `QueryExecute`, `QueryCopyRow`, `QueryCopyAll`
- `arboard = "3"` added as workspace dependency

### Result rendering (no-border approach)
The result table uses plain `Paragraph` lines instead of ratatui's `Table` widget:
- Column widths computed as `max(header.len(), max_cell_len)` across all rows
- Each cell padded to column width with 2-space separator
- Header rendered in `theme.accent.bold()`, separator line in `─` chars
- Full result set stored in memory; scroll offset applied at render time
- Horizontal scroll supported for wide result sets

## Execution mechanism
Uses kube-rs exec API (not kubectl subprocess):
```
["env", "PGPASSWORD=<pw>", "psql", "-U", user, "-d", db, "-p", port, "--csv", "-c", sql]
```
Stdout collected and parsed as CSV. Stderr collected for error reporting.

## Implementation Stages

Each stage is independently shippable and testable before moving to the next.

---

### Stage 1 — Connection Dialog (no DB, no pane)
**Goal**: prove the modal + env-var autofill works end-to-end before touching any DB logic.

- Read pod spec from K8s API, extract env vars (`POSTGRES_DB`, `POSTGRES_USER`,
  `POSTGRES_PASSWORD`, `PGPORT`)
- Open a centered modal dialog with pre-filled fields: Database, User, Password (masked), Port
- Tab/Shift+Tab cycles fields; Esc cancels; Enter just closes with a toast showing the collected
  config values (no connection yet)
- **Acceptance**: select any pod → press `Q` → dialog appears with correct pre-fills → Enter
  shows toast `"Config: db=mydb user=postgres port=5432"` → Esc dismisses

Files touched: `query.rs` (read_postgres_env only), `query_dialog.rs`, `command.rs`
(InputMode::QueryDialog + dialog commands), `event.rs` (QueryPromptReady), `app/query.rs`
(open_query_pane_for_selected, open_query_dialog, cancel_query_dialog),
`keybindings.rs` (QueryDialog mode dispatch), `layout.rs` (QueryDialogView in RenderContext)

---

### Stage 2 — Empty QueryPane + connection test
**Goal**: open the pane and verify the DB connection with a `SELECT 1` before the user types
anything.

- On dialog confirm, create `QueryPane` (split horizontal), focus it, set mode to QueryEditor
- Immediately fire a background `SELECT 1` using the kube exec path
- Pane shows a loading spinner / status line `"Connecting…"`
- On success: status line shows `"Connected — PostgreSQL 15.2"` (from `SELECT version()`)
- On failure: status line shows `"Connection failed: <psql error>"` in red; pane stays open so
  the user can see the error and close manually
- **Acceptance**: correct credentials → pane opens → connection confirmed in status bar;
  wrong password → pane opens → error message visible

Files touched: `query.rs` (execute_query skeleton), `query_pane.rs` (struct + basic render,
set_loading/set_error/set_results stubs), `event.rs` (QueryReady, QueryError),
`app/query.rs` (confirm_query_dialog, execute_query_for_pane, handle_query_ready/error),
`panes/mod.rs`, `ViewType::Query`

---

### Stage 3 — Single-line editor + execute
**Goal**: type a query and run it; see raw results.

- Editor area renders a single editable line (no multi-line yet)
- Ctrl+Enter sends the query via kube exec, results arrive as `QueryResult`
- Results rendered as plain text lines (no column alignment yet): one row per line,
  cells separated by `|`
- Scroll up/down through results with j/k
- **Acceptance**: type `SELECT now();` → Ctrl+Enter → result line appears

Files touched: `query_pane.rs` (editor_input, editor_backspace, editor_query, basic result
rendering), `keybindings.rs` (QueryEditor dispatch), `command.rs` (editor commands,
QueryExecute), `app/input.rs` (route QueryEditorInput/Backspace/Execute)

---

### Stage 4 — Multi-line editor
**Goal**: support queries with newlines (JOINs, CTEs, subqueries).

- Editor becomes `Vec<String>` with cursor (row, col)
- Enter adds a new line; Backspace at col 0 merges with previous line
- Up/Down arrow moves cursor between lines
- Home/End move to start/end of current line
- Visual cursor highlight (invert style on cursor cell)
- **Acceptance**: write a 3-line query, navigate with arrows, execute with Ctrl+Enter

Files touched: `query_pane.rs` (editor_newline, cursor navigation, render cursor)

---

### Stage 5 — Aligned result table
**Goal**: replace raw pipe-separated output with a properly aligned, header-separated table.

- Column widths computed as `max(header.len(), max_cell_len_in_col)`
- Header row in `theme.accent.bold()`
- Separator line of `─` characters
- Cells padded to column width, 2-space gap between columns
- Selected row highlighted with `theme.selection`
- Vertical scrollbar on right edge
- **Acceptance**: wide multi-column result looks clean; scrollbar reflects position

Files touched: `query_pane.rs` (col_widths computation, aligned render, scrollbar)

---

### Stage 6 — Horizontal scroll
**Goal**: handle result sets wider than the terminal.

- Horizontal scroll offset shifts all rendered lines left
- ScrollLeft/ScrollRight keybinds move by one column width
- Status line shows column range in view: `cols 1–8 of 24`
- **Acceptance**: query with 20 columns → scroll right reveals off-screen columns

Files touched: `query_pane.rs` (horizontal_offset, scroll clamping, status line)

---

### Stage 7 — Clipboard copy
**Goal**: copy without borders.

- Add `arboard = "3"` workspace dependency
- `y` → copy selected row as CSV line (`"val1","val2","val3"\n`)
- `Y` → copy full result as CSV with header row
- Success toast: `"Copied 1 row"` / `"Copied 150 rows"`
- Graceful error toast if clipboard unavailable (e.g. no display server)
- **Acceptance**: `Y` → paste into spreadsheet → clean data, no border characters

Files touched: `Cargo.toml`, `kubetile-app/Cargo.toml`, `app/query.rs`
(query_copy_row, query_copy_all), `command.rs` (QueryCopyRow, QueryCopyAll),
`keybindings/commands.rs` (browse bindings for y/Y)

---

## Future extensions
- MySQL / SQLite support via configurable `db_type` field
- Query history (persistent, per-pod)
- Named saved queries
- Export to file as alternative/complement to clipboard
