# Step 4.9 — Overlays (Confirm Dialog, Toast, Context-Sensitive Help)

> `feat(tui,app): add confirm dialog, toast, and context-sensitive help`

## Goal

Implement three overlay/feedback components:

1. **ConfirmDialog** — modal confirmation for destructive actions
2. **Toast** — auto-dismissing notification messages
3. **Context-sensitive Help** — extend existing HelpPane to show actions
   relevant to the current resource kind

## Files

| File | Action |
|------|--------|
| `crates/crystal-tui/src/widgets/confirm_dialog.rs` | NEW — confirmation popup widget |
| `crates/crystal-tui/src/widgets/toast.rs` | NEW — toast notification widget |
| `crates/crystal-tui/src/widgets/mod.rs` | UPDATE — add modules |
| `crates/crystal-app/src/panes/help.rs` | UPDATE — context-sensitive keybinding sections |
| `crates/crystal-app/src/app.rs` | UPDATE — dialog state, toast queue, help context |

## Confirmation Dialog

### Widget

```rust
// crates/crystal-tui/src/widgets/confirm_dialog.rs

pub struct ConfirmDialogWidget<'a> {
    pub message: &'a str,
    pub confirm_label: &'a str,  // "y"
    pub cancel_label: &'a str,   // "n / Esc"
}
```

### Rendering Layout

```
┌─ Confirm ─────────────────────────┐
│                                    │
│  Delete pod nginx-abc123           │
│  in namespace default?             │
│                                    │
│         [y] Confirm  [n] Cancel    │
│                                    │
└────────────────────────────────────┘
```

- Centered on screen, width = max(message_width + 4, 40)
- Border in `theme::STATUS_FAILED` (red) for destructive actions
- Confirm button highlighted when focused
- Background area cleared (overlay)

### Rendering Implementation

```rust
impl<'a> Widget for ConfirmDialogWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = (self.message.len() + 4).max(40).min(area.width as usize) as u16;
        let height = 7;
        let popup = centered_rect(width, height, area);

        Clear.render(popup, buf);

        let block = Block::default()
            .title(" Confirm ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::STATUS_FAILED));
        let inner = block.inner(popup);
        block.render(popup, buf);

        // Render message text centered
        // Render button labels at bottom
    }
}
```

### App State

```rust
// crates/crystal-app/src/app.rs

pub struct PendingConfirmation {
    pub message: String,
    pub action: PendingAction,
}

pub enum PendingAction {
    Delete {
        kind: ResourceKind,
        name: String,
        namespace: String,
    },
}

pub struct App {
    // ... existing fields ...
    pending_confirmation: Option<PendingConfirmation>,
}
```

### Command Flow

```
Command::DeleteResource
  → App looks up selected resource name/ns from focused pane
  → App sets pending_confirmation = Some(PendingConfirmation { ... })
  → App sets InputMode::ConfirmDialog
  → UI renders ConfirmDialogWidget overlay

Command::ConfirmAction ('y')
  → App takes pending_confirmation
  → Spawns async delete via ActionExecutor
  → On success: push Toast::success(...)
  → On error: push Toast::error(...)
  → Clears pending_confirmation, restores InputMode::Normal

Command::DenyAction ('n' / Esc)
  → Clears pending_confirmation
  → Restores InputMode::Normal
```

## Toast Notifications

### Data Types

```rust
// crates/crystal-tui/src/widgets/toast.rs

#[derive(Clone, Debug)]
pub enum ToastLevel {
    Success,
    Error,
    Info,
}

#[derive(Clone, Debug)]
pub struct ToastMessage {
    pub text: String,
    pub level: ToastLevel,
    pub created_at: std::time::Instant,
    pub ttl: std::time::Duration,
}

impl ToastMessage {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: ToastLevel::Success,
            created_at: std::time::Instant::now(),
            ttl: std::time::Duration::from_secs(3),
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            level: ToastLevel::Error,
            created_at: std::time::Instant::now(),
            ttl: std::time::Duration::from_secs(5), // errors stay longer
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.ttl
    }
}
```

### Widget

```rust
pub struct ToastWidget<'a> {
    pub toasts: &'a [ToastMessage],
}
```

### Rendering Layout

```
                                    ┌──────────────────────┐
                                    │ ✓ Deleted nginx-abc  │  ← success (green)
                                    └──────────────────────┘
                                    ┌──────────────────────┐
                                    │ ✗ Permission denied  │  ← error (red)
                                    └──────────────────────┘
```

