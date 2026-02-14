# Step 5.7 — Log Streaming & Logs View

> `feat(core): implement streaming log reader with tail/follow/reconnect`

## Goal

Implement `LogStream` in `crystal-core` for streaming pod logs via the Kubernetes
log API, and `LogsView` in `crystal-tui` for rendering them with filtering,
scrolling, and auto-follow. Log streams reconnect automatically on interruption.

## Files

| File | Action |
|------|--------|
| `crates/crystal-core/src/logs.rs` | NEW — LogStream, LogLine, LogRequest |
| `crates/crystal-tui/src/views/logs_view.rs` | NEW — LogsView (pure renderer) |

## LogStream

```rust
// crates/crystal-core/src/logs.rs

pub struct LogStream {
    lines: Vec<LogLine>,
    rx: mpsc::UnboundedReceiver<LogLine>,
}

pub struct LogLine {
    pub timestamp: Option<jiff::Timestamp>,
    pub content: String,
    pub container: String,
    pub is_stderr: bool,
}

pub struct LogRequest {
    pub pod_name: String,
    pub namespace: String,
    pub container: Option<String>,   // None = all containers
    pub follow: bool,
    pub tail_lines: Option<i64>,     // default: 1000
    pub since_seconds: Option<i64>,
    pub previous: bool,
    pub timestamps: bool,
}

impl LogStream {
    pub async fn start(
        client: &KubeClient,
        request: LogRequest,
    ) -> anyhow::Result<Self> { /* ... */ }

    pub async fn next_lines(&mut self) -> Option<Vec<LogLine>> { /* ... */ }
}
```

## Log Streaming Flow

```
User selects pod → presses `l`
   ↓
Command::LogsStart { request }
   ↓
App Core: LogStream::start(client, request) → spawns background task
   ↓
Background task: reads log API, sends LogLines via mpsc channel
   ↓
Each tick: App Core drains channel, appends to log buffer
   ↓
RenderContext provides &[LogLine] to LogsView
```

## Reconnection Strategy

When the log stream is interrupted (pod restart, API timeout):
1. Detect error from the K8s log API response stream
2. Wait with exponential backoff: 1s, 2s, 4s, 8s, max 30s
3. Re-issue the log request with `since_seconds` set to avoid duplicates
4. Emit `Event::LogsReconnecting { stream_id }` → view shows status
5. On success: emit `Event::LogsReconnected { stream_id }`
6. After 5 consecutive failures: stop retrying, show error in view

## LogsView

```rust
// crates/crystal-tui/src/views/logs_view.rs

/// Pure rendering view — receives log lines from App Core, renders them.
pub struct LogsView {
    scroll_offset: usize,
    auto_scroll: bool,       // follows tail when true
    filter: Option<String>,  // grep-like filter
    show_timestamps: bool,
    wrap_lines: bool,
    container_filter: Option<String>,
}
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll down / up |
| `g` / `G` | Jump to top / bottom |
| `f` | Toggle follow (auto-scroll) |
| `/` | Open filter input |
| `t` | Toggle timestamp display |
| `w` | Toggle line wrap |
| `c` | Switch container (multi-container pods) |
| `p` | Toggle previous container logs |
| `s` | Save visible logs to file |
| `Esc` | Close logs view |

## Filter Behavior

- `/` enters filter input mode (similar to vim search)
- Filter is a case-insensitive substring match by default
- Lines not matching the filter are hidden (not deleted)
- Filter text shown in status bar: `Filter: "error"  (42/1000 lines)`
- Empty filter shows all lines

## Rendering

- Timestamp column (if enabled): dimmed, fixed width
- Container column (if multi-container): colored per container
- Content: default foreground, highlight filter matches
- Stderr lines: rendered in warning color (from theme)
- Status bar at bottom: follow indicator, filter, line count, reconnection status

## Tests

- `LogStream::start()` receives lines from a running pod (integration)
- `next_lines()` returns `None` when stream ends
- Filter hides non-matching lines
- `auto_scroll` follows new lines when at bottom
- `auto_scroll` pauses when user scrolls up
- Scrolling up/down respects line count bounds
- Timestamp toggle shows/hides timestamp column
- Container filter shows only selected container's lines

## Demo

- [ ] Select a pod, press `l` → log stream opens
- [ ] Logs auto-scroll as new lines arrive
- [ ] Press `f` → auto-scroll pauses, manual scroll works
- [ ] Press `/`, type "error" → only matching lines shown
- [ ] Press `t` → timestamps toggle
- [ ] Press `c` on multi-container pod → container picker
- [ ] Press `s` → logs saved to file, toast confirms
- [ ] Kill the pod → "Reconnecting..." appears, then new logs after restart
