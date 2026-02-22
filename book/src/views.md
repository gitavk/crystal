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

## Debug Mode

Debug mode lets you replace a running container's command with `sleep infinity` so you can exec into it for investigation without the application process interfering.

Select a **Pod** and press `Ctrl+Alt+d` to toggle debug mode. KubeTile will:
1. Resolve the Pod's owner Deployment via its ReplicaSet.
2. Save the original `command` and `args` as annotations on the Deployment.
3. Patch the Deployment to run `sleep infinity` — all pods in the Deployment restart.
4. Press the same key again to restore the original command and exit debug mode.

### Root Debug Mode

Press `F5` on a Pod to toggle **root debug mode**. This does everything debug mode does, and additionally sets:

```yaml
securityContext:
  runAsUser: 0
```

This is useful when you need root access inside the container (e.g., to inspect system files or install debugging tools). The original `securityContext` is preserved and restored on exit.

> **Safety:** if the Deployment is already in one debug mode when you activate the other, the original application command is never overwritten — it is reused from the existing saved annotation.
