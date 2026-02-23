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
- **Overlays:** Namespace and context switching overlays let you change the active namespace or kubeconfig context without leaving the keyboard.
- **Resource Actions:** Supports resource actions such as delete with confirmation, deployment rollout restart, and debug mode toggling.
  - **Debug mode** (`Ctrl+Alt+d` on a Pod): patches the owner Deployment to replace the container command with `sleep infinity`, letting you exec in for investigation. Re-press to restore the original command.
  - **Root debug mode** (`F5` on a Pod): same as debug mode but additionally sets `securityContext.runAsUser: 0` for root access. Both modes preserve the original command and securityContext as annotations and restore them on exit.
- **Context Awareness:** Shows a status bar with mode hints plus current cluster and namespace. Toast notifications, confirmation dialogs, and a context-sensitive help overlay (`F1`) are available throughout.
- **Resource Switcher:** Press `:` to open the command palette and jump to any resource kind instantly.
- **Resilient:** Works even without a cluster connection by showing a clear error state in the resource view.

## What it does not do yet

- **General Purpose Shell:** Opens embedded terminal panes for general shell work.
- **Interactive Scale:** No interactive scale workflow yet (scale action scaffolding exists but is not wired through the UI).
- **Plugin System:** No plugin system yet (WASM-based plugin system is planned).

KubeTile is under active development as an incremental learning project, so the feature set is intentionally focused on the foundation of the UI and core Kubernetes browsing flow.
