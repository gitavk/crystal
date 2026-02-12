# Stage 4 — Resource Views (Pods, Deployments, Services, ...)

## Goal

Implement full resource views for all major Kubernetes resource types. Each view
has a list mode and a detail mode. Add YAML view, describe view, and resource
deletion. This stage makes the app genuinely useful as a k9s alternative.

## Prerequisites

- Stage 3 complete (pane layout system working)

## YouTube Episodes

1. **"Every K8s Resource in One TUI — Generic Views"**: resource view pattern
2. **"Pod Deep Dive — Containers, Status, Events"**: pod detail view
3. **"Deployments, Services, and Beyond"**: remaining resources
4. **"CRUD in the Terminal — Delete, Edit, Scale"**: actions on resources

---

## Design Rules

These rules are **non-negotiable** and must hold across all tasks in this stage.
They are inherited from the architecture spec and Stage 3's pane contract.

### All Actions Through Commands
- Keyboard input → `KeybindingDispatcher` → `Command` enum → `App::handle_command()` → state mutation → UI re-render
- No module skips this flow — resource actions follow the same path as pane splits and tab switches

### UI Never Mutates State Directly
- Panes receive data via `ResourceListState` or equivalent read-only structs
- Panes never call K8s APIs, never access global state, never talk to other panes
- Root UI composes pane output into the final frame

### Kubernetes Access Only Via crystal-core
- All K8s operations (watch, delete, scale, get YAML) go through `crystal-core` types
- `crystal-app` calls into `crystal-core`; `crystal-tui` never imports kube-rs

### Config Over Magic
- New keybindings for resource actions are config-driven via TOML
- Help screen reflects active keybindings, not hardcoded defaults

---

## Steps

| Step | File | Commit | Summary |
|------|------|--------|---------|
| 4.1 | [04a-resource-types.md](04a-resource-types.md) | `feat(tui,core): expand ResourceKind enum and ResourceSummary trait` | Extend ResourceKind with all 14 variants, add row()/detail_sections() to ResourceSummary |
| 4.2 | [04b-summary-structs.md](04b-summary-structs.md) | `feat(core): implement resource summary structs for all 14 types` | DeploymentSummary, ServiceSummary, etc. — all implement ResourceSummary |
| 4.3 | [04c-generic-watcher.md](04c-generic-watcher.md) | `feat(core): generalize ResourceWatcher for any k8s resource type` | Generic watch<K,S>(), multi-watcher management in App |
| 4.4 | [04d-commands-modes.md](04d-commands-modes.md) | `feat(app): extend commands and input modes for resource actions` | New Command variants, PaneCommand extensions, InputMode additions |
| 4.5 | [04e-resource-list-pane.md](04e-resource-list-pane.md) | `feat(app): add filter, sort, and all-namespaces to ResourceListPane` | Filter bar, column sorting, namespace toggle in list view |
| 4.6 | [04f-detail-yaml-panes.md](04f-detail-yaml-panes.md) | `feat(app): implement detail pane and YAML pane` | ResourceDetailPane with scrollable sections, YamlPane with syntax highlighting |
| 4.7 | [04g-action-executor.md](04g-action-executor.md) | `feat(core): add ActionExecutor for delete, scale, restart, get_yaml` | crystal-core action execution via KubeClient |
| 4.8 | [04h-resource-switcher.md](04h-resource-switcher.md) | `feat(app): add resource switcher command palette` | `:` command palette with fuzzy match over resource types |
| 4.9 | [04i-overlays.md](04i-overlays.md) | `feat(tui,app): add confirm dialog, toast, and context-sensitive help` | ConfirmDialog, Toast widget, context-sensitive HelpPane |
| 4.10 | [04j-wire-app.md](04j-wire-app.md) | `feat(app): wire resource views, actions, and watchers into App core` | Integration in App::handle_command(), RenderContext, event routing |

## Implementation Dependency Graph

```
4.1 Expand ResourceKind + ResourceSummary
 │
 ├─► 4.2 Implement all summary structs
 │    │
 │    └─► 4.3 Generalize ResourceWatcher
 │         │
 │         └─► 4.10 Wire in App (final integration)
 │
 ├─► 4.4 Extend Command enum + InputMode
 │    │
 │    ├─► 4.5 ResourceListPane (filter, sort)
 │    │
 │    ├─► 4.6 Detail + YAML panes
 │    │
 │    └─► 4.8 Resource Switcher
 │
 ├─► 4.7 ActionExecutor (crystal-core)
 │
 └─► 4.9 Overlays (confirm, toast, help)
```

Steps 4.1 → 4.2 → 4.3 are sequential (each depends on the previous).
Steps 4.4–4.9 can be parallelized after 4.1 is done.
Step 4.10 is the final integration pass that wires everything together.

## All Files Touched

