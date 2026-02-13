# Stage 5 — Context-Aware Terminal, Exec & Logs

## Goal

Implement the "Cluster-Aware Internal Terminal" — an embedded terminal emulator
that auto-configures KUBECONFIG, context, and namespace. Add `exec` into pods
and streaming `logs` as first-class features. This is the highest-value
differentiator for the app.

## Prerequisites

- Stage 4 complete (resource views with detail and actions)

## YouTube Episodes

1. **"Embedding a Terminal in a Rust TUI — PTY Basics"**: portable-pty, raw I/O
2. **"Context-Aware Shell — Auto-Configured kubectl"**: env injection
3. **"Live Logs & Exec — Streaming K8s Data"**: WebSocket, log tailing

---

## Design Rules

These rules align with the core architecture established in Stage 1.

### Terminal Is Context-Aware but Stateless
- The terminal crate holds no persistent state beyond the current PTY session
- All cluster/context state comes from App Core at spawn time
- Switching context in the app does not mutate running terminals

### All Actions Flow Through Commands
- Terminal operations (spawn, exec, logs, port-forward) are dispatched as
  `Command` variants through App Core
- The TUI never spawns a PTY or exec session directly
- Keyboard input in Insert mode becomes `Command::TerminalInput`

### No Kubernetes Logic in the Terminal Crate
- `crystal-terminal` handles PTY management and VT rendering only
- K8s exec, logs, and port-forward live in `crystal-core`
- The terminal crate is a backend **adapter**, not a K8s client

### UI Is a Pure Renderer
- `TerminalView`, `LogsView`, and `ExecView` receive handles/data from
  App Core and render them — they do not own backend sessions
- Screen state flows from `TerminalManager` through `RenderContext`

### Plugins Cannot Crash the Core
- Future plugin hooks (custom shell init, log formatters) run sandboxed
- Plugin failures are caught and reported, never propagated

---

## Data Flow

```
Keyboard Input (Insert mode)
   ↓
Command::TerminalInput { session_id, bytes }
   ↓
App Core (TerminalManager)
   ↓
PtySession::write() / ExecSession::write()
   ↓
PTY/WebSocket output → VT parser update
   ↓
UI Re-render (TerminalView reads VT screen)
```

---

## Steps

| Step | File | Commit | Summary |
|------|------|--------|---------|
| 5.1 | [05a-pty-session.md](05a-pty-session.md) | `feat(terminal): create crystal-terminal crate with PTY session management` | New crate, PtySession spawn/read/write/resize/kill |
| 5.2 | [05b-vt-renderer.md](05b-vt-renderer.md) | `feat(terminal): implement VT100 screen to ratatui renderer` | vt100::Screen → ratatui Spans, cursor, scrollback, truecolor |
| 5.3 | [05c-context-env.md](05c-context-env.md) | `feat(terminal): build context-aware environment for cluster shells` | ContextEnv, env map, shell init script, PS1 prompt |
| 5.4 | [05d-terminal-manager.md](05d-terminal-manager.md) | `feat(core): add TerminalManager to own session lifecycle in App Core` | TerminalManager, SessionId, Command variants, session routing |
| 5.5 | [05e-terminal-view.md](05e-terminal-view.md) | `feat(tui): implement terminal view as pure renderer over TerminalManager` | TerminalView (pure renderer), scrollback, title bar |
| 5.6 | [05f-exec.md](05f-exec.md) | `feat(core): add pod exec via kube-rs WebSocket API` | ExecSession, WebSocket stdin/stdout, resize, ExecView |
| 5.7 | [05g-logs.md](05g-logs.md) | `feat(core): implement streaming log reader with tail/follow/reconnect` | LogStream, LogLine, LogRequest, LogsView with filter/scroll |
| 5.8 | [05h-port-forward.md](05h-port-forward.md) | `feat(core): add port forwarding support` | PortForward start/stop, local↔remote binding |
| 5.9 | [05i-input-mode.md](05i-input-mode.md) | `feat(app): implement Insert mode for terminal input passthrough` | Insert/Normal mode toggle, key routing, status bar hints |

## New Workspace Dependencies

```toml
# Cargo.toml [workspace.dependencies]
portable-pty = "0.8"
vt100 = "0.15"
```

## All Files Touched

```
crates/
├── crystal-terminal/              # NEW CRATE — terminal emulator (adapter)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── pty.rs                 # PTY management (portable-pty)
│       ├── vt.rs                  # VT100/ANSI parser (vt100 crate)
│       ├── renderer.rs            # Convert VT screen → ratatui spans
│       └── context_env.rs         # KUBECONFIG/context env builder
├── crystal-core/
│   └── src/
│       ├── terminal_manager.rs    # NEW — owns PTY/exec sessions, dispatches I/O
│       ├── command.rs             # UPDATE — Terminal*/Exec*/Logs*/PortForward* variants
│       ├── exec.rs                # NEW — pod exec via WebSocket
│       ├── logs.rs                # NEW — pod log streaming
│       └── port_forward.rs        # NEW — port forwarding
├── crystal-tui/
│   └── src/
│       └── views/
│           ├── terminal_view.rs   # NEW — terminal pane view (pure renderer)
│           ├── logs_view.rs       # NEW — log viewer (pure renderer)
│           └── exec_view.rs       # NEW — exec session view (pure renderer)
├── crystal-app/
│   └── src/
│       └── app.rs                 # UPDATE — Insert mode, Command routing
```

## Error Handling & Reconnection

- **PTY crash**: `TerminalManager` detects via `is_alive()` polling, sends
  `Event::TerminalExited`. View shows "[Process exited with code N] Press Enter to restart".
- **Exec disconnect**: WebSocket drops surface a toast with option to reconnect.
- **Log stream interruption**: Automatic reconnection with backoff. Status shows "Reconnecting...".
- **Port forward failure**: Bind errors reported immediately. Runtime disconnects trigger retry toast.

## Plugin Extensibility (Future)

Hooks reserved for the plugin system (Stage 7+):

- **`on_terminal_spawn`** — inject env vars or shell init commands
- **`on_log_line`** — transform/annotate log lines before display
- **`on_exec_start`** — modify exec command or add audit logging
- **`on_port_forward`** — register custom port-forward handlers

## Full Demo Checklist

- [ ] Open cluster-aware terminal in a pane (Alt+Enter or shortcut)
- [ ] Terminal shows custom prompt with cluster/namespace
- [ ] `kubectl get pods` works without manual config
- [ ] Select a pod → press `e` → exec into container
- [ ] Full terminal emulation (vim, top, etc. work inside exec)
- [ ] Select a pod → press `l` → streaming logs
- [ ] Logs auto-scroll, toggle with `f`
- [ ] Filter logs with `/`
- [ ] Multi-container pod: switch containers with `c`
- [ ] Open multiple terminals in split panes
- [ ] Port forward a service, access from browser
- [ ] Terminal recovers gracefully from process exit
- [ ] Exec reconnects after WebSocket disconnect
