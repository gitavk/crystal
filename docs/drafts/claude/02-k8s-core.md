# Stage 2 — Kubernetes Core Integration

## Goal

Connect to a Kubernetes cluster using kube-rs, list namespaces and pods, and
display them in a basic scrollable list. Establish the data layer patterns that
all future resource views will follow.

## Philosophy: Read-Only First

All K8s operations in this stage are **strictly read-only**. Zero write
operations, zero mutations. The app must start and remain usable even without a
cluster. This safety-first approach lets us build confidence in the data layer
before adding any destructive capabilities later.

## K8s Module Rules

- Wraps kube-rs — no other K8s libraries
- Read-only operations only (list, watch, get)
- No UI code in the K8s module
- No terminal code in the K8s module
- Panes never talk to K8s directly — all data flows through App Core

## Prerequisites

- Stage 1 complete (TUI skeleton renders and quits)
- Access to a Kubernetes cluster (minikube, kind, or remote) — app runs without one too

## YouTube Episodes

1. **"Talking to Kubernetes from Rust — kube-rs basics"**: client, API calls
2. **"Live Cluster Data in the TUI"**: async data fetching, rendering lists
3. **"Watch & React — Kubernetes Informers"**: real-time updates

## New/Modified Files

```
crates/
├── kubeforge-core/
│   └── src/
│       ├── lib.rs                 # re-exports
│       ├── client.rs              # KubeClient wrapper
│       ├── resource.rs            # GenericResource enum/trait
│       ├── informer.rs            # Watch/informer manager
│       ├── context.rs             # Context Resolver (single source of truth)
│       └── error.rs               # Custom error types
├── kubeforge-app/
│   └── src/
│       ├── app.rs                 # Add K8s client, data state
│       └── state.rs               # NEW — resource list state (selected index, items)
└── kubeforge-tui/
    └── src/
        ├── lib.rs
        ├── layout.rs              # Updated — body now has sidebar + content
        └── widgets/
            ├── mod.rs
            ├── resource_list.rs   # Scrollable list widget
            └── namespace_selector.rs
```

## Tasks

### 2.1 Add kube-rs Dependencies

```toml
# workspace Cargo.toml — add to [workspace.dependencies]
kube = { version = "0.98", features = ["client", "runtime", "derive"] }
k8s-openapi = { version = "0.23", features = ["latest"] }
futures = "0.3"
```

### 2.2 Define Cluster Context Model

```rust
// crates/kubeforge-core/src/context.rs

/// Lightweight representation of the active cluster connection.
/// This is the single source of truth for context across the app.
pub struct ClusterContext {
    pub name: String,
    pub namespace: String,
}

/// The Context Resolver owns the active cluster + namespace,
/// produces environment variables, and notifies dependents on change.
pub struct ContextResolver {
    active: Option<ClusterContext>,
}

impl ContextResolver {
    pub fn resolve(&self) -> Option<&ClusterContext> { /* ... */ }

    /// Environment variables injected into terminals, exec sessions, plugins.
    /// Contract: KUBECONFIG, K8S_CONTEXT, K8S_NAMESPACE — no more, no less.
    pub fn env_vars(&self) -> Vec<(String, String)> { /* ... */ }

    pub fn set_context(&mut self, ctx: ClusterContext) { /* ... */ }
    pub fn set_namespace(&mut self, ns: &str) { /* ... */ }
}
```

**Context update rules:**
- New pane → receives current context at creation time
- Context change → existing panes are **not** restarted (predictable behavior)
- New panes created after a switch use the updated context

### 2.3 Implement KubeClient Wrapper

```rust
// crates/kubeforge-core/src/client.rs

pub struct KubeClient {
    client: kube::Client,
    current_namespace: String,
    current_context: String,
}

impl KubeClient {
    /// Create from default kubeconfig
    pub async fn from_kubeconfig() -> anyhow::Result<Self> { /* ... */ }

    /// Create from specific kubeconfig path + context
    pub async fn from_config(path: &Path, context: &str) -> anyhow::Result<Self> { /* ... */ }

    /// List all namespaces
    pub async fn list_namespaces(&self) -> anyhow::Result<Vec<String>> { /* ... */ }

    /// List available kube contexts (for future context switching)
    pub async fn list_contexts(&self) -> anyhow::Result<Vec<String>> { /* ... */ }

    /// List pods in current namespace (or all namespaces)
    pub async fn list_pods(&self, namespace: Option<&str>) -> anyhow::Result<Vec<PodSummary>> { /* ... */ }

    /// Switch namespace
    pub fn set_namespace(&mut self, ns: &str) { /* ... */ }

    /// Get current context name
    pub fn context(&self) -> &str { /* ... */ }

    /// Get current namespace
    pub fn namespace(&self) -> &str { /* ... */ }
}
```

