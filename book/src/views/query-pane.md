# Query Pane

The Query Pane is an integrated SQL environment for interacting with PostgreSQL
databases running inside your Kubernetes cluster — without leaving the terminal.
It combines a multi-line editor, a scrollable result table, query history,
saved queries, and clipboard copy into a single keyboard-driven interface.

---

## The Problem

Running `psql` inside a Kubernetes pod is a common pattern, but it creates
friction the moment you try to get data out of the terminal:

- Results that overflow the screen cannot be captured at all without shell
  redirects and additional `kubectl exec` invocations.
- There is no persistent history or way to name and recall frequently-used
  queries across sessions.

KubeTile solves these problems by executing queries through the Kubernetes API
(not a subprocess), storing the full result set in memory regardless of viewport
size, and rendering a clean, border-free table that can be copied or exported
as standards-compliant CSV.

---

## Opening the Query Pane

### Selecting a Target

Select any **Pod**, **Service**, or **StatefulSet** in the resource list and
press `Shift+Q`.

KubeTile reads the pod's environment variables from the Kubernetes API
(no exec required at this point). It looks for the standard PostgreSQL
environment variable names:

| Env var | Maps to |
|---|---|
| `POSTGRES_DB` or `PGDATABASE` | Database name |
| `POSTGRES_USER` or `PGUSER` | Username |
| `POSTGRES_PASSWORD` or `PGPASSWORD` | Password |
| `PGPORT` or `POSTGRES_PORT` | Port (default `5432`) |

### Container Detection

When a pod runs multiple containers, KubeTile automatically selects the PostgreSQL
container by preferring one that exposes port `5432` or whose name or image contains
`"postgres"` (but not `"exporter"`). Environment variables are resolved from inline
`env`, `envFrom`-backed ConfigMaps, and Secrets — so credentials stored in
Kubernetes Secrets are discovered automatically without manual entry.

### The Connection Dialog

A centered modal appears pre-filled with the detected values:

```
┌─── Connect to PostgreSQL ────────────────────────────────────────┐
│                                                                    │
│  Database  [mydb                          ]                        │
│  User      [postgres                      ]                        │
│  Password  [••••••••                      ]                        │
│  Port      [5432                          ]                        │
│                                                                    │
│  Tab / Shift+Tab: cycle fields   Enter: connect   Esc: cancel      │
└────────────────────────────────────────────────────────────────────┘
```

- `Tab` / `Shift+Tab` cycles through the four fields.
- `Enter` confirms and opens the Query Pane.
- `Esc` cancels without any connection.
- Password is always masked with `•` characters.

---

## The Query Pane Layout

On confirm, KubeTile opens the Query Pane. By default it opens in a **new tab**
(full-screen), giving the editor and result table the full terminal width.
A `split` mode is also available via configuration for users who prefer a
side-by-side view with the resource list.

The tab is named `query:<pod>`. Closing the sole pane in the tab (with `q` or
navigating away) closes the tab and returns to the previous tab automatically.

A status line below the editor shows the current connection state and the
active input mode.

---

> **Tip:** Press `F2` at any time to see a summary of all Query Pane keybindings.

## The SQL Editor

The editor is active as soon as the pane opens (`InputMode::QueryEditor`).
It is a full multi-line editor backed by a `Vec<String>` with a (row, col)
cursor, vertical scroll, and a visual cursor marker rendered with reversed
colors.

### Basic Editing

| Key | Action |
|---|---|
| Any printable character | Insert at cursor |
| `Backspace` | Delete character before cursor; at column 0 merges line with the one above |
| `Enter` | Insert a newline (splits the current line at the cursor) |
| `Tab` | Indent: insert 2 spaces at the start of the current line |
| `Shift+Tab` | De-indent: remove up to 2 leading spaces from the current line |

### Editor Actions

| Key | Action |
|---|---|
| `Ctrl+Enter` | Execute the query |
| `Ctrl+Space` | Trigger autocomplete |
| `Ctrl+R` | Open query history |
| `Ctrl+S` | Save current query with a name |
| `Ctrl+O` | Open saved queries |
| `Ctrl+Down` | Switch focus to the result table (Browse mode) |
| `Esc` | Return to Normal mode |

