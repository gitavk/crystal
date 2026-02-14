# Step 5.6 — Pod Exec & Exec View

> `feat(core): add pod exec via kube-rs WebSocket API`

## Goal

Implement `ExecSession` in `crystal-core` for executing commands inside pod
containers via the Kubernetes exec API (WebSocket). Wrap it in `ExecView` for
TUI display. Exec sessions are registered with `TerminalManager` so they follow
the same lifecycle as shell terminals.

## Files

| File | Action |
|------|--------|
| `crates/crystal-core/src/exec.rs` | NEW — ExecSession (K8s WebSocket) |
| `crates/crystal-tui/src/views/exec_view.rs` | NEW — ExecView (pure renderer) |

## ExecSession

```rust
// crates/crystal-core/src/exec.rs

pub struct ExecSession {
    // Uses kube-rs attach/exec API (WebSocket-based)
    // stdin: AsyncWrite, stdout: AsyncRead, resize channel
}

impl ExecSession {
    pub async fn start(
        client: &KubeClient,
        pod_name: &str,
        namespace: &str,
        container: Option<&str>,  // None = first container
        command: Vec<String>,     // default: ["/bin/sh"]
    ) -> anyhow::Result<Self> { /* ... */ }

    pub async fn write(&mut self, data: &[u8]) -> anyhow::Result<()> { /* ... */ }

    pub async fn read(&mut self) -> anyhow::Result<Vec<u8>> { /* ... */ }

    pub async fn resize(&mut self, cols: u16, rows: u16) -> anyhow::Result<()> { /* ... */ }

    pub async fn close(self) -> anyhow::Result<()> { /* ... */ }
}
```

## Exec Flow

```
User selects pod → presses `e`
   ↓
Command::ExecStart { pod, namespace, container, command }
   ↓
App Core: ExecSession::start(client, pod, ns, container, ["/bin/sh"])
   ↓
TerminalManager::spawn_exec(exec_session, pod_info) → SessionId
   ↓
App creates ExecView pane with SessionId
   ↓
Auto-enters Insert mode
```

## ExecView

```rust
// crates/crystal-tui/src/views/exec_view.rs

/// Pure renderer — delegates to TerminalManager for screen state.
/// Title bar shows pod/container info.
/// Ctrl+d or `exit` → returns to resource view.
pub struct ExecView {
    session_id: SessionId,
    pod_name: String,
    container: String,
    namespace: String,
}
```

ExecView is nearly identical to TerminalView but with:
- Different title format: `[exec:pod-name/container @ namespace]`
- Auto-close behavior: when the exec process exits, the pane auto-closes
  after showing the exit message for 2 seconds

## Container Selection

When a pod has multiple containers:
1. `Command::ExecStart` with `container: None` → show container picker overlay
2. Container picker lists all containers in the pod
3. User selects one → re-dispatch with `container: Some(name)`

## Error Handling

- **Pod not running**: ExecSession::start returns error → toast: "Pod is not running"
- **Container not found**: Error → toast: "Container not found in pod"
- **WebSocket disconnect**: `read()` returns error → view shows disconnect message,
  toast offers reconnect
- **Permission denied**: API error → toast: "Exec not allowed (RBAC)"

## Tests

- `ExecSession::start()` with valid pod connects (integration, requires kind cluster)
- `write()` + `read()` round-trips bytes through exec
- `resize()` sends resize message over WebSocket
- `close()` cleanly terminates the session
- Error case: start with non-existent pod returns error
- Error case: start with stopped pod returns error

## Demo

- [ ] Select a running pod, press `e` → exec session opens
- [ ] Shell prompt appears inside exec pane
- [ ] Run commands (`ls`, `cat /etc/hostname`) — output renders correctly
- [ ] `vim` works inside exec (full terminal emulation)
- [ ] `exit` or Ctrl+d → pane closes, focus returns to resource list
- [ ] Multi-container pod: container picker appears, select one
