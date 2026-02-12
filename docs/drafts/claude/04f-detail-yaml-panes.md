# Step 4.6 — Detail Pane & YAML Pane

> `feat(app): implement detail pane and YAML pane`

## Goal

Implement two new pane types:

1. **ResourceDetailPane** — scrollable view of a resource's detail sections
   (metadata, status, containers, conditions, etc.)
2. **YamlPane** — full YAML viewer with syntax highlighting and search

Both implement the existing `Pane` trait and integrate with the pane tree
system. They are opened by splitting from a resource list pane.

## Files

| File | Action |
|------|--------|
| `crates/crystal-app/src/panes/resource_detail.rs` | NEW — detail pane |
| `crates/crystal-app/src/panes/yaml_pane.rs` | NEW — YAML viewer pane |
| `crates/crystal-tui/src/widgets/breadcrumb.rs` | NEW — breadcrumb widget |
| `crates/crystal-tui/src/pane.rs` | UPDATE — ensure ViewType::Detail and ViewType::Yaml exist |

## ResourceDetailPane

```rust
// crates/crystal-app/src/panes/resource_detail.rs

pub struct ResourceDetailPane {
    view_type: ViewType,
    kind: ResourceKind,
    name: String,
    namespace: Option<String>,
    sections: Vec<DetailSection>,
    scroll_offset: usize,
    selected_section: usize,
    visible_height: u16,
}

impl ResourceDetailPane {
    pub fn new(kind: ResourceKind, name: String, namespace: Option<String>, sections: Vec<DetailSection>) -> Self {
        Self {
            view_type: ViewType::Detail(kind.clone(), name.clone()),
            kind,
            name,
            namespace,
            sections,
            scroll_offset: 0,
            selected_section: 0,
            visible_height: 0,
        }
    }
}
```

### Rendering Layout

```
┌─ Pods > nginx-abc123 ────────────────┐
│                                       │
│ ┌─ Metadata ────────────────────────┐ │
│ │ Name:       nginx-abc123          │ │
│ │ Namespace:  default               │ │
│ │ Node:       worker-01             │ │
│ │ IP:         10.244.1.5            │ │
│ └───────────────────────────────────┘ │
│                                       │
│ ┌─ Status ──────────────────────────┐ │
│ │ Phase:      Running      (green)  │ │
│ │ Ready:      1/1                   │ │
│ │ Restarts:   0                     │ │
│ └───────────────────────────────────┘ │
│                                       │
│ ┌─ Containers ──────────────────────┐ │
│ │ nginx:                            │ │
│ │   Image:    nginx:1.25            │ │
│ │   Ready:    true                  │ │
│ │   Restarts: 0                     │ │
│ └───────────────────────────────────┘ │
└───────────────────────────────────────┘
```

- Top line: breadcrumb showing `Kind > Name`
- Each section rendered as a bordered block with title
- Status values color-coded using theme (Running=green, Failed=red, etc.)
- Selected section has accent-colored border
- Content scrolls vertically when it overflows

### Pane Trait Implementation

```rust
impl Pane for ResourceDetailPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        // 1. Render breadcrumb widget at top (1 line)
        // 2. For each section in scroll range:
        //    - Render bordered block with section title
        //    - Render key-value pairs inside block
        //    - Highlight selected section border with theme::ACCENT
        // 3. Color-code known status values
    }

    fn handle_command(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::ScrollUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            PaneCommand::ScrollDown => {
                self.scroll_offset += 1; // clamp in render
            }
            PaneCommand::SelectNext => {
                // Move to next section
                if self.selected_section < self.sections.len().saturating_sub(1) {
                    self.selected_section += 1;
                }
            }
            PaneCommand::SelectPrev => {
                self.selected_section = self.selected_section.saturating_sub(1);
            }
            PaneCommand::Back => {
                // Signal to App to close this pane (handled at App level)
            }
            _ => {}
        }
    }

    fn view_type(&self) -> &ViewType { &self.view_type }
}
```

### Keybindings

| Key | Command | Notes |
|-----|---------|-------|
| j/k | ScrollUp/ScrollDown | scroll within view |
| Tab | SelectNext | next section |
| Shift+Tab | SelectPrev | prev section |
| y | ViewYaml | open YAML pane |
| Esc/q | Back | close detail pane |
| e | ExecInto | pods only |
| l | ViewLogs | pods only |

## YamlPane

```rust
// crates/crystal-app/src/panes/yaml_pane.rs

pub struct YamlPane {
    view_type: ViewType,
    resource_name: String,
    content: String,
    styled_lines: Vec<Line<'static>>,  // pre-computed ratatui styled lines
    total_lines: usize,
    scroll_offset: usize,
    search_query: Option<String>,
    search_matches: Vec<usize>,        // line numbers with matches
    current_match: usize,
    visible_height: u16,
}

impl YamlPane {
    pub fn new(kind: ResourceKind, name: String, yaml_content: String) -> Self {
        let styled_lines = Self::highlight_yaml(&yaml_content);
        let total_lines = styled_lines.len();
        Self {
            view_type: ViewType::Yaml(kind, name.clone()),
            resource_name: name,
            content: yaml_content,
            styled_lines,
            total_lines,
            scroll_offset: 0,
            search_query: None,
            search_matches: vec![],
            current_match: 0,
            visible_height: 0,
        }
    }
}
```

