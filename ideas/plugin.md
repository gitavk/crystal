# Plugin System — Step-by-Step Implementation Guide

> Synthesized from `plugin_claude.md` (primary), `plugin_gemini.md`, and `plugin_codex.md`.
> Target: v0.2.0. Branch: `plugins`.

---

## Design Decisions

| Decision | Chosen | Rationale |
|----------|--------|-----------|
| Async bridge | `AppEvent` variants | Matches `QueryReady`, `LogsSnapshotReady` — plugins never block the event loop |
| Inbound protocol | `PaneCommand` (existing 22 variants) | Already the contract between App and all panes; WASM plugins just serialise it |
| ABI | Buffer-based JSON (ptr+len) | Stable across toolchain versions; WIT/component model deferred |
| Render payload | `Vec<String>` for v0.2.0 | Matches `LogsPane` cache pattern; Widget IR deferred to post-v0.2.0 |
| Capability enforcement | Scoped wasmtime `Linker` | Each plugin only gets linker entries for its declared permissions |
| Widget IR | Deferred | Gemini suggestion; good long-term but Vec<String> is sufficient now |
| Hot reloading | Deferred (Phase 5) | Needs `notify` crate watcher; out of scope for v0.2.0 |

---

## Phase 0 — PaneRegistry (pure Rust refactor)

**Goal:** Replace scattered `Box::new(SomethingPane::new(...))` calls with a central registry lookup. No WASM yet — just plumbing. Ships independently with no user-visible change.

### Files to create

**`kubetile-app/src/plugin_registry.rs`**

```rust
pub struct PaneContext<'a> {
    pub pane_id:     PaneId,
    pub kube_client: Option<&'a KubeClient>,
    pub event_tx:    mpsc::UnboundedSender<AppEvent>,
    pub theme:       &'a Theme,
    pub config:      &'a AppConfig,
}

pub type PaneFactory = Arc<dyn Fn(&PaneContext<'_>) -> Box<dyn Pane> + Send + Sync>;

pub struct PaneRegistry {
    factories: HashMap<String, PaneFactory>,
}

impl PaneRegistry {
    pub fn new() -> Self { ... }
    pub fn register(&mut self, key: impl Into<String>, factory: PaneFactory);
    pub fn create(&self, key: &str, ctx: &PaneContext<'_>) -> Option<Box<dyn Pane>>;
}
```

### Tasks

1. Add `plugin_registry.rs` to `kubetile-app/src/` and declare `mod plugin_registry` in `lib.rs`.
2. Add `fn register_builtins(registry: &mut PaneRegistry)` that registers:
   - `"app-logs"` → `AppLogsPane::new`
   - `"port-forwards"` → `PortForwardsPane::new`
   - `"resource-list"` → `ResourceListPane::new`
   - `"query"` → `QueryPane::new`
3. Add `registry: PaneRegistry` field to `App`; initialise in `App::new()` via `register_builtins`.
4. In `app/tabs.rs`, replace every `Box::new(AppLogsPane::new(...))` / `Box::new(PortForwardsPane::new(...))` with `registry.create("app-logs", &ctx).unwrap_or_else(|| placeholder_pane())`.
5. Repeat step 4 for every pane creation site in `app/pane_ops.rs` and `app/query.rs`.
6. Verify: `cargo test` green, existing UX unchanged.

---

## Phase 1 — PluginPane wrapper (host-side, pure Rust)

**Goal:** Introduce a generic `PluginPane` that implements `Pane` and delegates to a `PluginSession` trait. At this phase the session is a local Rust stub — easy to unit-test. WASM replaces it in Phase 2.

### Files to create

**`kubetile-app/src/panes/plugin_pane.rs`**

```rust
pub enum PluginStatus { Loading, Ready, Error(String) }

pub trait PluginSession: Send + 'static {
    fn handle_command(&mut self, cmd: PluginCommand);
    fn on_focus(&mut self, focused: bool);
    fn on_resize(&mut self, width: u16, height: u16);
}

pub struct PluginPane {
    view_type:    ViewType,
    render_lines: Vec<String>,   // cached last payload from plugin
    status:       PluginStatus,
    session:      Box<dyn PluginSession>,
}

impl Pane for PluginPane {
    fn render(&self, frame, area, focused, theme) {
        // Loading  → spinner paragraph
        // Error    → error text with theme.error colour
        // Ready    → render self.render_lines as Paragraph lines
    }
    fn handle_command(&mut self, cmd: &PaneCommand) {
        if let Some(pc) = PluginCommand::from_pane_command(cmd) {
            self.session.handle_command(pc);
        }
    }
    fn on_focus_change(&mut self, _prev: Option<&ViewType>) {
        self.session.on_focus(true);
    }
    // view_type, as_any, as_any_mut ...
}
```

