# Step 3.8 — Wire Pane Layout to Render Loop

> `feat(app): wire pane layout system to render loop with command routing`

## Goal

Connect all the pieces: the pane tree, tab manager, border renderer, widgets,
keybinding dispatcher, and command routing into the app's main event loop.
After this step the full Stage 3 layout system is operational.

## Files

| File | Action |
|------|--------|
| `crates/crystal-app/src/app.rs` | UPDATE — pane-aware render + event routing |
| `crates/crystal-tui/src/view_registry.rs` | NEW — maps ViewType to render functions |

## Render Flow

```
1. Render tab bar (top row)
2. Get active tab's pane tree
3. Calculate pane layout rects via PaneNode::layout()
4. Render borders via render_pane_borders()
5. For each pane, look up view in registry, render within its rect
6. Render status bar (bottom row)
```

## Event Routing Flow

```
1. KeybindingDispatcher determines Command (from config, not hardcoded)
2. Global commands handled by App Core:
   - Quit, Help, Split, Close, Tab ops, Focus, Resize, Fullscreen, Mode switch
3. Command::Pane(cmd) routed to focused pane's view
4. Mode switches update the dispatcher
5. In Insert mode, all keys except global shortcuts forwarded as SendInput
```

## View Registry

```rust
// crates/crystal-tui/src/view_registry.rs

/// Maps ViewType to render functions.
/// Each view type has a corresponding render function that takes
/// the pane's Rect, focus state, and any view-specific state.
///
/// This indirection exists so that new view types (plugins, terminal)
/// can be added without modifying the render loop.
```

## Pane Type Catalog

These are the pane types the layout system must support. Not all are
implemented in Stage 3, but the tree and rendering contract must accommodate
them from day one.

| Pane Type | Stage | Description |
|-----------|-------|-------------|
| Empty     | 3     | Placeholder, shown on fresh splits |
| Help      | 3     | Contextual help — a pane, not a modal |
| ResourceList | 3  | Generic resource list (pods, services, etc.) |
| Detail    | 3     | Resource detail view |
| Terminal  | 4     | PTY-backed shell, context-injected via env vars |
| Logs      | 5     | Streaming pod logs with follow mode |
| Exec      | 5     | Interactive shell in a pod via exec |
| Plugin    | 6+    | Plugin-rendered pane via sandbox API |

### Forward References

**Terminal Pane (Stage 4):**
- All keys forwarded to PTY shell except global shortcuts
- Lifecycle: spawn on create, forward on focus, resize PTY on resize, terminate on close
- Context injected via env vars (KUBECONFIG, K8S_CONTEXT)

**Logs Pane (Stage 5):**
- Streams logs from a selected pod (identity passed by App Core)
- No cross-pane coupling — never queries Pods pane directly
- Stream starts on open, continues unfocused, stops on close
- Buffer capped to last N lines

**Exec Pane (Stage 5):**
- Interactive shell in a pod via PTY forwarding
- Context-aware: cluster + namespace + pod passed by App Core
- Same keyboard rules as Terminal pane (Insert mode)

**Plugin Panes (Stage 6+):**
- Integrate like normal panes, render via pane system
- Respect focus and global shortcuts
- Interact via defined API (Commands + PaneRender)
- Cannot mutate App Core directly

## Error Handling

Pane-level errors are displayed **inside the pane**, never as global modals:

| Scenario | Behavior |
|----------|----------|
| Pod deleted while logs streaming | Logs stop gracefully, message in pane |
| Permission denied on exec | Error message shown in pane |
| Network error | Retry with backoff, message in pane |
| Invalid kubeconfig | Warning in status bar, app still starts |
| Plugin panic | Pane shows error, app remains stable |

Never crash the app. Never leave orphaned streams or processes.

## Tests

- Full render cycle produces correct frame (tab bar, panes, borders, status bar)
- Keyboard event → dispatcher → command → correct handler → state update → re-render
- Pane command reaches only the focused pane
- Mode switch changes status bar hints
- View registry returns correct render fn for each ViewType
- Help pane content updates when focus changes

## Demo

This is the full Stage 3 integration demo:

- [ ] Launch with single pane showing pods
- [ ] Split vertically (Alt+v) → two panes side by side
- [ ] Split horizontally (Alt+h) → three panes
- [ ] Navigate between panes with Alt+arrows
- [ ] Resize panes with Alt+[ and Alt+]
- [ ] Close a pane (Alt+w) → sibling expands
- [ ] Create new tab (Alt+t), switch between tabs
- [ ] Status bar updates hints based on current mode
- [ ] Alt+f toggles fullscreen for a pane
- [ ] Help pane shows context-sensitive shortcuts
- [ ] Unfocused pane ignores pane-local input
