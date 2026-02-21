# Configuration

KubeTile is designed to be highly customizable. You can configure keybindings, theme colors, and resource view columns by creating a configuration file.

## Configuration File Location

KubeTile reads `~/.config/kubetile/config.toml` on startup. All keys are optional — omitted keys fall back to defaults.

To get started, run:
```bash
kubetile --init-config
```
This will generate a default config file with all available options and comments.

The configuration file is **hot-reloaded**, so changes you save will be applied instantly without restarting the app.

## General

```toml
[general]
tick_rate_ms = 250          # UI refresh rate in milliseconds
default_namespace = "default"
default_view = "pods"       # View shown when opening a new pane
editor = "$EDITOR"          # Editor used to open YAML (env var or path)
shell = "$SHELL"            # Shell used for terminal panes
log_tail_lines = 1000       # Lines of logs to fetch initially
confirm_delete = true       # Require confirmation before deleting resources
show_managed_fields = false # Show managedFields in YAML view
```

## Terminal

```toml
[terminal]
scrollback_lines = 10000
cursor_style = "block"      # "block" | "underline" | "bar"
```

## Features

```toml
[features]
hot_reload = true           # Reload config without restarting
command_palette = true
port_forward = true
```

## Theme

Colors accept hex values (`"#89b4fa"`) or `"default"` to use the terminal default.

```toml
[theme]
accent = "#89b4fa"
bg = "default"
fg = "#cdd6f4"
header-bg = "#1e1e2e"
header-fg = "#cdd6f4"
selection-bg = "#45475a"
selection-fg = "#cdd6f4"
border = "#585b70"
border-active = "#89b4fa"
text-dim = "#6c7086"
overlay-bg = "#1e1e2e"

# Status colours
status-running = "#a6e3a1"
status-pending = "#f9e2af"
status-failed = "#f38ba8"
status-unknown = "#585b70"

# YAML syntax highlighting
yaml-key = "#89b4fa"
yaml-string = "#a6e3a1"
yaml-number = "#fab387"
yaml-boolean = "#cba6f7"
yaml-null = "#585b70"

# Insert-mode indicator
insert-mode-bg = "#a6e3a1"
insert-mode-fg = "#1e1e2e"
```

## Keybindings

Override individual keys. See [Keybindings](keybindings.md) for the full reference.

```toml
[keybindings.global]
quit = "ctrl+q"

[keybindings.navigation]
scroll_up = "k"
scroll_down = "j"

[keybindings.browse]
filter = "/"
view_yaml = "y"

# ... etc.
```

## View columns

Control which columns appear in each resource list.

```toml
[views.pods]
columns = ["name", "ready", "status", "restarts", "age", "node"]

[views.deployments]
columns = ["name", "ready", "up-to-date", "available", "age"]

[views.services]
columns = ["name", "type", "cluster-ip", "external-ip", "ports", "age"]
```

Available resource kinds: `pods`, `deployments`, `services`, `statefulsets`, `daemonsets`, `jobs`, `cronjobs`, `configmaps`, `secrets`, `ingresses`, `nodes`, `namespaces`, `pvs`, `pvcs`.
