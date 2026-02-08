# Stage 1 — Project Scaffold & TUI Skeleton

## Goal

Set up a Cargo workspace with proper crate boundaries, CI pipeline, and a
minimal ratatui application that renders a blank screen with a status bar and
responds to `q` to quit. This is the "Hello World" of the project.

## Architecture

### Architecture Style

Event-driven, core + adapters:
- **Core** = state + logic
- **UI** = pure rendering (no business logic)
- **Backend services** = adapters (K8s, terminal, plugins)

This keeps context small and testable.

### Component Diagram

```
+-----------------------+
|        TUI            |
|  (zellij-like UI)     |
+-----------+-----------+
            |
            v
+-----------------------+
|     App Core          |
|  State + Commands     |
+-----------+-----------+
            |
   +--------+--------+
   |                 |
   v                 v
+--------+     +-------------+
| K8s    |     | Plugin Host |
| Client |     | (WASM/Dylib)|
+--------+     +-------------+
            |
            v
     +----------------+
     | Internal Term  |
     | (Context-Aware)|
     +----------------+
```

### Module Boundaries

- **app_core** — owns global state, dispatches commands; no UI code, no direct K8s calls
- **ui** — renders state, handles keyboard input; zero business logic
- **k8s** — wraps kube-rs; provides cluster, pod, exec, logs APIs; no UI awareness
- **terminal** — context-aware shell; injects KUBECONFIG + context; no K8s logic (uses k8s module)
- **plugins** — defines plugin API, loads plugins, sandboxes execution
- **config** — keyboard shortcuts, feature flags, user preferences

### Command Flow

```
Keyboard Input
   ↓
Command (enum)
   ↓
App Core
   ↓
State Mutation / Side Effect
   ↓
UI Re-render
```

No module skips this flow.

### Design Rules

- UI never mutates state directly
- All actions go through Commands
- Kubernetes access only via k8s module
- Terminal is context-aware but stateless
- Plugins cannot crash the core

## Prerequisites

- Rust toolchain installed (rustup, cargo)
- Git repository initialized

## Tutorial Sections

1. **Project Setup** — workspace, crates, CI
2. **First Pixels on Screen** — ratatui basics, TUI loop, event handling

## File Tree After This Stage

```
crystal/
├── Cargo.toml                     # Workspace root
├── Cargo.lock
├── .github/
│   └── workflows/
│       └── ci.yml                 # Lint, test, build
├── .gitignore
├── rustfmt.toml
├── clippy.toml
├── README.md
├── crates/
│   ├── crystal-app/             # Binary crate — TUI entry point
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs            # Entry point, terminal setup/teardown
│   │       ├── app.rs             # App state struct, tick loop
│   │       └── event.rs           # Crossterm event polling (async)
│   ├── crystal-core/            # Library — business logic, K8s client
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs             # Re-exports, empty for now
│   ├── crystal-tui/             # Library — UI components, layout
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── layout.rs          # Root layout (header, body, status bar)
│   │       └── theme.rs           # Color palette constants
│   └── crystal-config/          # Library — config parsing
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs             # Stub config struct
```

## Tasks

### 1.1 Initialize Cargo Workspace

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = [
    "crates/crystal-app",
    "crates/crystal-core",
    "crates/crystal-tui",
    "crates/crystal-config",
]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
ratatui = "0.29"
crossterm = "0.28"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

### 1.2 Create crystal-app Binary Crate

Dependencies:
```toml
[dependencies]
crystal-tui = { path = "../crystal-tui" }
crystal-config = { path = "../crystal-config" }
tokio.workspace = true
ratatui.workspace = true
crossterm.workspace = true
anyhow.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
```

### 1.3 Implement main.rs — Terminal Setup/Teardown

```rust
// Key structure — implement this pattern:
fn main() -> anyhow::Result<()> {
    // 1. Init tracing subscriber
    // 2. Enable raw mode
    // 3. Create CrosstermBackend
    // 4. Create Terminal
    // 5. Run app (async via tokio)
    // 6. Restore terminal on exit (including panic hook)
}
```

Critical: install a custom panic hook that restores the terminal BEFORE printing
the panic message. Otherwise panics leave the terminal in a broken state.

### 1.4 Implement app.rs — Main Event Loop

```rust
pub struct App {
    pub running: bool,
    pub tick_rate: Duration, // default 250ms
}

impl App {
    pub fn new() -> Self { /* ... */ }

    /// Main loop: poll events, dispatch, render
    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        while self.running {
            // 1. Draw UI
            terminal.draw(|frame| self.render(frame))?;
            // 2. Poll for events with timeout = tick_rate
            // 3. Handle key events
            // 4. Handle tick (periodic refresh)
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.running = false,
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame) {
        // Delegate to crystal-tui layout
    }
}
```

### 1.5 Implement event.rs — Async Event Stream

```rust
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
}

pub struct EventHandler {
    rx: tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        // Spawn a task that:
        // - polls crossterm::event::poll()
        // - sends Key/Resize events
        // - sends Tick on interval
    }

    pub async fn next(&mut self) -> anyhow::Result<AppEvent> {
        self.rx.recv().await.ok_or_else(|| anyhow::anyhow!("Event channel closed"))
    }
}
```

### 1.6 Implement crystal-tui Layout

```rust
// layout.rs
use ratatui::prelude::*;
use ratatui::widgets::*;

/// Root layout: 3 rows — header(1), body(fill), status_bar(1)
pub fn render_root(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // header
            Constraint::Min(0),    // body
            Constraint::Length(1),  // status bar
        ])
        .split(frame.area());

    // Header: app name + context info (placeholder)
    // Body: empty for now (will become pane layout in Stage 3)
    // Status bar: keybinding hints like zellij
}
```

### 1.7 Setup CI

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --all
      - run: cargo build --release
```

### 1.8 Configure Linting

```toml
# rustfmt.toml
max_width = 120
use_small_heuristics = "Max"
```

```toml
# clippy.toml
cognitive-complexity-threshold = 30
```

## Tests

- `cargo build` succeeds with no warnings
- `cargo clippy` passes with `-D warnings`
- `cargo fmt --check` passes
- App launches, renders status bar, quits on `q`
- Panic hook correctly restores terminal

## Demo Checklist

- [ ] Show workspace structure in editor
- [ ] `cargo build` from scratch
- [ ] Run app → see blank TUI with status bar
- [ ] Press `q` → clean exit
- [ ] Trigger a panic → terminal restores correctly
- [ ] Show CI passing on GitHub

## Commit Messages

```
feat(scaffold): initialize cargo workspace with 4 crates
feat(app): implement terminal setup with panic hook
feat(app): add async event loop with tick support
feat(tui): implement root layout with header and status bar
ci: add GitHub Actions workflow for lint, test, build
```