```
crates/
├── crystal-core/
│   └── src/
│       ├── resource.rs           # MODIFY — expand ResourceSummary, add DetailSection
│       ├── resources/
│       │   ├── mod.rs            # NEW — resource registry, re-exports
│       │   ├── pod.rs            # NEW — refactor PodSummary here
│       │   ├── deployment.rs     # NEW
│       │   ├── service.rs        # NEW
│       │   ├── statefulset.rs    # NEW
│       │   ├── daemonset.rs      # NEW
│       │   ├── job.rs            # NEW
│       │   ├── cronjob.rs        # NEW
│       │   ├── configmap.rs      # NEW
│       │   ├── secret.rs         # NEW
│       │   ├── ingress.rs        # NEW
│       │   ├── node.rs           # NEW
│       │   ├── namespace.rs      # NEW
│       │   ├── pv.rs             # NEW
│       │   └── pvc.rs            # NEW
│       ├── actions.rs            # NEW — ActionExecutor
│       └── informer.rs           # MODIFY — generalize watcher
├── crystal-tui/
│   └── src/
│       ├── pane.rs               # MODIFY — extend ResourceKind, PaneCommand
│       └── widgets/
│           ├── resource_list.rs  # MODIFY — filter bar, sort indicators
│           ├── breadcrumb.rs     # NEW — navigation breadcrumb
│           ├── confirm_dialog.rs # NEW — confirmation popup
│           └── toast.rs          # NEW — notification toast
├── crystal-app/
│   └── src/
│       ├── command.rs            # MODIFY — resource action commands
│       ├── keybindings.rs        # MODIFY — new modes + bindings
│       ├── app.rs                # MODIFY — multi-watcher, action routing
│       ├── resource_switcher.rs  # NEW — command palette
│       └── panes/
│           ├── resource_list.rs  # MODIFY — filter, sort support
│           ├── resource_detail.rs # NEW — detail pane
│           ├── yaml_pane.rs       # NEW — YAML view pane
│           └── help.rs           # MODIFY — context-sensitive
└── crystal-config/
    └── src/lib.rs                # MODIFY — resource keybinding config
```

## Resource Columns Reference

| Resource | Columns |
|----------|---------|
| Pod | NAME, READY, STATUS, RESTARTS, AGE, NODE |
| Deployment | NAME, READY, UP-TO-DATE, AVAILABLE, AGE |
| Service | NAME, TYPE, CLUSTER-IP, EXTERNAL-IP, PORTS, AGE |
| StatefulSet | NAME, READY, AGE |
| DaemonSet | NAME, DESIRED, CURRENT, READY, AGE |
| Job | NAME, COMPLETIONS, DURATION, AGE |
| CronJob | NAME, SCHEDULE, SUSPEND, ACTIVE, LAST SCHEDULE |
| ConfigMap | NAME, DATA, AGE |
| Secret | NAME, TYPE, DATA, AGE |
| Ingress | NAME, CLASS, HOSTS, ADDRESS, PORTS, AGE |
| Node | NAME, STATUS, ROLES, AGE, VERSION |
| Namespace | NAME, STATUS, AGE |
| PV | NAME, CAPACITY, ACCESS MODES, STATUS, CLAIM |
| PVC | NAME, STATUS, VOLUME, CAPACITY, ACCESS MODES |

## Resource Actions Matrix

Not every action applies to every resource. Keybindings are silently ignored
when the action doesn't apply to the current resource kind.

| Action | Pods | Deploy | StatefulSet | DaemonSet | Jobs | CronJobs | Other |
|--------|------|--------|-------------|-----------|------|----------|-------|
| View YAML (y) | x | x | x | x | x | x | x |
| Describe (d) | x | x | x | x | x | x | x |
| Delete (Ctrl+d) | x | x | x | x | x | x | x |
| Logs (l) | x | | | | | | |
| Exec (e) | x | | | | | | |
| Scale (S) | | x | x | | | | |
| Restart (R) | | x | | | | | |

## Full Demo Checklist

- [ ] Navigate through all 14 resource types using `:` command palette
- [ ] Pod list → select pod → detail view opens in split pane
- [ ] Detail view shows Metadata, Status, Containers sections
- [ ] View YAML with syntax highlighting, search within YAML
- [ ] Delete a pod with confirmation dialog → toast on success
- [ ] Scale a deployment → toast shows new replica count
- [ ] Restart rollout on deployment
- [ ] Filter pods with `/`, clear filter with Esc
- [ ] Sort by different columns with `s`
- [ ] Toggle all-namespaces view with `a`
- [ ] `?` shows context-sensitive help (different for pods vs deployments)
- [ ] Resource actions are ignored when not applicable (e.g., `l` on a ConfigMap does nothing)
- [ ] Multiple panes: pods list left, detail right, YAML below — all independent