**K8s capabilities (v0 — this stage):**
- Load kubeconfig
- Detect current context
- List namespaces
- List pods by namespace
- Watch pod updates

**Not yet:** exec, logs, delete, scale, or any mutation.

### 2.4 Define Resource Summary Types

```rust
// crates/kubeforge-core/src/resource.rs

/// Lightweight summary for display in lists — avoids holding full API objects
pub struct PodSummary {
    pub name: String,
    pub namespace: String,
    pub status: PodPhase,
    pub ready: String,        // "1/1", "2/3"
    pub restarts: i32,
    pub age: Duration,
    pub node: Option<String>,
}

pub enum PodPhase {
    Running,
    Pending,
    Succeeded,
    Failed,
    Unknown,
}

/// Trait for converting K8s API objects to display summaries
pub trait ResourceSummary: Send + Sync {
    fn name(&self) -> &str;
    fn namespace(&self) -> Option<&str>;
    fn status_display(&self) -> String;
    fn age(&self) -> Duration;
    fn columns(&self) -> Vec<(&str, String)>; // (header, value) for table display
}
```

### 2.5 Implement Informer/Watcher

```rust
// crates/kubeforge-core/src/informer.rs
use kube::runtime::watcher;

pub struct ResourceWatcher<K: kube::Resource> {
    // Wraps kube-rs watcher with a local cache
    // Sends updates through a channel to the TUI
}

impl<K> ResourceWatcher<K>
where
    K: kube::Resource + Clone + DeserializeOwned + Debug + Send + 'static,
{
    pub fn new(api: Api<K>) -> (Self, mpsc::UnboundedReceiver<ResourceEvent<K>>) { /* ... */ }
    pub async fn run(&self) -> anyhow::Result<()> { /* ... */ }
}

pub enum ResourceEvent<K> {
    Added(K),
    Modified(K),
    Deleted(K),
    Restarted(Vec<K>),
    Error(String),
}
```

**Watcher rules:**
- All watches must be cancellable
- Background tasks must self-terminate when the owning pane closes
- No unbounded buffers — cap event history

### 2.6 Add App State Management

```rust
// crates/kubeforge-app/src/state.rs

pub struct ResourceListState {
    pub items: Vec<Vec<String>>,    // rows of column values
    pub headers: Vec<String>,       // column headers
    pub selected: Option<usize>,    // cursor position
    pub scroll_offset: usize,
    pub loading: bool,
    pub error: Option<String>,
}

impl ResourceListState {
    pub fn next(&mut self) { /* move selection down, wrap around */ }
    pub fn previous(&mut self) { /* move selection up, wrap around */ }
    pub fn selected_item(&self) -> Option<&Vec<String>> { /* ... */ }
}
```

### 2.7 Implement Resource List Widget

```rust
// crates/kubeforge-tui/src/widgets/resource_list.rs

/// A table widget that renders resource summaries with:
/// - Column headers (bold)
/// - Selectable rows with highlight
/// - Status column with color coding (green=Running, red=Failed, yellow=Pending)
/// - Scrollbar when items exceed viewport
/// - "Loading..." / error / empty states
pub struct ResourceListWidget<'a> {
    state: &'a ResourceListState,
    title: &'a str,
}
```

**UX rules:**
- Selected row is highlighted
- Empty state shown when no resources exist
- Loading state shown while fetching
- No spinners needed — text is fine

### 2.8 Wire It All Together in app.rs

```rust
// Update App struct:
pub struct App {
    pub running: bool,
    pub tick_rate: Duration,
    pub kube_client: Option<KubeClient>,
    pub context_resolver: ContextResolver,
    pub pod_list: ResourceListState,
    pub current_view: View, // enum { Pods, Namespaces, ... }
    pub namespaces: Vec<String>,
    pub selected_namespace: usize,
}

// Key bindings for this stage:
// j / Down  → next item
// k / Up    → previous item
// Enter     → (noop for now, will open detail view later)
// :         → namespace selector popup
// 1         → switch to Pods view
// q         → quit
```