---

## Executing a Query

Press `Ctrl+Enter` to execute the SQL currently in the editor.

KubeTile sends the query to the pod via the Kubernetes exec API — the same
mechanism as `kubectl exec` but without spawning a subprocess. The command
executed inside the pod is:

```
env PGPASSWORD=<password> psql -U <user> -d <database> -p <port> --csv -c <sql>
```

A successful query appends the SQL to the persistent per-pod history file.

---

## The Result Table

### Vertical Navigation (Browse Mode)

Enter Browse mode by pressing `Ctrl+Down` in the editor, or `i`/`Enter` from Normal mode.

| Key | Action |
|---|---|
| `j` / `Down` | Select next row |
| `k` / `Up` | Select previous row |
| `PgDn` / `Ctrl+F` | Page down |
| `PgUp` / `Ctrl+B` | Page up |
| `Ctrl+Up` / `Enter` | Return to editor |
| `Esc` | Return to Normal mode |

### Horizontal Scroll

For result sets wider than the terminal, KubeTile supports horizontal
column-by-column scrolling.

| Key | Action |
|---|---|
| `l` / `Right` | Scroll right by one column |
| `h` / `Left` | Scroll left by one column |

---

## Clipboard Copy

In Browse mode:

| Key | Action |
|---|---|
| `y` | Copy the selected row as a CSV line |
| `Y` | Copy all rows (with the header row) as CSV |
| `E` | Export to File. |

The clipboard integration uses the `arboard` crate, which supports X11,
Wayland, and macOS. The `Clipboard` instance is kept alive for the duration of
the application so that X11's selection ownership is maintained — copying to
clipboard and then quitting KubeTile does not clear the clipboard contents.

---

## Query History

Press `Ctrl+R` in the editor to open the history popup.

### Storage

History is stored per-pod at:

```
~/.config/kubetile/query_history/<namespace>__<pod>__<db>.json
```

Each entry is a JSON object: `{ "sql": "...", "ts": "2026-02-26T12:00:00Z" }`.
The list is capped at **200 entries** per pod; older entries are pruned on
each append. Consecutive duplicate queries are de-duplicated (only the most
recent timestamp is kept).

---

## Named Saved Queries

### Saving a Query

Press `Ctrl+S` in the editor. A small name-input popup appears:

```
┌─── Save Query ──────────────────────────┐
│                                          │
│  Name: [slow queries▌               ]    │
│                                          │
│  Enter: save   Esc: cancel               │
└──────────────────────────────────────────┘
```

Type a name and press `Enter`. The current editor content is saved under that
name. Saved queries are global — not per-pod — since the same query often
works across different pods.

### Opening Saved Queries

Press `Ctrl+O` in the editor to open the saved queries popup. It has the same
two-panel layout as the history popup.

| Key | Action |
|---|---|
| `d` | Delete the selected entry |
| `e` | Rename the selected entry inline |
| `/` | Filter the list by name |
| `Esc` | Close sub-mode or close popup |

```
~/.config/kubetile/saved_queries.json
```

Format: `[ { "name": "slow queries", "sql": "...", "ts": "..." }, … ]`

---

## Autocomplete

Press `Ctrl+Space` in the editor to trigger autocomplete. A floating popup
appears just below the cursor showing up to 8 matching suggestions.

Typing re-filters the suggestions live. Backspace removes a character and
re-filters. The popup closes automatically if there are no matches.

### Schema-Aware Completion

When the connection test succeeds, KubeTile runs a background query against
`information_schema.columns` to fetch the full schema: table names, schemas,
column names, and data types. This is cached for the lifetime of the pane — no
repeated round-trips.

---

## Configuration

One configuration key affects the Query Pane's opening behavior:

```toml
[general]
query-open-new-tab = true   # default: true
                            # false → split the current pane horizontally (0.7 ratio)
```

---

See also: [Keybindings reference](../keybindings.md)
