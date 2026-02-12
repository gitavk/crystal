# Step 4.8 — Resource Switcher (Command Palette)

> `feat(app): add resource switcher command palette`

## Goal

Implement a k9s-style command palette activated with `:` that lets users
quickly switch between resource types. The switcher supports fuzzy matching
over resource short names and display names, rendering as a centered overlay.

## Files

| File | Action |
|------|--------|
| `crates/crystal-app/src/resource_switcher.rs` | NEW — ResourceSwitcher state + logic |
| `crates/crystal-tui/src/widgets/resource_switcher.rs` | NEW — overlay widget |
| `crates/crystal-tui/src/widgets/mod.rs` | UPDATE — add module |
| `crates/crystal-app/src/keybindings.rs` | UPDATE — ResourceSwitcher mode bindings |

## ResourceSwitcher State

```rust
// crates/crystal-app/src/resource_switcher.rs

pub struct ResourceSwitcher {
    input: String,
    all_kinds: Vec<ResourceKind>,
    filtered_kinds: Vec<ResourceKind>,
    selected: usize,
}

impl ResourceSwitcher {
    pub fn new() -> Self {
        let all_kinds: Vec<ResourceKind> = ResourceKind::all().to_vec();
        let filtered_kinds = all_kinds.clone();
        Self {
            input: String::new(),
            all_kinds,
            filtered_kinds,
            selected: 0,
        }
    }

    pub fn on_input(&mut self, ch: char) {
        self.input.push(ch);
        self.filter();
    }

    pub fn on_backspace(&mut self) {
        self.input.pop();
        self.filter();
    }

    pub fn select_next(&mut self) {
        if !self.filtered_kinds.is_empty() {
            self.selected = (self.selected + 1) % self.filtered_kinds.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.filtered_kinds.is_empty() {
            self.selected = self.selected
                .checked_sub(1)
                .unwrap_or(self.filtered_kinds.len() - 1);
        }
    }

    pub fn confirm(&self) -> Option<ResourceKind> {
        self.filtered_kinds.get(self.selected).cloned()
    }

    fn filter(&mut self) {
        let query = self.input.to_lowercase();
        if query.is_empty() {
            self.filtered_kinds = self.all_kinds.clone();
        } else {
            self.filtered_kinds = self.all_kinds.iter()
                .filter(|k| {
                    k.short_name().to_lowercase().contains(&query)
                        || k.display_name().to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }
        // Clamp selection
        if self.selected >= self.filtered_kinds.len() {
            self.selected = self.filtered_kinds.len().saturating_sub(1);
        }
    }

    // Accessors for the widget
    pub fn input(&self) -> &str { &self.input }
    pub fn filtered(&self) -> &[ResourceKind] { &self.filtered_kinds }
    pub fn selected(&self) -> usize { self.selected }
}
```

## Overlay Widget

```rust
// crates/crystal-tui/src/widgets/resource_switcher.rs

pub struct ResourceSwitcherWidget<'a> {
    pub input: &'a str,
    pub items: &'a [ResourceKind],
    pub selected: usize,
}
```

### Rendering Layout

```
┌─ Switch Resource ─────────────┐
│ :deploy_                       │
│                                │
│   po     Pods                  │
│ > deploy Deployments       ◄── │  selected (accent color)
│   svc    Services              │
│   sts    StatefulSets          │
│   ds     DaemonSets            │
│   job    Jobs                  │
│   ...                          │
└────────────────────────────────┘
```

- Centered on screen, fixed width (40 chars), height = min(items + 3, 20)
- Input line at top with `:` prefix and cursor
- Items listed with short_name + display_name in two columns
- Selected item highlighted with `theme::ACCENT` and `>` marker
- Background dimmed (semi-transparent overlay effect via Block)

### Rendering Implementation

```rust
impl<'a> Widget for ResourceSwitcherWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 1. Calculate centered rect (40 wide, dynamic height)
        let width = 40;
        let height = (self.items.len() + 3).min(20) as u16;
        let popup = centered_rect(width, height, area);

        // 2. Clear background area
        Clear.render(popup, buf);

        // 3. Render bordered block with title
        let block = Block::default()
            .title(" Switch Resource ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT));
        let inner = block.inner(popup);
        block.render(popup, buf);

        // 4. Render input line: ":{input}_"
        // 5. Render filtered items with selection highlight
    }
}
```

## Keyboard Handling

When `InputMode::ResourceSwitcher` is active:

| Key | Command |
|-----|---------|
| Any character | `ResourceSwitcherInput(ch)` |
| Backspace | `ResourceSwitcherBackspace` |
| j / Down | `ResourceSwitcherInput` → select_next (handled in App) |
| k / Up | `ResourceSwitcherInput` → select_prev (handled in App) |
| Enter | `ResourceSwitcherConfirm` |
| Esc | `DenyAction` (closes switcher) |

The dispatcher routes all keys to ResourceSwitcher commands when in this mode.
No global keybindings except Esc are active.

## App Integration

```rust
// crates/crystal-app/src/app.rs

pub struct App {
    // ... existing fields ...
    resource_switcher: Option<ResourceSwitcher>,
}

// In handle_command():
Command::EnterResourceSwitcher => {
    self.resource_switcher = Some(ResourceSwitcher::new());
    self.dispatcher.set_mode(InputMode::ResourceSwitcher);
}

Command::ResourceSwitcherInput(ch) => {
    if let Some(ref mut sw) = self.resource_switcher {
        if ch == 'j' || ch == '\x1b' /* down arrow handled separately */ {
            sw.select_next();
        } else {
            sw.on_input(ch);
        }
    }
}

Command::ResourceSwitcherConfirm => {
    if let Some(ref sw) = self.resource_switcher {
        if let Some(kind) = sw.confirm() {
            self.switch_resource(focused_pane_id, kind).await;
        }
    }
    self.resource_switcher = None;
    self.dispatcher.set_mode(InputMode::Normal);
}

Command::DenyAction => {
    // Also closes resource switcher
    self.resource_switcher = None;
    self.dispatcher.set_mode(InputMode::Normal);
}
```

## RenderContext Extension

```rust
// The resource switcher is rendered as an overlay on top of everything else.
// Add to RenderContext:
pub struct RenderContext<'a> {
    // ... existing fields ...
    pub resource_switcher: Option<ResourceSwitcherView<'a>>,
}

pub struct ResourceSwitcherView<'a> {
    pub input: &'a str,
    pub items: &'a [ResourceKind],
    pub selected: usize,
}
```

The layout renderer checks if `resource_switcher` is Some and renders the
overlay widget after all panes, similar to the namespace selector pattern.

## Tests

- Empty input shows all 14 resource kinds
- "po" filters to just Pods
- "dep" filters to Deployments
- "s" filters to Services, StatefulSets, Secrets (all containing 's')
- "xyz" filters to empty list
- select_next wraps from last to first
- select_prev wraps from first to last
- confirm returns None when list is empty
- backspace restores previous filter state

## Demo

- [ ] Press `:` → overlay appears with all resource types
- [ ] Type "po" → filtered to Pods
- [ ] Press Enter → switches to Pods view, overlay closes
- [ ] Type "deploy" → filtered to Deployments
- [ ] j/k navigates the list
- [ ] Esc closes without switching
- [ ] Backspace removes last character, filter updates