**`kubetile-app/src/plugin_command.rs`**

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginCommand {
    ScrollUp, ScrollDown, SelectNext, SelectPrev,
    Select, Back, GoToTop, GoToBottom,
    PageUp, PageDown, ToggleFollow, ToggleWrap,
    ScrollLeft, ScrollRight,
    SendInput { text: String },
    Filter { query: String }, ClearFilter,
    // Lifecycle
    Focus { focused: bool },
    Resize { width: u16, height: u16 },
}

impl PluginCommand {
    pub fn from_pane_command(cmd: &PaneCommand) -> Option<Self> { ... }
}
```

### AppEvent additions (`kubetile-app/src/event.rs`)

```rust
PluginRender { pane_id: PaneId, lines: Vec<String> },
PluginError  { pane_id: PaneId, error: String },
```

### Tasks

1. Create `plugin_command.rs`; add `mod plugin_command` to `lib.rs`; implement `from_pane_command` mapping for all `PaneCommand` variants.
2. Create `panes/plugin_pane.rs`; add `mod plugin_pane; pub use plugin_pane::PluginPane` to `panes/mod.rs`.
3. Add `PluginRender` / `PluginError` variants to `event.rs`.
4. Route them in `app/input.rs::handle_event()` — find pane, downcast to `PluginPane`, call `set_render_lines()` / `set_error()`.
5. Register a stub factory in `register_builtins` for `"smoke-test"` that returns a `PluginPane` backed by a `StubPluginSession` (returns `vec!["smoke-ok"]`).
6. Write unit tests in `plugin_pane.rs`:
   - `handle_command` translates `PaneCommand::ScrollDown` → `PluginCommand::ScrollDown`.
   - Error status renders without panic.
7. Verify: `cargo test` green.

---

## Phase 2 — WASM host (wasmtime)

**Goal:** Replace `PluginSession` trait object with a real WASM instance running inside `wasmtime`. Plugin authors can now ship `.wasm` files.

### ABI (buffer-based, v0.2.0)

Plugin module **exports**:
```
plugin_init(config_ptr: i32, config_len: i32) -> i32
plugin_on_command(cmd_ptr: i32, cmd_len: i32) -> i32
```

Host **exports** to plugin (always linked):
```
host_send_render(lines_ptr: i32, lines_len: i32)
```

All payloads are UTF-8 JSON written into plugin linear memory. Host uses `wasmtime::Memory::read()` / `Memory::write()`.

### Files to create

**`kubetile-app/src/plugin_wasm.rs`**

```rust
pub struct WasmEngine {
    engine:  Engine,
    modules: HashMap<String, Module>,  // pre-compiled, keyed by plugin path
}

pub struct WasmPluginSession {
    cmd_tx:   mpsc::UnboundedSender<PluginCommand>,
    // tokio task owns Store + Instance; communicates back via event_tx: AppEvent
}

impl PluginSession for WasmPluginSession {
    fn handle_command(&mut self, cmd: PluginCommand) {
        let _ = self.cmd_tx.send(cmd);
    }
    fn on_focus(&mut self, focused: bool) { ... }
    fn on_resize(&mut self, width: u16, height: u16) { ... }
}

fn build_linker(engine: &Engine, caps: &[Capability]) -> Linker<PluginHostState> {
    let mut linker = Linker::new(engine);
    // always available
    linker.func_wrap("kubetile", "host_send_render", host_send_render);
    if caps.contains(&Capability::K8sListPods) {
        linker.func_wrap("kubetile", "list_pods", host_list_pods);
    }
    // ... other capability gates
    linker
}
```

### Resource limits

```rust
let limits = StoreLimitsBuilder::new()
    .memory_size(32 * 1024 * 1024)  // 32 MB per plugin
    .build();
```

### Capability enum

```rust
pub enum Capability {
    K8sListPods,
    K8sGetSecrets,
    ClipboardWrite,
    // extend as needed
}

