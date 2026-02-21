# Keybindings

All keybindings are configurable via `~/.config/kubetile/config.toml`. These are the defaults.

## Navigation

| Key | Action |
|-----|--------|
| `j` / `Down` | Scroll / select next |
| `k` / `Up` | Scroll / select previous |
| `Enter` | Confirm / select |
| `Esc` | Back / cancel |
| `g` | Go to top |
| `G` (`Shift+g`) | Go to bottom |
| `Ctrl+f` / `PageDown` | Page down |
| `Ctrl+b` / `PageUp` | Page up |
| `Ctrl+Right` | Scroll right |
| `Ctrl+Left` | Scroll left |

## Global

Available in every mode.

| Key | Action |
|-----|--------|
| `F1` | Show help |
| `Ctrl+q` | Quit |
| `Ctrl+n` | Open namespace selector |
| `Ctrl+k` | Open context selector |
| `Ctrl+Shift+p` | Toggle port-forwards panel |
| `Ctrl+Shift+l` | Toggle application logs |
| `i` | Enter insert mode |

## Browse (Resource List)

| Key | Action |
|-----|--------|
| `:` | Open resource switcher |
| `/` | Filter |
| `a` | Toggle all-namespaces |
| `y` | View YAML |
| `d` | View describe |
| `s` | Sort column |
| `S` (`Shift+s`) | Toggle sort order |
| `f` | Toggle log follow |
| `w` | Toggle log wrap |
| `Ctrl+s` | Save logs to file |
| `Ctrl+e` | Download (export) full log history |

## Interact (Resource Actions)

| Key | Action |
|-----|--------|
| `e` | Exec into pod |
| `l` | View logs |
| `p` | Port-forward |

## Mutate (Destructive — require confirmation)

| Key | Action |
|-----|--------|
| `Ctrl+Alt+d` | Delete resource |
| `Ctrl+Alt+s` | Scale resource |
| `Ctrl+Alt+r` | Restart / rollout restart |

## Pane & Tab Management

| Key | Action |
|-----|--------|
| `Tab` | Focus next pane |
| `Shift+Tab` | Focus previous pane |
| `Alt+Up` / `Down` / `Left` / `Right` | Focus pane in direction |
| `Alt+v` | Split pane vertically |
| `Alt+h` | Split pane horizontally |
| `Alt+x` | Close focused pane |
| `Alt+f` | Toggle fullscreen on focused pane |
| `Alt+Shift+Up` | Grow pane |
| `Alt+Shift+Down` | Shrink pane |
| `Ctrl+t` | New tab |
| `Ctrl+w` | Close tab |
| `Alt+1` – `Alt+9` | Jump to tab by number |
| `Alt+Enter` | Open terminal pane |
