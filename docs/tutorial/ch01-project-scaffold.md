# Chapter 1 — Project Scaffold & TUI Skeleton

This chapter sets up a Cargo workspace with proper crate boundaries, a CI
pipeline, and a minimal ratatui application that renders a blank screen with a
status bar and responds to `q` to quit.

## Prerequisites

- Rust toolchain installed (`rustup`, `cargo`)
- Git repository initialized

## What you will build

A terminal application with three visual zones — header, empty body, and a
status bar showing keybinding hints — driven by an async event loop. Pressing
`q` exits cleanly. A panic hook ensures the terminal is always restored, even
on crashes.

## File tree after this chapter

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
├── crates/
│   ├── crystal-app/               # Binary crate — TUI entry point
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs            # Entry point, terminal setup/teardown
│   │       ├── app.rs             # App state struct, event loop
│   │       └── event.rs           # Crossterm event polling (async)
│   ├── crystal-core/              # Library — business logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs             # Command enum stub
│   ├── crystal-tui/               # Library — UI components, layout
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── layout.rs          # Root layout (header, body, status bar)
│   │       └── theme.rs           # Color palette constants
│   └── crystal-config/            # Library — config parsing
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs             # Config struct with tests
```

---

## Step 1 — Initialize the Cargo workspace

Create the workspace root `Cargo.toml`. The `resolver = "2"` setting enables
the modern dependency resolver. All shared dependencies live under
`[workspace.dependencies]` so crate versions stay in sync.

```toml
# Cargo.toml (workspace root)
#
# A "workspace" lets us keep multiple related packages (called "crates")
# in one repository. Think of it like a monorepo — each crate compiles
# independently, but they share a single lock file and output directory.

[workspace]
# resolver = "2" turns on the modern dependency resolver.
# It handles platform-specific and feature-specific deps more correctly.
resolver = "2"

# List every crate that belongs to this workspace.
# Cargo will build/test/lint all of them together.
members = [
    "crates/crystal-app",       # the binary you run (the TUI application)
    "crates/crystal-core",      # business logic library (no UI, no I/O)
    "crates/crystal-tui",       # UI rendering library (layout, colors)
    "crates/crystal-config",    # configuration parsing library
]

# Shared dependency versions — every crate in the workspace can reference
# these with `dependency_name.workspace = true` instead of repeating the
# version number. This keeps all crates on the same version.
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }                          # async runtime (like Node's event loop)
ratatui = "0.29"                                                        # TUI rendering framework
crossterm = "0.28"                                                      # cross-platform terminal input/output
serde = { version = "1", features = ["derive"] }                        # serialization (for config files)
toml = "0.8"                                                            # TOML file parser
anyhow = "1"                                                            # ergonomic error handling
tracing = "0.1"                                                         # structured logging
tracing-subscriber = { version = "0.3", features = ["env-filter"] }     # log output + filtering by RUST_LOG env var
```

Create the four crate directories:

```bash
mkdir -p crates/{crystal-app/src,crystal-core/src,crystal-tui/src,crystal-config/src}
```

Each crate gets its own `Cargo.toml`. Dependencies reference the workspace
using `.workspace = true`.

**crates/crystal-app/Cargo.toml** — binary crate, depends on `crystal-tui`
and `crystal-config`:

```toml
[package]
name = "crystal-app"
version = "0.1.0"
edition = "2021"            # Rust edition — determines which language features are available

# [[bin]] means "this crate produces an executable binary".
# The compiled binary will be called "crystal" (what you type to run it).
[[bin]]
name = "crystal"            # the name of the executable: `cargo run` will run this
path = "src/main.rs"        # where the code lives

[dependencies]
# Local crates — referenced by file path (not from the internet).
# This is how crates within the same workspace depend on each other.
crystal-tui = { path = "../crystal-tui" }
crystal-config = { path = "../crystal-config" }

