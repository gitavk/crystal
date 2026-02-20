# Keybindings

All keybindings are configurable via `~/.config/kubetile/config.toml`. These are the defaults.

## Global

Available in every mode.

| Key | Action |
|-----|--------|
| `F1` | Toggle help overlay |
| `Ctrl+q` | Quit |
| `Ctrl+n` | Open namespace selector |
| `Ctrl+k` | Open context selector |
| `Ctrl+Shift+p` | Toggle port-forwards panel |
| `Ctrl+Shift+l` | Toggle application logs |
| `i` | Enter insert mode |

## Navigation

| Key | Action |
|-----|--------|
| `j` / `Down` | Scroll / select next |
| `k` / `Up` | Scroll / select previous |
| `Enter` | Confirm / open |
| `Esc` | Back / cancel |
| `g` | Go to top |
| `G` (`Shift+g`) | Go to bottom |
| `Ctrl+f` | Page down |
| `Ctrl+b` | Page up |
| `Ctrl+Right` | Scroll right |
| `Ctrl+Left` | Scroll left |

## Browse (resource list)

| Key | Action |
|-----|--------|
| `/` | Filter |
| `a` | Toggle all-namespaces |
| `:` | Open resource switcher |
| `y` | View YAML |
| `d` | View describe output |
| `s` | Sort by column |
| `S` (`Shift+s`) | Reverse sort order |
| `f` | Toggle log follow |
| `w` | Toggle log wrap |
| `Ctrl+s` | Save logs to file |

## Interact (pod actions)

| Key | Action |
|-----|--------|
| `e` | Exec into pod |
| `l` | View logs |
| `p` | Port-forward |

## Mutate (destructive — require confirmation)

| Key | Action |
|-----|--------|
| `Ctrl+Alt+d` | Delete resource |
| `Ctrl+Alt+s` | Scale resource |
| `Ctrl+Alt+r` | Restart / rollout restart |

## Pane & tab management

| Key | Action |
|-----|--------|
| `Tab` | Focus next pane |
| `Shift+Tab` | Focus previous pane |
| `Alt+Up/Down/Left/Right` | Focus pane in direction |
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

## Mode-specific keys

### Selectors (namespace / context / resource switcher)

| Key | Action |
|-----|--------|
| `Up` / `Down` | Navigate list |
| Type characters | Filter list |
| `Backspace` | Delete filter character |
| `Enter` | Confirm selection |
| `Esc` | Cancel |

### Confirm dialog

| Key | Action |
|-----|--------|
| `y` | Confirm |
| `n` / `Esc` | Cancel |

### Port-forward input

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` / arrows | Switch field |
| `0`–`9` | Enter port number |
| `Backspace` | Delete digit |
| `Enter` | Confirm |
| `Esc` | Cancel |