### YAML Syntax Highlighting

```rust
impl YamlPane {
    /// Parse YAML content line by line and apply syntax highlighting.
    fn highlight_yaml(content: &str) -> Vec<Line<'static>> {
        content.lines().map(|line| {
            // Detect line type by pattern:
            // - "key:" at start → key in ACCENT, value styled by type
            // - "- " at start → list marker in TEXT_DIM
            // - "# comment" → full line in TEXT_DIM italic
            // - quoted strings → strings in default text
            // - numbers → STATUS_RUNNING color
            // - true/false/null → STATUS_RUNNING color

            if let Some((key, value)) = line.split_once(':') {
                let indent = &key[..key.len() - key.trim_start().len()];
                let key_text = key.trim_start();
                // Build spans: indent + styled key + ":" + styled value
            } else {
                // Plain line or comment
            }
        }).collect()
    }
}
```

Theme color mapping:
| YAML Element | Theme Color |
|-------------|-------------|
| Keys | `theme::ACCENT` |
| String values | default text |
| Numbers | `theme::STATUS_RUNNING` |
| Booleans/null | `theme::STATUS_RUNNING` |
| Comments | `theme::TEXT_DIM` italic |
| List markers (-) | `theme::TEXT_DIM` |
| Line numbers | `theme::TEXT_DIM` |

### Rendering Layout

```
┌─ YAML: nginx-abc123 ─────── 42 lines ┐
│  1 │ apiVersion: v1                    │
│  2 │ kind: Pod                         │
│  3 │ metadata:                         │
│  4 │   name: nginx-abc123              │
│  5 │   namespace: default              │
│  6 │   labels:                         │
│  7 │     app: nginx       ← matched   │
│    │ ...                               │
│ /nginx                    [1/3 matches]│
└────────────────────────────────────────┘
```

- Line numbers in left gutter
- Search bar at bottom when search is active
- Match count shown at bottom right
- Current match line highlighted with `theme::SELECTION_BG`

### Pane Trait Implementation

```rust
impl Pane for YamlPane {
    fn handle_command(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::ScrollUp => self.scroll_up(),
            PaneCommand::ScrollDown => self.scroll_down(),
            PaneCommand::SearchInput(ch) => {
                self.search_query.get_or_insert_with(String::new).push(*ch);
                self.update_search_matches();
            }
            PaneCommand::SearchConfirm => {
                // Jump to next match
                if !self.search_matches.is_empty() {
                    self.current_match = (self.current_match + 1) % self.search_matches.len();
                    self.scroll_to_match();
                }
            }
            PaneCommand::SearchClear => {
                self.search_query = None;
                self.search_matches.clear();
            }
            PaneCommand::Back => { /* close pane */ }
            _ => {}
        }
    }
}
```

### Keybindings

| Key | Command |
|-----|---------|
| j/k | ScrollUp/ScrollDown |
| / | Enter search mode |
| n | SearchConfirm (next match) |
| N | Previous match |
| Esc/q | Back (close YAML view) |

## ViewType Extension

Ensure ViewType has a Yaml variant:

```rust
// crates/crystal-tui/src/pane.rs
pub enum ViewType {
    // ... existing ...
    Yaml(ResourceKind, String),  // NEW — kind + resource name
}
```

## Breadcrumb Widget

```rust
// crates/crystal-tui/src/widgets/breadcrumb.rs

pub struct BreadcrumbWidget<'a> {
    pub segments: &'a [&'a str],  // e.g., ["Pods", "nginx-abc123"]
}

// Renders: "Pods > nginx-abc123"
// Separator ">" in TEXT_DIM, segments in default text
// Last segment in ACCENT (current location)
```

## Tests

- `ResourceDetailPane::handle_command(ScrollDown)` increments scroll_offset
- `ResourceDetailPane::handle_command(SelectNext)` advances selected_section
- `ResourceDetailPane` renders all sections from provided DetailSection vec
- `YamlPane::highlight_yaml()` produces correct span styles for keys, values, comments
- `YamlPane` search: "nginx" finds correct line numbers
- `YamlPane` search: next match wraps around
- `YamlPane` scroll clamps to content bounds
- Breadcrumb renders correct "A > B > C" format

## Demo

- [ ] Select pod in list → detail pane opens in horizontal split
- [ ] Detail shows Metadata, Status, Containers sections
- [ ] Tab/Shift+Tab navigates between sections
- [ ] j/k scrolls through long detail content
- [ ] Press 'y' on detail → YAML pane opens
- [ ] YAML has line numbers and syntax colors
- [ ] '/' + "nginx" highlights all matches, 'n' jumps between them
- [ ] Esc closes YAML pane, returns to detail
