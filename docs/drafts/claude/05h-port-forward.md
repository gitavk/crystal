# Step 5.8 — Port Forwarding

> `feat(core): add port forwarding support`

## Goal

Implement `PortForward` in `crystal-core` for forwarding a local port to a pod
port via the Kubernetes port-forward API. Port forwards run as background tasks
managed by App Core and are displayed in the status bar.

## Files

| File | Action |
|------|--------|
| `crates/crystal-core/src/port_forward.rs` | NEW — PortForward start/stop |

## Data Structures

```rust
// crates/crystal-core/src/port_forward.rs

pub type ForwardId = u64;

pub struct PortForward {
    id: ForwardId,
    local_port: u16,
    remote_port: u16,
    pod_name: String,
    namespace: String,
    handle: JoinHandle<()>,
}

impl PortForward {
    pub async fn start(
        client: &KubeClient,
        pod_name: &str,
        namespace: &str,
        local_port: u16,
        remote_port: u16,
    ) -> anyhow::Result<Self> { /* ... */ }

    pub async fn stop(self) -> anyhow::Result<()> { /* ... */ }

    pub fn local_port(&self) -> u16 { self.local_port }
    pub fn remote_port(&self) -> u16 { self.remote_port }
    pub fn pod_name(&self) -> &str { &self.pod_name }
}
```

## Port Forward Flow

```
User selects pod/service → presses `p` (or action menu)
   ↓
Prompt: "Local port: [8080]  Remote port: [80]"
   ↓
Command::PortForwardStart { pod, namespace, local_port, remote_port }
   ↓
App Core: PortForward::start() → spawns background task
   ↓
Toast: "Port forward active: localhost:8080 → pod:80"
   ↓
Status bar shows: "⇄ 8080→80"
```

## Error Handling

- **Port in use**: `start()` returns bind error → toast: "Port 8080 already in use"
- **Pod not running**: API error → toast: "Pod is not in Running state"
- **Runtime disconnect**: Background task detects closure → emit
  `Event::PortForwardDropped { id }` → toast with retry option
- **Permission denied**: API error → toast: "Port forward not allowed (RBAC)"

## Active Forwards Display

Active port forwards are shown in the status bar:
```
⇄ 8080→80 (my-pod)  ⇄ 3000→3000 (api-pod)
```

A global keybinding (e.g., `Alt+p`) opens a list of active forwards where the
user can stop individual ones.

## Tests

- `start()` with valid pod and available port succeeds (integration)
- `stop()` cleanly terminates the background task
- `start()` with port already in use returns error
- Multiple forwards can run simultaneously
- Forward stops automatically when pod is deleted

## Demo

- [ ] Select a pod, press `p` → port forward prompt appears
- [ ] Enter ports → forward starts, toast confirms
- [ ] Status bar shows active forward
- [ ] Open browser to `localhost:<port>` → traffic reaches pod
- [ ] Stop forward via `Alt+p` → status bar updates
- [ ] Forward a service (resolves to pod automatically)
