# Stage 3 — Zellij-Style Pane Layout System

## Goal

Implement a multi-pane layout system inspired by zellij. Users can split the
screen horizontally/vertically, resize panes, navigate between panes, and close
them. Each pane hosts a "view" (resource list, detail, terminal, etc.).

The pane system is the foundational abstraction that every later feature builds
on — pods, logs, exec, plugins, and AI all render inside panes.

## Prerequisites

- Stage 2 complete (pod list renders with live data)

## YouTube Episodes

1. **"Building a Zellij-Style Layout Engine in Rust"**: pane tree, splitting
2. **"Pane Navigation & Resize — Keyboard-First UX"**: focus, keybindings
3. **"Tab System & View Registry"**: tabs, view lifecycle

---

## Design Principles

These rules are non-negotiable and enable plugins, testability, and clean UX.

### Pane Isolation
- Panes render in isolation — no pane knows about other panes
- Root UI composes pane output into the final frame
- Panes never access global input directly
- Pane-local shortcuts are inactive when the pane is unfocused

### Command = User Intent
- Commands are UI-agnostic and keyboard-agnostic
- The input layer knows about keys, NOT about app state
- The input layer emits Commands only — keyboard code never touches AppState
- UI code never handles key events directly

### Data Flow Through App Core
- Panes never talk to K8s or each other directly
- All cross-pane data flows through App Core (e.g. selected pod → logs pane)
- App Core passes context explicitly; no implicit coupling

### Config Over Magic
- No hardcoded shortcuts in code — all keybindings are config-driven
- Defaults shipped embedded in the binary
- User config (`~/.config/crystal/config.toml`) overrides defaults
- Help screen must always reflect active (not default) keybindings

---

## Steps

| Step | File | Commit | Summary |
|------|------|--------|---------|
| 3.1 | [03a-pane-tree.md](03a-pane-tree.md) | `feat(tui): implement pane tree data structure with split/close/resize` | PaneNode, ViewType, SplitDirection, split/close/resize/layout ops |
| 3.2 | [03b-pane-contract-commands.md](03b-pane-contract-commands.md) | `feat(tui): define pane rendering contract and command scoping` | Pane trait, Command/PaneCommand enums, routing rules |
| 3.3 | [03c-borders.md](03c-borders.md) | `feat(tui): add zellij-style border rendering between panes` | Box-drawing borders, focus highlight, dimmed unfocused |
| 3.4 | [03d-tab-manager.md](03d-tab-manager.md) | `feat(tui): implement tab manager with create/close/switch` | Tab, TabManager, pane ID allocation |
| 3.5 | [03e-tab-bar-status-bar.md](03e-tab-bar-status-bar.md) | `feat(tui): add tab bar widget and status bar with mode hints` | TabBarWidget, StatusBar, mode-dependent hints |
| 3.6 | [03f-keybinding-dispatcher.md](03f-keybinding-dispatcher.md) | `feat(app): implement config-driven keybinding dispatcher` | InputMode, KeybindingDispatcher, TOML config, loading order |
| 3.7 | [03g-directional-navigation.md](03g-directional-navigation.md) | `feat(app): add directional pane focus navigation` | find_pane_in_direction algorithm, focus cycling |
| 3.8 | [03h-wire-layout-loop.md](03h-wire-layout-loop.md) | `feat(app): wire pane layout system to render loop with command routing` | Render flow, event routing, pane type catalog, error handling |

## All Files Touched

```
crates/
├── crystal-tui/
│   └── src/
│       ├── layout.rs              # REWRITE — pane tree system
│       ├── pane.rs                # NEW — pane container + trait
│       ├── tab.rs                 # NEW — tab management
│       ├── view_registry.rs       # NEW — maps view types to render fns
│       ├── borders.rs             # NEW — pane border rendering (zellij-style)
│       └── widgets/
│           ├── status_bar.rs      # NEW — zellij-style mode/keybinding hints
│           └── tab_bar.rs         # NEW — tab strip at top
├── crystal-app/
│   └── src/
│       ├── app.rs                 # Update: pane-aware event routing
│       └── keybindings.rs         # NEW — mode-based key dispatch
```

## Full Demo Checklist

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