pub fn parse_capabilities(perms: &[String]) -> Vec<Capability> { ... }
```

### Tasks

1. Add `wasmtime = { version = "...", features = ["cranelift"] }` to `kubetile-app/Cargo.toml`.
2. Create `plugin_wasm.rs`; add `mod plugin_wasm` to `lib.rs`.
3. Implement `WasmEngine::precompile(path) -> Result<Module>` (AOT compilation at startup).
4. Implement `WasmPluginSession::spawn(module, linker, pane_id, event_tx)` — starts a tokio task that owns the store and runs the plugin event loop.
5. Implement `host_send_render`: reads lines JSON from plugin memory, sends `AppEvent::PluginRender { pane_id, lines }`.
6. Implement `build_linker` with scoped capability gates.
7. Add `register_wasm(key, path, caps)` to `PaneRegistry` — pre-compiles the module and stores a `PaneFactory` that calls `WasmPluginSession::spawn`.
8. Replace the `"smoke-test"` stub factory with a real WASM-backed one pointing at `crates/kubetile-plugin-smoke/target/wasm32-unknown-unknown/release/plugin.wasm`.
9. Write unit tests in `plugin_wasm.rs`:
   - `plugin_on_command` receives correct JSON payload.
   - `host_send_render` pushes `AppEvent::PluginRender`.
   - Unlinked host function call results in `AppEvent::PluginError` (not panic).
10. Verify: `cargo test` green.

---

## Phase 3 — Configuration & lifecycle

**Goal:** Plugins are driven entirely by TOML config; no code changes required to add a new plugin.

### Config schema

```toml
# ~/.config/kubetile/config.toml

[[plugins]]
key         = "my-view"
path        = "~/.local/share/kubetile/plugins/my_view.wasm"
permissions = ["k8s:list:pods", "clipboard:write"]

[[plugins]]
key         = "port-dash"
path        = "~/.local/share/kubetile/plugins/port_dash.wasm"
permissions = []
keybinding  = "ctrl+p"
```

### Config struct additions (`kubetile-config/src/`)

```rust
// plugin_config.rs (new) or extend lib.rs
#[derive(Deserialize, Default)]
pub struct PluginEntry {
    pub key:         String,
    pub path:        String,
    #[serde(default)]
    pub permissions: Vec<String>,
    pub keybinding:  Option<String>,
}

// AppConfig gains:
#[serde(default)]
pub plugins: Vec<PluginEntry>,
```

### Lifecycle hook exports (all optional except `plugin_on_command`)

| Export | When called |
|--------|-------------|
| `plugin_init(cfg_ptr, cfg_len) -> i32` | Module loaded (once per key) |
| `plugin_on_open(pane_id: i32)` | Pane instance created |
| `plugin_on_close(pane_id: i32)` | Pane/tab closed |
| `plugin_on_command(cmd_ptr, cmd_len) -> i32` | PaneCommand forwarded from host |
| `plugin_on_tick()` | Every app tick, if exported |

### Tasks

1. Create `kubetile-config/src/plugin_config.rs`; add `PluginEntry` struct; add `pub plugins: Vec<PluginEntry>` to `AppConfig`.
2. In `main.rs` (after config parse, before `App::new`):
   ```rust
   for entry in &config.plugins {
       let caps = parse_capabilities(&entry.permissions);
       registry.register_wasm(&entry.key, &entry.path, caps)?;
   }
   ```
3. Call `plugin_on_open` in `WasmPluginSession::spawn` after init.
4. In `app/pane_ops.rs::close_focused()`, after removing a pane, downcast to `PluginPane` and call `session.on_close(pane_id)`.
5. If `entry.keybinding` is set, register a toggle command in `keybindings.rs` (similar to `open_query` in `defaults.toml`).
6. Verify: plugin loads from TOML without code change; closing a tab calls `plugin_on_close`.

---

## Phase 4 — SDK & testing

**Goal:** Plugin authors can write idiomatic Rust without knowing the ABI. CI validates the full stack with a smoke plugin.

### New crates

**`crates/kubetile-plugin-sdk/`** — `no_std`-compatible; only `serde_json` as runtime dep.

```
src/
  lib.rs        # re-exports macros + types
  host_api.rs   # extern "C" host function declarations
  lifecycle.rs  # #[plugin_on_command] proc-macro helper
  render.rs     # RenderPayload builder
