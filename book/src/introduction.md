# KubeTile

KubeTile is a terminal-based Kubernetes workspace focused on fast, 
keyboard-driven navigation and a flexible pane-and-tab layout. 
It aims to make everyday cluster inspection feel like working in a tiled, 
multi-view console without leaving the terminal.

## What it does today

- **Live Resource Lists:** Connects to your current Kubernetes context and shows live resource lists across multiple Kubernetes kinds (including Pods, Deployments, Services, StatefulSets, DaemonSets, Jobs, CronJobs, ConfigMaps, Secrets, Ingresses, Nodes, Namespaces, PVs, and PVCs).
- **Flexible Layout:** Lets you split the screen into multiple panes, move focus between them, and work with tabs including fullscreen and close operations.
- **Efficient Workflows:** Provides resource-list workflows like filter input, column sorting, and all-namespaces toggling.
- **Deep Inspection:** Opens detail-oriented views from selections, including YAML and describe output in dedicated panes.
- **Integrated Terminal & Logs:** Streams pod logs and opens interactive exec sessions in dedicated panes.
- **Port Forwarding:** Supports port forwarding to pods so local tools can reach in-cluster services.
- **Resource Actions:** Supports resource actions such as delete with confirmation and deployment rollout restart.
- **Context Awareness:** Shows a status bar with mode hints plus current cluster and namespace.
- **Resilient:** Works even without a cluster connection by showing a clear error state in the resource view.

## What it does not do yet

- **General Purpose Shell:** Opens embedded terminal panes for general shell work.
- **Interactive Scale:** No interactive scale workflow yet (scale action scaffolding exists but is not wired through the UI).
- **Plugin System:** No plugin system yet (WASM-based plugin system is planned).

KubeTile is under active development as an incremental learning project, so the feature set is intentionally focused on the foundation of the UI and core Kubernetes browsing flow.
