# Step 4.3 — Generalize ResourceWatcher

> `feat(core): generalize ResourceWatcher for any k8s resource type`

## Goal

The existing `ResourceWatcher::watch_pods()` only watches pods. Generalize it
to watch any Kubernetes resource type using the same snapshot-based pattern.
The App manages multiple concurrent watchers (one per active resource view)
and cancels them on resource type switch.

## Files

| File | Action |
|------|--------|
| `crates/crystal-core/src/informer.rs` | UPDATE — generic `watch<K, S>()` method |
| `crates/crystal-app/src/app.rs` | UPDATE — multi-watcher management |
| `crates/crystal-app/src/event.rs` | UPDATE — generic AppEvent variant |

## Existing State

```rust
// crates/crystal-core/src/informer.rs — current implementation

pub enum ResourceEvent<S> {
    Updated(Vec<S>),
    Error(String),
}

pub struct ResourceWatcher;

impl ResourceWatcher {
    pub async fn watch_pods(
        api: Api<Pod>,
        tx: mpsc::Sender<ResourceEvent<PodSummary>>,
        cancel: CancellationToken,
    ) {
        // Maintains HashMap<String, PodSummary> snapshot
        // Uses kube 3.0 watcher Event: Apply, Delete, Init, InitApply, InitDone
        // Sends full snapshot on each change
    }
}
```

## Generic Watcher

```rust
// crates/crystal-core/src/informer.rs — generalized

impl ResourceWatcher {
    /// Watch any Kubernetes resource type and emit summary snapshots.
    ///
    /// Type parameters:
    /// - K: the k8s-openapi resource type (Pod, Deployment, etc.)
    /// - S: the summary struct (PodSummary, DeploymentSummary, etc.)
    ///
    /// Requirements:
    /// - S must implement From<K> for conversion
    /// - S must implement ResourceSummary for name extraction (snapshot keys)
    pub async fn watch<K, S>(
        api: Api<K>,
        tx: mpsc::Sender<ResourceEvent<S>>,
        cancel: CancellationToken,
    ) where
        K: Resource<DynamicType = ()>
            + Clone
            + DeserializeOwned
            + Debug
            + Send
            + 'static,
        S: ResourceSummary + From<K> + 'static,
    {
        // Same algorithm as watch_pods, now generic:
        let mut snapshot: HashMap<String, S> = HashMap::new();

        // 1. Start kube watcher stream
        // 2. On Event::Apply / Event::InitApply:
        //    - Convert K → S via From
        //    - Insert into snapshot using S.name() + S.namespace() as key
        //    - Send ResourceEvent::Updated(snapshot.values().cloned().collect())
        // 3. On Event::Delete:
        //    - Remove from snapshot
        //    - Send Updated
        // 4. On Event::Init:
        //    - Clear snapshot
        // 5. On Event::InitDone:
        //    - Send full snapshot
        // 6. On cancel signal: break loop
    }
}
```

The existing `watch_pods()` can be kept as a convenience wrapper or removed
in favor of the generic version. Prefer removing it to avoid duplication.

## Multi-Watcher Management in App

```rust
// crates/crystal-app/src/app.rs

pub struct App {
    // ... existing fields ...

    /// Active watchers keyed by pane ID (not resource kind).
    /// Each pane showing a resource list has its own watcher.
    /// When a pane switches resource type, its watcher is cancelled
    /// and a new one spawned.
    active_watchers: HashMap<PaneId, CancellationToken>,
}

impl App {
    /// Start watching a resource type for a specific pane.
    /// Cancels any existing watcher for that pane first.
    async fn start_watcher_for_pane(&mut self, pane_id: PaneId, kind: &ResourceKind) {
        // 1. Cancel existing watcher if any
        if let Some(token) = self.active_watchers.remove(&pane_id) {
            token.cancel();
        }

        // 2. Create new cancellation token
        let token = CancellationToken::new();
        self.active_watchers.insert(pane_id, token.clone());

        // 3. Build Api<K> for the resource kind
        //    (requires matching ResourceKind → k8s-openapi type)
        // 4. Spawn ResourceWatcher::watch::<K, S>()
        // 5. Forward events into AppEvent channel
    }

    /// Switch a pane to a different resource type.
    /// Called from resource switcher confirmation.
    async fn switch_resource(&mut self, pane_id: PaneId, kind: ResourceKind) {
        // 1. Start new watcher
        self.start_watcher_for_pane(pane_id, &kind).await;
        // 2. Update pane's ViewType to ResourceList(kind)
        // 3. Set pane's ResourceListState to loading
    }
}
```

## Dispatching by ResourceKind

The generic watcher requires matching `ResourceKind` to concrete types at
the call site. Use a match statement in `start_watcher_for_pane`:

```rust
match kind {
    ResourceKind::Pods => {
        let api: Api<Pod> = Api::namespaced(client.clone(), ns);
        tokio::spawn(ResourceWatcher::watch::<Pod, PodSummary>(api, tx, token));
    }
    ResourceKind::Deployments => {
        let api: Api<Deployment> = Api::namespaced(client.clone(), ns);
        tokio::spawn(ResourceWatcher::watch::<Deployment, DeploymentSummary>(api, tx, token));
    }
    // ... all 14 types
}
```

This match is the one place where concrete types are wired together.
It lives in crystal-app (not crystal-core) because it's integration logic.

## AppEvent Extension

```rust
// crates/crystal-app/src/event.rs

pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
    /// Resource update for a specific pane.
    /// The Vec<Vec<String>> is pre-rendered rows (via ResourceSummary::row()).
    /// This erases the generic S type so AppEvent doesn't need type params.
    ResourceUpdate {
        pane_id: PaneId,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    ResourceError {
        pane_id: PaneId,
        error: String,
    },
}
```

The watcher bridge converts `ResourceEvent<S>` into `AppEvent::ResourceUpdate`
by calling `S::columns()` and `s.row()` on each item. This erases the generic
type at the boundary so the rest of the app works with plain strings.

## Tests

- Generic watcher compiles for Pod, Deployment, Service (type-level test)
- Watcher sends Updated event on Apply
- Watcher sends Updated event (item removed) on Delete
- Watcher clears snapshot on Init, sends full snapshot on InitDone
- CancellationToken stops the watcher loop
- Multi-watcher: two concurrent watchers for different panes don't interfere
- Switching resource type cancels old watcher before starting new one
- AppEvent::ResourceUpdate rows match expected column count

## Demo

- [ ] Start app → pods load (existing behavior preserved)
- [ ] Switch to Deployments → pod watcher cancelled, deployment watcher starts
- [ ] Switch back to Pods → deployment watcher cancelled, pods reload
- [ ] Two panes showing different resource types simultaneously
