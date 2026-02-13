# Step 5.1 — PTY Session Management

> `feat(terminal): create crystal-terminal crate with PTY session management`

## Goal

Create the `crystal-terminal` crate and implement `PtySession` — the low-level
wrapper around `portable-pty` that spawns shell processes, forwards I/O, and
handles resize/kill. Every terminal and exec feature builds on this.

## Files

| File | Action |
|------|--------|
| `crates/crystal-terminal/Cargo.toml` | NEW — crate manifest |
| `crates/crystal-terminal/src/lib.rs` | NEW — module exports |
| `crates/crystal-terminal/src/pty.rs` | NEW — PtySession implementation |

## Crate Setup

```toml
# crates/crystal-terminal/Cargo.toml
[package]
name = "crystal-terminal"
version = "0.1.0"
edition = "2021"

[dependencies]
portable-pty.workspace = true
vt100.workspace = true
tokio.workspace = true
crossterm.workspace = true
anyhow.workspace = true
tracing.workspace = true
```

Note: This crate is a backend **adapter** — it sits alongside `crystal-core`
under App Core in the architecture diagram. It has no dependency on `crystal-tui`.

## Data Structures

```rust
// crates/crystal-terminal/src/pty.rs

use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send>,
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
}

impl PtySession {
    pub fn spawn(
        shell: &str,          // e.g., "/bin/bash"
        cwd: Option<&Path>,
        env: HashMap<String, String>,
        size: (u16, u16),     // (cols, rows)
    ) -> anyhow::Result<Self> { /* ... */ }

    pub fn resize(&self, cols: u16, rows: u16) -> anyhow::Result<()> { /* ... */ }

    pub fn write(&mut self, data: &[u8]) -> anyhow::Result<()> { /* ... */ }

    pub fn read(&mut self, buf: &mut [u8]) -> anyhow::Result<usize> { /* ... */ }

    pub fn is_alive(&self) -> bool { /* ... */ }

    pub fn kill(&mut self) -> anyhow::Result<()> { /* ... */ }
}
```

## Spawn Logic

1. Create a `PtySize` from `(cols, rows)`
2. Open a new PTY pair via `native_pty_system().openpty(size)`
3. Build a `CommandBuilder` for the shell path
4. Set `cwd` if provided
5. Inject all env vars from the `HashMap`
6. Spawn the child process on the slave PTY
7. Take a reader and writer from the master PTY
8. Return `PtySession` owning all handles

## Read Semantics

- `read()` is **non-blocking** — returns 0 bytes if nothing available
- Caller (TerminalManager) polls on a tick interval
- Large reads are buffered internally by the OS pipe

## Tests

- `spawn()` with default shell creates a live process (`is_alive() == true`)
- `write()` sends bytes that appear in `read()` output (echo test)
- `resize()` does not error on valid dimensions
- `kill()` terminates the child, `is_alive()` returns false after kill
- `spawn()` with invalid shell path returns an error
- `spawn()` with custom env vars: child inherits them (write `echo $VAR`, read output)

## Demo

- [ ] Spawn a shell, write `echo hello\n`, read back output containing "hello"
- [ ] Resize to 40x10, verify no error
- [ ] Kill the session, confirm `is_alive()` returns false
