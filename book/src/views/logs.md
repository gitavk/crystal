# Logs & Terminal

KubeTile's terminal integration provides shells and log streams automatically configured for your cluster.

---

## Log Streaming

Select a Pod and press `l` to stream its logs in a new pane.

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `Down` | Scroll down |
| `k` / `Up` | Scroll up |
| `g` | Go to top |
| `G` | Go to bottom |
| `Ctrl+F` / `PageDown` | Page down |
| `Ctrl+B` / `PageUp` | Page up |
| `f` | Toggle follow mode |
| `w` | Toggle line wrapping |
| `/` | Filter log lines |
| `Ctrl+S` | Save visible logs to file (respects active filter) |
| `Ctrl+E` | Download full log history to file |

---

## Exec into Pods

Select a Pod and press `e` to open an interactive shell inside its container (uses the shell configured in `[general] shell`).

- Press `Esc` to switch from the terminal to Normal mode.
- Press `i` to switch back to Insert mode and resume typing.

---

See also: [Keybindings reference](../keybindings.md)
