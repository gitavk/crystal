# Views

## Resource list

The default view when opening a pane. Shows a live-updating table of Kubernetes resources filtered to the selected namespace.

Supported resource kinds: Pods, Deployments, Services, StatefulSets, DaemonSets, Jobs, CronJobs, ConfigMaps, Secrets, Ingresses, Nodes, Namespaces.

Press `/` to filter, `s` / `S` to sort, `a` to toggle all-namespaces, `:` to switch resource kind.

## YAML view

Press `y` on any resource to open its full YAML. `show_managed_fields` in `[general]` controls whether `managedFields` is included.

## Describe view

Press `d` on any resource to view `kubectl describe`-style output.

## Logs view

Press `l` on a Pod row to stream its logs in a new pane. Press `f` to toggle follow, `w` to toggle line wrapping, `Ctrl+s` to save to a file.

## Exec view

Press `e` on a Pod row to open an interactive shell inside the container (uses the shell configured in `[general] shell`).

## Port-forward

Press `p` on a Pod or Service row to open the port-forward input. Manage active forwards with `Ctrl+Shift+p`.

## Terminal

Press `Alt+Enter` to open a general-purpose terminal pane. Scrollback is configurable via `[terminal] scrollback_lines`.
