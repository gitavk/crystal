# Step 5.4 — Terminal Manager (App Core)

> `feat(core): add TerminalManager to own session lifecycle in App Core`

## Goal

Implement `TerminalManager` — the App Core component that owns all active
terminal and exec sessions. This enforces the architecture rule that **panes
never talk to backends directly**. The TUI requests sessions via Commands, and
reads screen state via `RenderContext`.

This step also defines the `Command` variants for terminal operations.

## Files

| File | Action |
|------|--------|
| `crates/crystal-core/src/terminal_manager.rs` | NEW — session owner |
| `crates/crystal-core/src/command.rs` | UPDATE — add Terminal/Exec/Logs/PortForward commands |

## Command Variants

```rust
// crates/crystal-core/src/command.rs — new variants for Stage 5

enum Command {
    // ... existing variants from Stage 3/4 ...

    // Terminal lifecycle
    TerminalSpawn { context: ContextEnv },
    TerminalClose { session_id: SessionId },
    TerminalResize { session_id: SessionId, cols: u16, rows: u16 },
    TerminalInput { session_id: SessionId, bytes: Vec<u8> },

    // Exec lifecycle
    ExecStart { pod: String, namespace: String, container: Option<String>, command: Vec<String> },
    ExecClose { session_id: SessionId },

    // Logs
    LogsStart { request: LogRequest },
    LogsStop { stream_id: StreamId },

    // Port forwarding
    PortForwardStart { pod: String, namespace: String, local_port: u16, remote_port: u16 },
    PortForwardStop { forward_id: ForwardId },
}
```

## Data Structures

```rust
// crates/crystal-core/src/terminal_manager.rs

pub type SessionId = u64;

pub struct TerminalManager {
    terminals: HashMap<SessionId, TerminalSession>,
    next_id: u64,
}

struct TerminalSession {
    pty: PtySession,
    vt_parser: VtParser,
    title: String,
    kind: SessionKind,
}

pub enum SessionKind {
    Shell,
    Exec {
        pod: String,
        container: String,
        namespace: String,
    },
}
```

## Operations

```rust
impl TerminalManager {
    pub fn new() -> Self { /* ... */ }

    /// Spawn a new cluster-aware shell session.
    /// Called when App Core handles Command::TerminalSpawn.
    pub fn spawn_shell(
        &mut self,
        ctx: ContextEnv,
        size: (u16, u16),
    ) -> anyhow::Result<SessionId> { /* ... */ }

    /// Register an exec session (ExecSession created by crystal-core::exec).
    pub fn spawn_exec(
        &mut self,
        exec: ExecSession,
        pod_info: PodInfo,
    ) -> SessionId { /* ... */ }

    /// Forward input bytes to a session's PTY/exec.
    pub fn write_input(
        &mut self,
        id: SessionId,
        data: &[u8],
    ) -> anyhow::Result<()> { /* ... */ }

    /// Resize a session's PTY and VT parser.
    pub fn resize(
        &mut self,
        id: SessionId,
        cols: u16,
        rows: u16,
    ) -> anyhow::Result<()> { /* ... */ }

    /// Poll output from a session's PTY, feed to VT parser.
    /// Called on each tick by the render loop.
    pub fn poll_output(
        &mut self,
        id: SessionId,
    ) -> anyhow::Result<()> { /* ... */ }

    /// Poll all sessions. Returns IDs of sessions that have exited.
    pub fn poll_all(&mut self) -> Vec<SessionId> { /* ... */ }

    /// Close and clean up a session.
    pub fn close(
        &mut self,
        id: SessionId,
    ) -> anyhow::Result<()> { /* ... */ }

    /// Get VT screen state for rendering (read-only borrow for TUI).
    pub fn screen(
        &self,
        id: SessionId,
    ) -> Option<&vt100::Screen> { /* ... */ }

    /// Get session metadata (kind, title) for display.
    pub fn session_info(
        &self,
        id: SessionId,
    ) -> Option<(&SessionKind, &str)> { /* ... */ }
}
```

## Session Lifecycle

```
Command::TerminalSpawn
   → TerminalManager::spawn_shell()
   → PtySession::spawn() with ContextEnv
   → Returns SessionId
   → App creates TerminalView pane with that SessionId

Each tick:
   → TerminalManager::poll_all()
   → For each session: read PTY output, feed to VtParser
   → Detect exited processes → emit Event::TerminalExited

Command::TerminalInput { session_id, bytes }
   → TerminalManager::write_input()
   → PtySession::write()

Command::TerminalClose { session_id }
   → TerminalManager::close()
   → PtySession::kill()
   → Remove from HashMap
```

## Tests

- `spawn_shell()` returns a valid SessionId, session appears in the map
- `spawn_shell()` twice returns different SessionIds
- `write_input()` with invalid SessionId returns an error
- `close()` removes the session from the map
- `screen()` returns `None` for unknown SessionId
- `poll_all()` returns exited session IDs after kill
- `resize()` propagates to both PTY and VT parser

## Demo

- [ ] Spawn a shell session via Command, get SessionId
- [ ] Write `ls\n` via Command::TerminalInput, poll_output, read screen — shows listing
- [ ] Close the session via Command::TerminalClose, verify screen() returns None
