# Resource List

The default view when opening a pane. Shows a live-updating table of Kubernetes resources filtered to the selected namespace.

**Supported resource kinds:** Pods, Deployments, Services, StatefulSets, DaemonSets, Jobs, CronJobs, ConfigMaps, Secrets, Ingresses, Nodes, Namespaces, PVs, and PVCs.

---

## Keybindings

### Browse

| Key | Action |
|-----|--------|
| `/` | Filter by name |
| `s` | Sort by column |
| `S` (`Shift+S`) | Toggle sort order |
| `a` | Toggle all-namespaces view |
| `:` | Open resource switcher |

### Open

| Key | Action |
|-----|--------|
| `Enter` | Open detail view |
| `y` | Open YAML view |
| `d` | Open describe view |
| `l` | Stream logs |
| `e` | Exec into pod |
| `p` | Port-forward |
| `Shift+Q` | Open query pane (PostgreSQL) |

### Mutate

> These actions require confirmation and use triple-modifier chords to prevent accidents.

| Key | Action |
|-----|--------|
| `Ctrl+Alt+X` | Delete resource |
| `Ctrl+Alt+S` | Scale resource |
| `Ctrl+Alt+R` | Restart / rollout restart |
| `Ctrl+Alt+D` | Toggle debug mode |
| `F5` | Toggle root debug mode |

---

See also: [Keybindings reference](../keybindings.md)
