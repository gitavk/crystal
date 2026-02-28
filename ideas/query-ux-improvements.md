# Query Pane UX Improvements

Six self-contained feature areas, ordered from easiest to most complex.
Each is independently shippable.

---

## Feature 1 — Tab key: Indent instead of mode-switch

### Problem
Tab in QueryEditor currently switches to QueryBrowse mode. Every SQL IDE uses
Tab for indentation (insert N spaces at cursor). Muscle memory fights this every
time a user writes a multi-line query.

### Solution
- `Tab` in QueryEditor → insert 2 spaces at cursor position (indent)
- `Shift+Tab` → remove up to 2 leading spaces on the current line (de-indent)
- Switch to browse mode: `Ctrl+Down` (move focus down to results)
- Switch back to editor: `Ctrl+Up` (move focus up to editor)
  - Replaces current `i` / `Enter` in QueryBrowse (keep those as aliases)

### Implementation
Files: `keybindings.rs` (rebind Tab/Shift+Tab in QueryEditor; add Ctrl+Down/Up
in both modes), `command.rs` (`QueryEditorIndent`, `QueryEditorDeIndent`),
`query_pane.rs` (`editor_indent`, `editor_deindent`), `app/query.rs` (delegation)

---

## Feature 2 — Optimal pane sizing on open

### Problem
When the query pane splits at 0.7 height ratio, the available width is still
shared with the resource list on the left. Writing and reading a complex query
in a narrow column is uncomfortable.

### Solution
Open the query pane in a **new tab** (full-screen) rather than a horizontal
split of the current pane. The resource list tab stays in its own tab.

- `Q` on a pod → connection dialog (unchanged)
- On confirm → open a new tab named `query:<pod>` with the QueryPane as the
  sole pane, filling the full terminal area
- Closing the QueryPane (`q` or `Esc` → ExitMode when pane is empty/idle) closes
  the tab and returns to the previous tab

Fallback preference (config): `query_open_mode = "new_tab" | "split"` (default
`"new_tab"`). The split behavior (current) stays available for users who prefer it.

### Implementation
Files: `app/query.rs` (`confirm_query_dialog` opens new tab instead of splitting),
`kubetile-config` (`query_open_mode` field), `app.rs` (read config on open)

---

## Feature 3 — Query history (persistent, per-pod)

### Problem
Every session starts with an empty editor. Re-typing or copy-pasting the same
queries across reconnects wastes time.

### Solution
- On each successful `Ctrl+Enter` execution, append the query to a per-pod
  history file
- `Ctrl+R` in QueryEditor opens a history popup (modal overlay)
- History popup: list of past queries (newest first), navigable with `j/k`,
  preview of full text on the right side, `Enter` loads into editor, `Esc` cancels
- `dd` in history popup deletes the selected entry
- Last 200 entries kept per pod; older entries pruned on append

### Storage
`~/.config/kubetile/query_history/<namespace>__<pod>__<db>.json`
Format: JSON array of `{ "sql": "...", "ts": "2026-02-26T12:00:00Z" }`

### Implementation
New files:
- `kubetile-core/src/query_history.rs` — `QueryHistory`, `load()`, `append()`,
  `delete()`, `prune(max=200)`
- `kubetile-tui/src/widgets/query_history_popup.rs` — list + preview widget

Modified files:
- `app/query.rs` — append to history on `QueryReady`; open/close popup on `Ctrl+R`
- `command.rs` — `OpenQueryHistory`, `QueryHistoryNext/Prev/Select/Delete/Close`
- `keybindings.rs` — `InputMode::QueryHistory` arm
- `event.rs` — no new events needed (history is sync file I/O)

---

## Feature 4 — Named saved queries

### Problem
History is ephemeral and unsorted. Frequently-used queries (health checks,
table size reports, slow query diagnostics) need a permanent named slot.

### Solution
- `Ctrl+S` in QueryEditor → prompt for a name (single-line input in a small modal)
- Named queries stored globally (not per-pod), since the same query often works
  across pods
- `Ctrl+O` (open) → popup listing saved queries, searchable with `/`, `Enter` loads
- `dd` in saved queries popup deletes the entry
- `e` renames the selected entry inline

### Storage
`~/.config/kubetile/saved_queries.json`
Format: JSON array of `{ "name": "slow queries", "sql": "...", "ts": "..." }`

### Implementation
New files:
- `kubetile-core/src/saved_queries.rs` — `SavedQueries`, `load()`, `save()`,
  `add()`, `rename()`, `delete()`
- `kubetile-tui/src/widgets/saved_queries_popup.rs`
- `kubetile-tui/src/widgets/name_input_popup.rs` (reusable single-line prompt)

Modified files: `command.rs`, `keybindings.rs`, `app/query.rs`

---

## Feature 5 — Export to file

### Problem
Clipboard has practical limits. A result set with 500 rows or a table with
JSONB columns (a single cell may be tens of KB) overflows what's comfortable
to paste. The border-free copy goal is the same; the destination changes.

