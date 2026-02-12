# Step 4.5 — ResourceListPane (Filter, Sort, All-Namespaces)

> `feat(app): add filter, sort, and all-namespaces to ResourceListPane`

## Goal

Extend the existing `ResourceListPane` to support:
- Fuzzy filtering by name (activated with `/`)
- Column sorting (activated with `s`, cycles through columns)
- All-namespaces toggle (activated with `a`)

The pane already renders a table from `ResourceListState`. This step adds
the interactive features that make it usable for real workflows.

## Files

| File | Action |
|------|--------|
| `crates/crystal-app/src/panes/resource_list.rs` | UPDATE — add filter, sort, namespace toggle state |
| `crates/crystal-app/src/state.rs` | UPDATE — add filter/sort fields to ResourceListState |
| `crates/crystal-tui/src/widgets/resource_list.rs` | UPDATE — render filter bar, sort indicators |

## Extended ResourceListPane

```rust
// crates/crystal-app/src/panes/resource_list.rs

pub struct ResourceListPane {
    kind: ResourceKind,
    state: ResourceListState,

    // NEW — filtering
    filter_text: String,
    filtered_indices: Vec<usize>,  // indices into state.items that match filter

    // NEW — sorting
    sort_column: Option<usize>,    // None = natural order (as received from watcher)
    sort_ascending: bool,

    // NEW — namespace scope
    all_namespaces: bool,
}
```

## Filter Logic

```rust
impl ResourceListPane {
    /// Apply fuzzy filter across all columns, prioritizing name (column 0).
    fn apply_filter(&mut self) {
        if self.filter_text.is_empty() {
            // Show all items
            self.filtered_indices = (0..self.state.items.len()).collect();
        } else {
            let query = self.filter_text.to_lowercase();
            self.filtered_indices = self.state.items.iter()
                .enumerate()
                .filter(|(_, row)| {
                    row.iter().any(|cell| cell.to_lowercase().contains(&query))
                })
                .map(|(i, _)| i)
                .collect();
        }
        // Reset selection to first match
        self.state.selected = if self.filtered_indices.is_empty() {
            None
        } else {
            Some(0)
        };
    }
}
```

## Sort Logic

```rust
impl ResourceListPane {
    /// Sort by the given column index. If already sorting by this column,
    /// toggle ascending/descending. If a different column, sort ascending.
    fn sort_by_column(&mut self, col: usize) {
        if self.sort_column == Some(col) {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = Some(col);
            self.sort_ascending = true;
        }
        self.apply_sort();
    }

    fn apply_sort(&mut self) {
        let Some(col) = self.sort_column else { return };
        let asc = self.sort_ascending;

        self.filtered_indices.sort_by(|&a, &b| {
            let va = &self.state.items[a][col];
            let vb = &self.state.items[b][col];
            let ord = va.cmp(vb);
            if asc { ord } else { ord.reverse() }
        });
    }
}
```

## PaneCommand Handling

```rust
impl Pane for ResourceListPane {
    fn handle_command(&mut self, cmd: &PaneCommand) {
        match cmd {
            // Existing
            PaneCommand::SelectNext => {
                // Navigate within filtered_indices, not raw items
            }
            PaneCommand::SelectPrev => { /* ... */ }
            PaneCommand::Select => { /* ... */ }

            // NEW
            PaneCommand::Filter(text) => {
                self.filter_text = text.clone();
                self.apply_filter();
                self.apply_sort(); // re-sort after filter
            }
            PaneCommand::ClearFilter => {
                self.filter_text.clear();
                self.apply_filter();
                self.apply_sort();
            }
            PaneCommand::SortByColumn(col) => {
                self.sort_by_column(*col);
            }
            PaneCommand::ToggleSortOrder => {
                self.sort_ascending = !self.sort_ascending;
                self.apply_sort();
            }
            _ => {}
        }
    }
}
```

## Widget Rendering Updates

```rust
// crates/crystal-tui/src/widgets/resource_list.rs — updates

// 1. Filter bar: when filter is active, render a top row:
//    "Filter: nginx_" with cursor
//    Use theme::TEXT_DIM for "Filter:" label, theme::ACCENT for input text

// 2. Sort indicator: in the header row, append ▲ or ▼ to the sorted column
//    e.g., "NAME ▲" or "AGE ▼"

// 3. Row count: show "42 items" or "12/42 items" (filtered/total) in footer

// 4. The widget receives filtered+sorted indices and renders only matching rows
```

## Keyboard Flow

```
User presses '/' (in Normal/Pane mode)
  → Command::EnterMode(InputMode::FilterInput)
  → Status bar shows "FILTER" mode
  → All character keys → Command::Pane(PaneCommand::Filter(accumulated_text))
  → Esc → Command::Pane(PaneCommand::ClearFilter) + Command::ExitMode
  → Enter → Command::ExitMode (keeps filter active)

User presses 's' (in Normal/Pane mode)
  → Command::SortByColumn
  → App routes to focused pane as PaneCommand::SortByColumn(next_column)
  → Column index cycles: 0 → 1 → 2 → ... → 0

User presses 'a' (in Normal/Pane mode)
  → Command::ToggleAllNamespaces
  → App::handle_command() toggles the namespace scope
  → Restarts watcher with all-namespaces Api (Api::all() vs Api::namespaced())
  → ResourceListState set to loading while new data arrives
```

## All-Namespaces Toggle

This is handled at the App level (not pane level) because it requires
restarting the watcher with a different API scope:

```rust
// In App::handle_command()
Command::ToggleAllNamespaces => {
    let pane = self.focused_resource_list_pane_mut();
    pane.all_namespaces = !pane.all_namespaces;

    // Restart watcher with correct API scope
    let client = self.kube_client.as_ref().unwrap().inner_client();
    let api = if pane.all_namespaces {
        Api::<K>::all(client)
    } else {
        Api::<K>::namespaced(client, &self.current_namespace())
    };
    self.start_watcher_for_pane(pane_id, &pane.kind).await;
}
```

## Tests

- Filter "ngi" matches row with name "nginx-pod-abc123"
- Filter matches across any column (not just name)
- Empty filter shows all items
- Sort by column 0 ascending: alphabetical order
- Sort toggle: same column press flips direction
- Different column press resets to ascending
- Filter + sort compose: filter first, then sort filtered results
- Selection resets to 0 after filter change
- Filtered indices stay in bounds when items update from watcher

## Demo

- [ ] Press `/`, type "nginx" → only matching pods shown
- [ ] Press Esc → filter cleared, all pods visible
- [ ] Press `s` → sorted by NAME ascending (▲ indicator)
- [ ] Press `s` again → sorted by READY
- [ ] Press `a` → "All Namespaces" shown, pods from all namespaces appear
- [ ] Press `a` again → back to current namespace only
- [ ] Filter active + sort active → both compose correctly