```

Usage in a plugin:

```rust
#[plugin_on_command]
fn handle(cmd: PluginCommand) -> Option<RenderPayload> {
    match cmd {
        PluginCommand::ScrollDown => { /* update state */ }
        _ => {}
    }
    Some(RenderPayload::lines(vec!["Hello from WASM".into()]))
}
```

The macro generates the `plugin_on_command(ptr, len)` extern fn, deserialises the command, calls the user fn, serialises the payload, and calls `host_send_render`.

**`crates/kubetile-plugin-smoke/`** — test-only `cdylib`; returns `"smoke-ok"` regardless of command. Built in CI via `cargo build --target wasm32-unknown-unknown`.

### Test matrix

| Test type | What | Location |
|-----------|------|----------|
| Unit — registry | `register` + `create` returns correct `Box<dyn Pane>` | `plugin_registry.rs` inline tests |
| Unit — PluginPane | `handle_command` translates correctly; error renders without panic | `plugin_pane.rs` inline tests |
| Unit — WasmSession | JSON payload delivered; `host_send_render` pushes `AppEvent::PluginRender` | `plugin_wasm.rs` tests |
| Integration — App | `ViewType::Plugin("smoke-test")` tab renders `"smoke-ok"` in headless terminal | `app/tests.rs` |
| Integration — error | Unknown key → error pane, no panic | `app/tests.rs` |
| Stress | 10 concurrent smoke WASM instances all receive commands | `plugin_wasm.rs` |

### Tasks

1. Create `crates/kubetile-plugin-sdk/` with `Cargo.toml` (`cdylib + rlib`, `no_std` compatible).
2. Implement `host_api.rs`: `extern "C" { fn host_send_render(ptr: i32, len: i32); }`.
3. Implement `render.rs`: `RenderPayload { lines: Vec<String> }` + `fn lines(v: Vec<String>) -> Self`.
4. Implement the `#[plugin_on_command]` proc-macro (or a simpler `fn register_handler` approach if proc-macros are too heavy for v0.2.0).
5. Create `crates/kubetile-plugin-smoke/`; implement the smoke handler.
6. Add CI step: `cargo build -p kubetile-plugin-smoke --target wasm32-unknown-unknown`.
7. Wire smoke WASM path into `app/tests.rs` integration test config.
8. Write all tests in the matrix above.
9. Verify: `cargo test` green; CI passes.

---

## Rollout

| Phase | Milestone | Shippable |
|-------|-----------|-----------|
| 0 — PaneRegistry refactor | All panes created via registry | Yes — no user-visible change |
| 1 — PluginPane wrapper | Local Rust sessions; stub smoke plugin works | Yes — v0.2.0-alpha |
| 2 — WASM host | `.wasm` files loadable; smoke plugin renders in real run | Yes — **v0.2.0** |
| 3 — Config & lifecycle | TOML-driven discovery; keybinding shortcuts; lifecycle hooks | Yes — v0.2.0 polish |
| 4 — SDK & CI | Plugin SDK crate; CI green with integration test | Yes — v0.2.0 release |

---

## File tree (new / modified)

```
kubetile-app/src/
  plugin_registry.rs       NEW — PaneRegistry, PaneContext, PaneFactory
  plugin_command.rs        NEW — PluginCommand enum + PaneCommand → PluginCommand mapping
  plugin_wasm.rs           NEW — WasmPluginSession, WasmEngine, build_linker, Capability
  panes/
    plugin_pane.rs         NEW — PluginPane, PluginSession trait, PluginStatus
  app.rs                   MOD — add registry field; remove hardcoded Plugin matches
  app/tabs.rs              MOD — call registry.create() instead of Box::new(...)
  app/pane_ops.rs          MOD — call registry.create(); call plugin_on_close on remove
  app/input.rs             MOD — route PluginRender / PluginError events
  event.rs                 MOD — add PluginRender, PluginError variants
kubetile-config/src/
  plugin_config.rs         NEW — PluginEntry struct
  lib.rs                   MOD — add plugins: Vec<PluginEntry> to AppConfig
crates/
  kubetile-plugin-sdk/     NEW crate — host_api.rs, render.rs, lifecycle.rs
  kubetile-plugin-smoke/   NEW crate (test only) — returns "smoke-ok"
```

---

## Key rules / gotchas

- **Never block in `PluginSession::handle_command`** — forward to a channel; the tokio task calls the WASM export.
- **`AppEvent` is the only way plugin output reaches the UI** — same pattern as `QueryReady`.
- **All plugin keys must be kebab-case** — enforced by convention; registry keys, TOML keys, and `ViewType::Plugin` strings must match exactly.
- **`PaneCommand` → `PluginCommand` mapping is lossy by design** — commands that don't make sense for plugins (e.g., internal app state commands) simply return `None` and are silently dropped.
- **Clippy deny rules still apply** — no `never_loop`, use `.clamp()`, no `for { break }`.
- **`RenderContext` changes** — if a pane type adds a field to `RenderContext`, also update `layout/tests.rs` (exhaustive struct literal).
