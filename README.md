# Crystal

Crystal is a terminal-based Kubernetes workspace focused on fast, keyboard-driven navigation and a flexible pane-and-tab layout. It aims to make everyday cluster inspection feel like working in a tiled, multi-view console without leaving the terminal.

**Prerequisites**
- `kubectl` must be installed and available in your `PATH` to use pod exec sessions.
- Crystal now checks this on startup and shows a notification if `kubectl` is missing.

**What it does today**
1. Connects to your current Kubernetes context and shows live resource lists across multiple Kubernetes kinds (including Pods, Deployments, Services, StatefulSets, DaemonSets, Jobs, CronJobs, ConfigMaps, Secrets, Ingresses, Nodes, Namespaces, PVs, and PVCs).
2. Lets you split the screen into multiple panes, move focus between them, and work with tabs including fullscreen and close operations.
3. Provides resource-list workflows like filter input, column sorting, and all-namespaces toggling.
4. Opens detail-oriented views from selections, including YAML and describe output in dedicated panes.
5. Streams pod logs and opens interactive exec sessions in dedicated panes.
6. Opens embedded terminal panes for general shell work.
7. Supports port forwarding to pods so local tools can reach in-cluster services.
8. Includes overlays for namespace switching, confirmation dialogs, transient toast notifications, context-sensitive help, and a resource switcher command palette.
9. Supports resource actions such as delete with confirmation and deployment rollout restart.
10. Shows a status bar with mode hints plus current cluster and namespace.
11. Works even without a cluster connection by showing a clear error state in the resource view.

**What it does not do yet**
1. No interactive scale workflow yet (scale action scaffolding exists but is not wired through the UI).
2. No plugin system.

Crystal is under active development as an incremental learning project, so the feature set is intentionally focused on the foundation of the UI and core Kubernetes browsing flow.