- Positioned in bottom-right corner of the screen
- Multiple toasts stack upward
- Color by level:
  - Success: `theme::STATUS_RUNNING` (green) border
  - Error: `theme::STATUS_FAILED` (red) border
  - Info: `theme::ACCENT` border
- Maximum 3 visible toasts at once

### App Integration

```rust
pub struct App {
    // ... existing fields ...
    toasts: Vec<ToastMessage>,
}

impl App {
    fn push_toast(&mut self, toast: ToastMessage) {
        self.toasts.push(toast);
    }

    /// Called every tick to remove expired toasts.
    fn cleanup_toasts(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }
}
```

Toast cleanup happens in the tick handler, which already runs at the
configured tick rate. No additional timer needed.

### RenderContext Extension

```rust
pub struct RenderContext<'a> {
    // ... existing fields ...
    pub confirm_dialog: Option<ConfirmDialogView<'a>>,
    pub toasts: &'a [ToastMessage],
}

pub struct ConfirmDialogView<'a> {
    pub message: &'a str,
}
```

Both overlays are rendered after all panes and the status bar,
in the same pattern as the namespace selector overlay.

## Context-Sensitive Help

### Extended HelpPane

The existing `HelpPane` shows a static list of shortcuts. Extend it to
include resource-specific actions based on the current view:

```rust
// crates/crystal-app/src/panes/help.rs

impl HelpPane {
    /// Build help content for the current context.
    pub fn for_context(
        mode: &InputMode,
        view: Option<&ViewType>,
        bindings: &KeybindingDispatcher,
    ) -> Self {
        let mut sections = Vec::new();

        // 1. Always show global shortcuts
        sections.push(HelpSection {
            title: "Global".into(),
            entries: vec![
                ("q", "Quit"),
                ("?", "Help"),
                ("Alt+v", "Split vertical"),
                // ... from active bindings
            ],
        });

        // 2. Show mode-specific shortcuts
        match mode {
            InputMode::Normal | InputMode::Pane => {
                sections.push(HelpSection {
                    title: "Navigation".into(),
                    entries: vec![
                        ("j/k", "Navigate"),
                        ("Enter", "Select"),
                        ("Esc", "Back"),
                    ],
                });
            }
            _ => {}
        }

        // 3. Show view-specific shortcuts
        if let Some(ViewType::ResourceList(kind)) = view {
            sections.push(HelpSection {
                title: format!("{} Actions", kind.display_name()),
                entries: Self::resource_entries(kind),
            });
        }

        Self { sections }
    }

    fn resource_entries(kind: &ResourceKind) -> Vec<(&'static str, &'static str)> {
        let mut entries = vec![
            ("y", "View YAML"),
            ("d", "Describe"),
            ("Ctrl+d", "Delete"),
            ("/", "Filter"),
            ("s", "Sort"),
            ("a", "All namespaces"),
            (":", "Switch resource"),
        ];
        match kind {
            ResourceKind::Pods => {
                entries.push(("l", "Logs"));
                entries.push(("e", "Exec"));
            }
            ResourceKind::Deployments => {
                entries.push(("S", "Scale"));
                entries.push(("R", "Restart"));
            }
            ResourceKind::StatefulSets => {
                entries.push(("S", "Scale"));
            }
            _ => {}
        }
        entries
    }
}
```

The help pane rebuilds its content whenever focus changes or the resource
type switches. The `ShowHelp` command handler passes the current context.

## Tests

- ConfirmDialog renders message text and button labels
- ConfirmAction with pending delete → calls ActionExecutor::delete
- DenyAction clears pending confirmation and restores mode
- ToastMessage::success creates toast with 3s TTL
- ToastMessage::error creates toast with 5s TTL
- is_expired returns true after TTL
- cleanup_toasts removes only expired messages
- HelpPane::for_context with Pods shows Logs and Exec entries
- HelpPane::for_context with Deployments shows Scale and Restart entries
- HelpPane::for_context with ConfigMaps shows no Logs/Exec/Scale entries
- HelpPane entries reflect active keybindings (from dispatcher, not hardcoded)

## Demo

- [ ] Ctrl+d on pod → confirmation dialog appears
- [ ] Press 'y' → pod deleted, green toast "Deleted pod nginx-abc123"
- [ ] Press 'n' → dialog dismissed, nothing happens
- [ ] Failed delete → red toast with error message, stays 5s
- [ ] Toast auto-dismisses after TTL
- [ ] Multiple actions → toasts stack in bottom-right
- [ ] `?` on pod list → help shows Logs, Exec, Delete, YAML, etc.
- [ ] `?` on deployment list → help shows Scale, Restart instead of Logs/Exec
- [ ] `?` on configmap list → help shows only YAML, Describe, Delete
