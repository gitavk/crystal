# Views

KubeTile features a powerful layout system inspired by Zellij, allowing you to create a workspace that fits your needs with Tabs and Panes.

## Resource List

The default view when opening a pane. Shows a live-updating table of Kubernetes resources filtered to the selected namespace.

- **Supported Resource Kinds:** Pods, Deployments, Services, StatefulSets, DaemonSets, Jobs, CronJobs, ConfigMaps, Secrets, Ingresses, Nodes, Namespaces, PVs, and PVCs.
- **Commands:** Press `/` to filter, `s` to sort, `a` to toggle all-namespaces, and `:` to open the **Resource Switcher** (command palette).

## YAML View

Press `y` on any resource to view its full YAML definition with syntax highlighting. The configuration `show_managed_fields` in `[general]` controls whether `managedFields` are included.

## Describe View

Press `d` on any resource to view detailed `kubectl describe`-style output.

## Detail View

Press `Enter` on any resource to open a detailed view in a new pane. This shows metadata, status, specs, and more.

## Integrated Terminal & Logs

KubeTile's terminal integration provides shells and log streams automatically configured for your cluster.

### Log Streaming
Select a Pod and press `l` to stream its logs in a new pane.
The log view supports:
- following (`f`) 
- filtering (`/`)
- toggling line wrapping (`w`)
- saving visible logs(include filtering) to a file (`Ctrl+s`)
- export (download) full availeble logs (`Ctrl+e`)

### Exec into Pods
Select a Pod and press `e` to open an interactive shell inside its container
(uses the shell configured in `[general] shell`).
To switch to from the terminal you should go to `NORMAL` mode by `esc` key,
to again start work in terminal need to switch to `INSERT` mode by `i` key.

## Port-forward

Press `p` on a Pod or Service row to open the port-forward input. Manage active forwards with `Ctrl+Shift+p`.