# External crates — `.workspace = true` means "use the version
# defined in the workspace root Cargo.toml" (keeps versions in sync).
tokio.workspace = true              # async runtime
ratatui.workspace = true            # TUI rendering
crossterm.workspace = true          # terminal I/O
anyhow.workspace = true             # error handling
tracing.workspace = true            # logging
tracing-subscriber.workspace = true # log output
```

**crates/crystal-core/Cargo.toml** — business logic library (stubbed):

```toml
[package]
name = "crystal-core"
version = "0.1.0"
edition = "2021"

# Note: no [[bin]] section — this is a *library* crate (code other crates import).
# Library crates use src/lib.rs as their entry point (not main.rs).
[dependencies]
anyhow.workspace = true     # error handling
```

**crates/crystal-tui/Cargo.toml** — UI rendering library:

```toml
[package]
name = "crystal-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui.workspace = true    # TUI rendering framework — provides widgets, layout, styling
crossterm.workspace = true  # terminal backend — handles raw mode, colors, input
```

**crates/crystal-config/Cargo.toml** — configuration parsing:

```toml
[package]
name = "crystal-config"
version = "0.1.0"
edition = "2021"

[dependencies]
serde.workspace = true      # deserialization framework — converts data formats into Rust structs
toml.workspace = true       # TOML parser — reads .toml config files
```

Add a `.gitignore` for Rust:

```gitignore
/target       # compiled output (like node_modules or __pycache__ — never commit this)
**/*.rs.bk    # backup files created by some editors
*.pdb         # Windows debug symbol files
```

Verify the workspace compiles (create empty `lib.rs` / placeholder `main.rs`
stubs first, then replace them in subsequent steps):

```bash
cargo build
```

**Commit:** `ch01: initialize cargo workspace with 4 crates`

---

## Step 2 — Implement crystal-config

A minimal `Config` struct that can be deserialized from TOML. For now it only
holds a `tick_rate_ms` setting with a sensible default.

**crates/crystal-config/src/lib.rs:**

```rust
// Bring in the Deserialize trait from serde.
// Traits in Rust are like interfaces — they define behavior a type can have.
// Deserialize means "this type can be created from data formats like TOML or JSON".
use serde::Deserialize;

// #[derive(...)] automatically generates code for common behaviors:
//   Debug   — allows printing the struct with {:?} (useful for logging)
//   Default — creates a Config with all fields set to their defaults (None here)
//   Deserialize — allows creating Config from a TOML/JSON string
//
// `pub` means this struct is visible to other crates that depend on crystal-config.
#[derive(Debug, Default, Deserialize)]
pub struct Config {
    // Option<u64> means this field is optional — it can be Some(value) or None.
    // u64 is an unsigned 64-bit integer (a positive whole number).
    // #[serde(default)] tells the parser: "if this field is missing, use None".
    #[serde(default)]
    pub tick_rate_ms: Option<u64>,
}

// `impl` block defines methods on the Config struct (like class methods).
impl Config {
    // Returns the tick rate, falling back to 250ms if not configured.
    // `&self` means this method borrows the struct (reads it without taking ownership).
    // `-> u64` is the return type.
    pub fn tick_rate_ms(&self) -> u64 {
        // unwrap_or: if tick_rate_ms is Some(value), return value; otherwise return 250
        self.tick_rate_ms.unwrap_or(250)
    }
}

// #[cfg(test)] means: only compile this module when running `cargo test`.
// This keeps test code out of the production binary.
#[cfg(test)]
mod tests {
    // `use super::*` imports everything from the parent module (Config, etc.)
    use super::*;

    // #[test] marks this function as a test case. `cargo test` will find and run it.
    #[test]
    fn default_config() {
        // Config::default() creates a Config with tick_rate_ms = None
        let config = Config::default();
        // assert_eq! checks that two values are equal; panics (= test fails) if not
        assert_eq!(config.tick_rate_ms(), 250); // should fall back to 250
    }

    #[test]
    fn parse_from_toml() {
        // A raw TOML string — like what you'd write in a config file
        let raw = "tick_rate_ms = 100";
        // toml::from_str parses the string into a Config struct.
        // .unwrap() crashes if parsing fails — acceptable in tests.
        let config: Config = toml::from_str(raw).unwrap();
        assert_eq!(config.tick_rate_ms(), 100); // should use the parsed value
    }
}
```

Run the tests:

```bash
cargo test -p crystal-config
```

**Commit:** `ch01: add crystal-config with tick rate setting`

---

## Step 3 — Implement crystal-core stub

A `Command` enum that will grow as features are added. For now it only has
`Quit` — aligning with the command-flow architecture where every action passes
through a typed command.

**crates/crystal-core/src/lib.rs:**

```rust
// An `enum` (enumeration) defines a type that can be one of several variants.
// Think of it like a tagged union or a TypeScript discriminated union.
//
// Every user action in Crystal will become a Command variant.
// This is the single entry point for all state changes — no module
// can skip this flow. For now we only have Quit; more will come later.
pub enum Command {
    Quit,
}
```

**Commit:** `ch01: add crystal-core with Command enum stub`

---

## Step 4 — Implement crystal-tui layout and theme

The UI crate owns all rendering. It exposes a `render_root` function that
splits the terminal into three rows and a color palette in `theme.rs`.

**crates/crystal-tui/src/lib.rs:**

```rust
// `pub mod` declares a public sub-module and tells Rust to look for its code
// in a file with the same name (layout.rs, theme.rs).
// Other crates can now use these as `crystal_tui::layout` and `crystal_tui::theme`.
pub mod layout;
pub mod theme;
```

**crates/crystal-tui/src/theme.rs** — Catppuccin-inspired palette:

```rust
use ratatui::style::Color;

// `const` defines a compile-time constant — the value is baked into the binary.
// Color::Rgb(r, g, b) creates a true-color value (like CSS #hex colors).
// These are inspired by the Catppuccin Mocha palette (a popular dark theme).
//
// Having all colors in one place makes it easy to change the look later
// without hunting through rendering code.

pub const HEADER_BG: Color = Color::Rgb(30, 30, 46);       // dark blue-gray background
pub const HEADER_FG: Color = Color::Rgb(205, 214, 244);    // light lavender text
pub const STATUS_BG: Color = Color::Rgb(49, 50, 68);       // slightly lighter gray
pub const STATUS_FG: Color = Color::Rgb(166, 173, 200);    // muted blue-gray text
pub const BODY_BG: Color = Color::Reset;                    // Reset = use terminal's default background
pub const ACCENT: Color = Color::Rgb(137, 180, 250);       // bright blue for highlights (future use)
```

**crates/crystal-tui/src/layout.rs** — three-row layout (header, body,
status bar):

```rust
// `prelude::*` imports commonly used types from ratatui (Layout, Frame, Style, etc.)
// so we don't have to list each one individually.
use ratatui::prelude::*;
// Block = a rectangular container, Paragraph = a text widget.
use ratatui::widgets::{Block, Paragraph};

// Import our color constants from the theme module.
use crate::theme;

// The main rendering function. Called every frame (many times per second).
//
// `frame: &mut Frame` — a mutable reference to the current frame.
//   A "frame" is like a canvas: you place widgets on it, and ratatui
//   figures out what changed since the last frame and redraws only that.
//
// This is a free function (not a method on a struct) because the UI crate
// is stateless — it just receives a frame and draws on it.
pub fn render_root(frame: &mut Frame) {
    // Layout splits the terminal area into chunks (regions).
    //   Direction::Vertical = stack rows top-to-bottom
    //   Constraint::Length(1) = exactly 1 row tall (for header and status bar)
    //   Constraint::Min(0)    = take all remaining space (the body)
    //
    // The result is a Vec of Rect (rectangles), one per constraint:
    //   chunks[0] = header area   (top row)
    //   chunks[1] = body area     (everything in between)
    //   chunks[2] = status bar    (bottom row)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area()); // frame.area() = the full terminal size

    // Render each section into its designated area
    render_header(frame, chunks[0]);
    render_body(frame, chunks[1]);
    render_status_bar(frame, chunks[2]);
}

// `fn` without `pub` means this function is private — only visible within this file.
// `area: Rect` is the rectangle (position + size) where this widget should be drawn.
fn render_header(frame: &mut Frame, area: Rect) {
    // Paragraph is a text widget. We set foreground (text) and background colors.
    // Style::default() starts with no styling, then we chain .fg() and .bg().
    let header =
        Paragraph::new(" crystal — kubernetes IDE").style(Style::default().fg(theme::HEADER_FG).bg(theme::HEADER_BG));
    // render_widget places the widget onto the frame at the given area.
    frame.render_widget(header, area);
}

fn render_body(frame: &mut Frame, area: Rect) {
    // An empty Block — just fills the area with the background color.
    // This will become the pane layout in later chapters.
    let body = Block::default().style(Style::default().bg(theme::BODY_BG));
    frame.render_widget(body, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect) {
    // Shows keybinding hints — similar to how zellij shows shortcuts at the bottom.
    let hints = Paragraph::new(" q: quit").style(Style::default().fg(theme::STATUS_FG).bg(theme::STATUS_BG));
    frame.render_widget(hints, area);
}
```

Key points:
- `render_root` is a free function, not a method — the UI crate has no state
- Layout uses `Constraint::Length(1)` for fixed header/footer and
  `Constraint::Min(0)` for the body that fills remaining space
- Colors are constants, making it easy to swap the palette later

**Commit:** `ch01: implement root layout with header and status bar`

---

## Step 5 — Implement crystal-app

This is the binary crate. It has three files:

### event.rs — Async event stream

The `EventHandler` spawns a tokio task that races two sources: a tick interval
and crossterm terminal events. Since `crossterm::event::poll` is blocking, it
runs inside `spawn_blocking` with a short 50ms timeout to avoid starving the
tick branch.

**crates/crystal-app/src/event.rs:**

```rust
use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};
// mpsc = "multiple producer, single consumer" channel.
// Think of it as a thread-safe queue: one side sends messages, the other receives them.
use tokio::sync::mpsc;

// Our own event type — we convert raw terminal events into these.
// This gives us a clean, simple type to work with in the rest of the app.
#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),          // a keyboard key was pressed/released
    Tick,                   // a periodic timer fired (for refreshing the UI)
    #[allow(dead_code)]     // suppress "unused" warning — we'll use Resize in a later chapter
    Resize(u16, u16),       // terminal window was resized (new width, new height)
}

// EventHandler owns the receiving end of a channel.
// The sending end runs in a background task (see `new()` below).
pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>, // "unbounded" = no limit on queued messages
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        // Create a channel pair: tx (transmitter/sender) and rx (receiver).
        // tx will be moved into a background task; rx stays here.
        let (tx, rx) = mpsc::unbounded_channel();

        // tokio::spawn launches a background async task (like a lightweight thread).
        // `async move` means this closure takes ownership of `tx` and `tick_rate`.
        tokio::spawn(async move {
            // Create a repeating timer that fires every `tick_rate` duration.
            let mut tick_interval = tokio::time::interval(tick_rate);

            // Infinite loop — runs until the receiver is dropped (app exits).
            loop {
                // tokio::select! races multiple async operations.
                // Whichever finishes first wins; the other is cancelled.
                // This is how we handle both "time passed" and "user pressed a key"
                // without blocking either one.
                let event = tokio::select! {
                    // Branch 1: the tick timer fired → produce a Tick event
                    _ = tick_interval.tick() => AppEvent::Tick,

                    // Branch 2: a terminal event arrived (key press, resize, etc.)
                    maybe = poll_crossterm_event() => match maybe {
                        Some(e) => e,       // got an event → use it
                        None => continue,   // no event (timeout or ignored) → try again
                    },
                };

                // Send the event through the channel to the main loop.
                // .is_err() means the receiver was dropped → app is shutting down → exit loop.
                if tx.send(event).is_err() {
                    break;
                }
            }
        });

        Self { rx }
    }

    // Wait for the next event from the background task.
    // `async` means this function can pause (await) without blocking the thread.
    pub async fn next(&mut self) -> anyhow::Result<AppEvent> {
        // .recv().await blocks until a message arrives.
        // .ok_or_else converts None (channel closed) into an error.
        self.rx.recv().await.ok_or_else(|| anyhow::anyhow!("Event channel closed"))
    }
}

// Helper function that polls the terminal for input events.
// Returns Some(AppEvent) if an event was read, or None if nothing happened.
async fn poll_crossterm_event() -> Option<AppEvent> {
    // crossterm's event::poll() is a BLOCKING call (it pauses the thread).
    // In async code, blocking is bad — it would freeze other tasks.
    // spawn_blocking moves it to a dedicated thread pool, keeping async tasks free.
    let event = tokio::task::spawn_blocking(|| {
        // Poll with a 50ms timeout: "is there an event within 50ms?"
        // .ok()? converts errors to None (the ? operator returns early on None).
        if event::poll(Duration::from_millis(50)).ok()? {
            event::read().ok() // yes → read the event
        } else {
            None // no event within 50ms → return None
        }
    })
    .await     // wait for the blocking task to complete
    .ok()??;   // two `?`: first unwraps the JoinHandle result, second unwraps the Option

    // Convert crossterm's Event type into our simpler AppEvent type.
    // `match` is like a switch statement — but it's exhaustive (must handle all cases).
    match event {
        Event::Key(key) => Some(AppEvent::Key(key)),        // keyboard event
        Event::Resize(w, h) => Some(AppEvent::Resize(w, h)), // terminal resized
        _ => None, // mouse events, focus events, etc. — ignore for now
    }
}
```

### app.rs — Main event loop

The `App` struct holds running state and tick rate. The `run` method is the
main loop: draw, wait for event, dispatch. Key handling filters for
`KeyEventKind::Press` to avoid duplicate events on some terminals.

**crates/crystal-app/src/app.rs:**

```rust
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::backend::Backend;  // Backend is a trait — an interface for terminal output
use ratatui::Terminal;

use crate::event::{AppEvent, EventHandler}; // our event types from event.rs

// The main application state.
// In Rust, structs are like classes but without inheritance.
// `pub` fields can be accessed from outside this module.
pub struct App {
    pub running: bool,          // when set to false, the main loop exits
    pub tick_rate: Duration,    // how often the UI refreshes (e.g., every 250ms)
}

impl App {
    // Constructor — creates a new App. Rust doesn't have a `new` keyword;
    // by convention, we write a function called `new` that returns Self.
    pub fn new(tick_rate_ms: u64) -> Self {
        Self { running: true, tick_rate: Duration::from_millis(tick_rate_ms) }
    }

    // The main event loop. This is where the app spends most of its time.
    //
    // `&mut self` means this method can modify the App's fields.
    // `impl Backend` means "any type that implements the Backend interface" —
    //   this makes the function work with different terminal backends.
    // `anyhow::Result<()>` means it returns either Ok (success) or an error.
    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        // Create the event handler — this starts the background event polling task.
        let mut events = EventHandler::new(self.tick_rate);

        // Main loop: runs until self.running is set to false.
        while self.running {
            // Step 1: Draw the UI.
            // terminal.draw() calls our render function, which paints widgets onto the frame.
            // ratatui uses double-buffering: it computes the diff and only redraws changed cells.
            // We pass `render_root` directly as a function pointer (no closure needed).
            terminal.draw(crystal_tui::layout::render_root)?;

            // Step 2: Wait for the next event (key press, tick, or resize).
            // .await pauses here until an event arrives (non-blocking for other async tasks).
            // The `?` at the end propagates errors — if something fails, exit the function.
            match events.next().await? {
                AppEvent::Key(key) => self.handle_key(key), // handle keyboard input
                AppEvent::Tick => {}                         // periodic refresh — nothing to do yet
                AppEvent::Resize(_, _) => {}                 // window resized — nothing to do yet
            }
        }

        Ok(()) // return success
    }

    // Handle a single key event.
    // `&mut self` because we might change `self.running`.
    fn handle_key(&mut self, key: KeyEvent) {
        // Some terminals send Press, Release, and Repeat events for the same key.
        // We only care about Press to avoid handling the same key multiple times.
        if key.kind != KeyEventKind::Press {
            return; // ignore Release and Repeat events
        }

        // `if let` is like a match with only one arm — checks if key.code is 'q'.
        // If the user pressed 'q', stop the app.
        if let KeyCode::Char('q') = key.code {
            self.running = false;
        }
    }
}
```

### main.rs — Terminal setup, teardown, and panic hook

The entry point follows a strict pattern:

1. Init tracing (writing to stderr so it doesn't corrupt the TUI)
2. Install a panic hook that restores the terminal **before** printing the
   panic — without this, panics leave the terminal in raw mode
3. Enable raw mode and enter the alternate screen
4. Run the app
5. Restore the terminal on exit

**crates/crystal-app/src/main.rs:**

```rust
// `mod` declares sub-modules that live in separate files (app.rs, event.rs).
// This is how Rust organizes code — the main file pulls in the other files.
mod app;
mod event;

use std::io;

// crossterm handles low-level terminal operations.
// `execute!` is a macro that sends commands to the terminal (like escape codes).
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
// CrosstermBackend connects ratatui's rendering to crossterm's terminal output.
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;

// #[tokio::main] transforms `async fn main` into a regular fn main that
// starts the tokio async runtime. Without this, you can't use `.await`.
// Think of it as: "set up the event loop, then run this async function".
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the logging system.
    // - EnvFilter: control log verbosity via the RUST_LOG environment variable
    //   (e.g., RUST_LOG=debug cargo run)
    // - with_writer(io::stderr): send logs to stderr, NOT stdout.
    //   This is critical because stdout is used for the TUI — mixing
    //   log text into it would corrupt the display.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(io::stderr)
        .init();

    // Install a safety net for crashes (see the function below).
    install_panic_hook();

    // --- Terminal setup ---
    // Raw mode: disables line buffering and special key handling by the OS.
    // Normally the terminal waits for Enter before sending input; raw mode
    // sends every keypress immediately (essential for a TUI).
    terminal::enable_raw_mode()?;

    // Alternate screen: switches to a separate screen buffer (like vim does).
    // When we exit, the user's previous terminal content is restored.
    execute!(io::stdout(), EnterAlternateScreen)?;

    // Create the ratatui Terminal, which manages double-buffered rendering.
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // --- Run the app ---
    // Load config (using defaults for now) and create the App.
    let config = crystal_config::Config::default();
    let mut app = App::new(config.tick_rate_ms());
    // .await runs the async event loop — this blocks until the app exits.
    // We save the result instead of using `?` so we can still clean up the terminal.
    let result = app.run(&mut terminal).await;

    // --- Terminal teardown ---
    // Always restore the terminal, even if the app returned an error.
    // disable_raw_mode: re-enables normal terminal behavior
    // LeaveAlternateScreen: switches back to the original screen buffer
    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    // Now propagate the app's result (either Ok or an error).
    result
}

// If the app panics (crashes), Rust normally prints an error message.
// But if the terminal is still in raw mode + alternate screen, that message
// would be invisible or garbled. This hook restores the terminal FIRST,
// then prints the panic message so you can actually read it.
fn install_panic_hook() {
    // Save the default panic handler so we can call it after cleanup.
    let original_hook = std::panic::take_hook();

    // Replace it with our custom handler.
    // Box::new(...) allocates the closure on the heap (required by set_hook).
    // `move` transfers ownership of `original_hook` into the closure.
    std::panic::set_hook(Box::new(move |panic_info| {
        // `let _ =` means "ignore the result" — if cleanup fails during a panic,
        // there's nothing useful we can do about it.
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        // Call the original handler to print the panic message + backtrace.
        original_hook(panic_info);
    }));
}
```

**Commit:** `ch01: implement terminal setup, event loop, and panic hook`

---

## Step 6 — Configure linting

**rustfmt.toml** — wider lines, maximized heuristics:

```toml
# rustfmt is Rust's official code formatter (like Prettier for JS).
# These settings are applied automatically when you run `cargo fmt`.

max_width = 120             # allow lines up to 120 chars (default is 100)
use_small_heuristics = "Max" # prefer putting things on one line when they fit
                              # (structs, function args, etc. won't be split unnecessarily)
```

**clippy.toml** — relaxed cognitive complexity for TUI rendering functions:

```toml
# clippy is Rust's linter (like ESLint for JS).
# This file adjusts clippy's default thresholds.

# Cognitive complexity measures how "hard to understand" a function is.
# TUI rendering code often has many branches (widgets, layouts, colors),
# which inflates the score. We raise the threshold from 25 to 30
# to avoid false warnings on rendering functions.
cognitive-complexity-threshold = 30
```

After creating these files, format all code and verify:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
```

**Commit:** `ch01: configure rustfmt and clippy`

---

## Step 7 — Setup CI

**.github/workflows/ci.yml:**

```yaml
name: CI

# When to run: on every push and on every pull request.
on: [push, pull_request]

env:
  CARGO_TERM_COLOR: always    # show colored output in CI logs

jobs:
  check:
    runs-on: ubuntu-latest    # use a Linux VM (free tier on GitHub)
    steps:
      # Step 1: Check out the repo code
      - uses: actions/checkout@v4

      # Step 2: Install the Rust toolchain with clippy (linter) and rustfmt (formatter)
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      # Step 3: Cache compiled dependencies between CI runs.
      # Without this, every CI run downloads and compiles all dependencies from scratch (~1-2 min).
      - uses: Swatinem/rust-cache@v2

      # Step 4: Check that all code is properly formatted.
      # --check means "don't change files, just report differences" (fails if code is unformatted).
      - run: cargo fmt --all -- --check

      # Step 5: Run the linter. -D warnings treats any warning as an error (strict mode).
      - run: cargo clippy --all-targets -- -D warnings

      # Step 6: Run all tests across all workspace crates.
      - run: cargo test --all

      # Step 7: Build in release mode (optimized). Catches issues that only appear with optimizations.
      - run: cargo build --release
```

The pipeline runs four checks in order: format, lint, test, build. The
`rust-cache` action caches `target/` and the cargo registry between runs.

**Commit:** `ch01: add GitHub Actions CI workflow`

---

## Verification

Run all four checks locally to confirm everything passes:

```bash
cargo fmt --all -- --check                  # check formatting (no changes, just reports)
cargo clippy --all-targets -- -D warnings   # lint all code (warnings = errors)
cargo test --all                            # run all tests (expect 2 passing in crystal-config)
cargo build --release                       # build optimized binary (slower to compile, faster to run)
```

Then launch the app:

```bash
cargo run
```

You should see:
- A header row: `crystal — kubernetes IDE`
- An empty body area
- A status bar: `q: quit`
- Pressing `q` exits cleanly back to your shell

## Architecture notes

The command flow established in this chapter:

```
Keyboard Input → AppEvent → App::handle_key → state mutation → UI re-render
```

Every module stays within its boundary:
- `crystal-tui` only renders — it receives a `Frame`, never mutates state
- `crystal-app` owns the event loop and delegates rendering to `crystal-tui`
- `crystal-config` is a standalone data crate with no runtime dependencies
- `crystal-core` defines the `Command` vocabulary (stubbed for now)

This separation will pay off as we add features in later chapters.