**Data flow (non-negotiable pattern):**
1. App initializes K8s client
2. Fetches active context → stores in ContextResolver
3. Pane requests data via App Core
4. App Core calls K8s module
5. Pane receives snapshot updates
6. UI renders context info in status bar

Panes **never** talk to K8s directly.

### 2.9 Namespace Selector Widget

```rust
/// A popup overlay widget for switching namespaces
/// - Fuzzy filterable list
/// - "All Namespaces" option at top
/// - Shows current selection
/// - Esc to close, Enter to select
pub struct NamespaceSelectorWidget { /* ... */ }
```

## Key Design Decisions

1. **Read-only first**: Zero mutations until explicitly unlocked in later stages.
   Invalid kubeconfig shows a warning, but the app still starts and remains usable.
2. **Summaries over full objects**: Never store full K8s API objects in TUI state.
   Convert to lightweight summary types at the boundary.
3. **Channel-based updates**: The watcher runs in a background tokio task and
   sends events through an mpsc channel. The TUI event loop merges these with
   key events.
4. **Generic resource pattern**: `ResourceSummary` trait allows uniform rendering
   for any resource type. New resources only need a summary impl.
5. **Context Resolver as single source of truth**: One component owns the active
   cluster + namespace. All panes and terminals read from it. No component may
   set K8s environment variables on its own.
6. **Pane isolation**: Pane failures do not crash the app. Panes never query each
   other directly — all cross-pane data flows through App Core.

## Error Handling Rules

- Invalid/missing kubeconfig → warning in UI, app still starts
- Cluster unreachable → degrade gracefully, show error in pane
- Network error during watch → retry with backoff
- Never crash on cluster issues — the TUI must always remain responsive
- Use error panes, not panics

## Performance Safeguards

- No unbounded buffers (cap event history, log lines, etc.)
- No blocking calls on the UI thread
- All watches cancellable
- Background tasks must self-terminate when owning component closes

## Tests

- Unit: `PodSummary::columns()` returns correct headers and values
- Unit: `ResourceListState` navigation wraps correctly
- Unit: `ContextResolver` produces correct env vars
- Unit: `ContextResolver` update rules (new pane gets current, old panes unchanged)
- Integration: `KubeClient::list_namespaces()` against a kind cluster
- Integration: `KubeClient::list_pods()` returns data matching `kubectl get pods`
- Integration: App starts without a kubeconfig (graceful degradation)

## Demo Checklist

- [ ] Launch app → auto-connects to current kubeconfig context
- [ ] Launch app without kubeconfig → starts with warning, no crash
- [ ] Status bar shows cluster name and current namespace
- [ ] Clear indicator when no cluster is available
- [ ] Pod list populates with real pods
- [ ] Navigate with j/k, see highlight move
- [ ] Open namespace selector with `:`, filter, switch namespace
- [ ] Pod list refreshes when a pod is created/deleted (live watch)
- [ ] Empty state renders correctly when namespace has no pods

## Future K8s Roadmap (Out of Scope for Stage 2)

These capabilities build on patterns established here but are implemented in
later stages:

- **Logs pane**: Stream pod logs with follow mode, capped line buffer, clean
  start/stop lifecycle. Context flows from pod selection through App Core.
- **Exec pane**: Interactive shell in a container via PTY forwarding. Context-
  aware with injected env vars. Global shortcuts remain active.
- **Context switching**: Keyboard-only cluster and namespace switching via
  dedicated selector panes (not modal dialogs). Global keybinds `c` / `n`.
- **XRay view**: Dependency visualization — services → pods → workloads as a
  tree with ASCII connectors. Read-only, answers "what depends on this?"
- **Pulse view**: Aggregated cluster health dashboard — node/pod counts, restart
  rate, warning events. Periodic refresh, no Prometheus required.
- **Hardening**: Resource limits, error isolation, context-window discipline
  (one feature = one module, each module < ~500 LOC), structured logging.

## Commit Messages

```
feat(core): add kube-rs client wrapper with kubeconfig support
feat(core): define ClusterContext and ContextResolver
feat(core): define ResourceSummary trait and PodSummary type
feat(core): implement resource watcher with channel-based updates
feat(app): add resource list state with navigation
feat(tui): implement scrollable resource list table widget
feat(tui): add namespace selector popup with fuzzy filter
feat(app): wire k8s client and context resolver to TUI event loop
```