### Threshold logic
Evaluate after each query completes:
- **Small** (< 100 rows AND estimated CSV < 64 KB): clipboard only
- **Medium** (100–500 rows OR 64 KB–512 KB): offer both clipboard and file export
  (status line hint: `Y copies all · E exports to file`)
- **Large** (> 500 rows OR > 512 KB): auto-suggest file export; clipboard still
  available but with a warning toast `"Result is large — consider E to export"`

Estimation: `Σ col_widths × row_count` (fast, no string allocation needed).

### UX
- `E` in QueryBrowse → file export dialog (pre-filled path:
  `~/kubetile_<pod>_<db>_<YYYYMMDD_HHMMSS>.csv`)
- User edits path, `Enter` confirms, `Esc` cancels
- Write CSV (header + all rows) to path, show toast `"Exported 842 rows →
  ~/kubetile_...csv"`
- Error toast on I/O failure

### Implementation
New files:
- `kubetile-tui/src/widgets/export_dialog.rs` — single path input field
Modified files:
- `query_pane.rs` — `size_hint()` returning `(row_count, estimated_bytes)`;
  `export_csv_to_file(path) -> Result<()>`
- `command.rs` — `OpenExportDialog`, `ExportDialogInput/Backspace/Confirm/Cancel`
- `keybindings.rs` — `InputMode::ExportDialog` arm; `E` in QueryBrowse
- `app/query.rs` — `open_export_dialog()`, `confirm_export()`, hint logic
- `layout.rs` — `ExportDialogView` in `RenderContext`

---

## Feature 6 — Smart autocomplete

Two independent sub-features; ship reserved-word completion first.

### 6a — Reserved-word completion
Trigger: `Ctrl+Space` in QueryEditor.
Static list of ~120 PostgreSQL reserved words (SELECT, FROM, WHERE, JOIN, ON,
GROUP BY, ORDER BY, HAVING, WITH, INSERT, UPDATE, DELETE, CREATE, DROP,
RETURNING, DISTINCT, LIMIT, OFFSET, UNION, INTERSECT, EXCEPT, CASE, WHEN,
THEN, ELSE, END, CAST, COALESCE, NULLIF, …).
Completion popup: floating overlay just below the cursor, shows up to 8
matching entries, filtered by the current token (word left of cursor).
Navigation: `Up/Down` or `Ctrl+P/N`, `Tab`/`Enter` accepts, `Esc` dismisses.

### 6b — Database object completion
Trigger: same `Ctrl+Space`, but context-aware:
- After `FROM` / `JOIN` / `UPDATE` / `INTO`: complete table names
- After `<table>.` : complete column names for that table
- After `SELECT` / `WHERE` / `ORDER BY`: mix of column names (from tables in the
  FROM clause already typed) + functions

Schema fetch on connect (after `SELECT version()` succeeds):
```sql
SELECT table_name, table_schema
FROM information_schema.tables
WHERE table_schema NOT IN ('pg_catalog','information_schema')
ORDER BY table_name;
```
Column fetch on first use of a table (lazy, cached in `QueryPane`):
```sql
SELECT column_name, data_type
FROM information_schema.columns
WHERE table_name = $1 AND table_schema = $2
ORDER BY ordinal_position;
```
Cache stored in `QueryPane` as `HashMap<String, Vec<ColumnInfo>>`.
Schema queries run via the same kube exec path as user queries.

### Completion popup architecture
```
QueryPane.completion_popup: Option<CompletionPopup>
CompletionPopup { items: Vec<String>, selected: usize, prefix: String }
```
Rendered as a floating `Block` + `List` widget overlaid on the editor area,
positioned at cursor coordinates.

Parser: simple token scan of the current query text to determine context
(no full SQL parser needed — heuristic based on last keyword before cursor).

### Implementation
New files:
- `kubetile-core/src/autocomplete.rs` — `CompletionSource`, `KeywordSource`,
  `SchemaSource { table_cache, column_cache }`
- `kubetile-tui/src/widgets/completion_popup.rs`

Modified files:
- `query_pane.rs` — `completion_popup` field; `trigger_completion()`,
  `complete_accept()`, `complete_dismiss()`, `complete_next/prev()`;
  render overlay in `render()`
- `command.rs` — `TriggerCompletion`, `CompleteNext/Prev/Accept/Dismiss`
- `keybindings.rs` — `Ctrl+Space` → `TriggerCompletion` in QueryEditor;
  intercept `Up/Down/Tab/Esc/Enter` when popup is open
- `app/query.rs` — fetch schema objects after `handle_query_ready` on the
  initial `SELECT version()` connection test; `trigger_completion()` delegates
  to pane

---

## Suggested shipping order

| Stage | Feature | Effort |
|-------|---------|--------|
| 8 | Tab → indent / Ctrl+Down to browse | XS |
| 9 | Optimal pane sizing (new tab) | S |
| 10 | Query history | M |
| 11 | Export to file | M |
| 12 | Named saved queries | M |
| 13 | Autocomplete 6a (reserved words) | M |
| 14 | Autocomplete 6b (schema objects) | L |
