# Crystal

Crystal is a terminal-based Kubernetes workspace focused on fast, keyboard-driven navigation and a flexible pane-and-tab layout. It aims to make everyday cluster inspection feel like working in a tiled, multi-view console without leaving the terminal.

**What it does today**
1. Connects to your current Kubernetes context and shows a live Pods list.
2. Lets you split the screen into multiple panes and move focus between them.
3. Supports tabs, fullscreening a pane, and closing panes or tabs.
4. Provides a help pane that shows active shortcuts for the current context.
5. Includes a namespace selector overlay to filter and switch namespaces.
6. Shows a status bar with mode hints plus current cluster and namespace.
7. Works even without a cluster connection by showing a clear error state in the Pods view.

**What it does not do yet**
1. No logs, exec, or terminal panes beyond placeholders.
2. No resource types beyond Pods in the live list view.
3. No plugins or command palette.

Crystal is under active development as an incremental learning project, so the feature set is intentionally focused on the foundation of the UI and core Kubernetes browsing flow.
