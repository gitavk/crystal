# Step 4.10 — Wire Everything in App Core

> `feat(app): wire resource views, actions, and watchers into App core`

## Goal

This is the final integration step. Connect all the pieces built in steps
4.1–4.9 into the App's event loop: command routing, watcher management,
pane creation, action execution, overlay rendering, and RenderContext.

After this step, the full Stage 4 resource view system is operational.

## Files

| File | Action |
|------|--------|
| `crates/crystal-app/src/app.rs` | UPDATE — command handlers, watcher lifecycle, state |
| `crates/crystal-app/src/event.rs` | UPDATE — ResourceUpdate event handling |
| `crates/crystal-tui/src/layout.rs` | UPDATE — render overlays (dialog, toast, switcher) |

## App State Additions

```rust
pub struct App {
    // --- existing fields ---
    running: bool,
    tick_rate: Duration,
    kube_client: Option<KubeClient>,
    context_resolver: ContextResolver,
    dispatcher: KeybindingDispatcher,
    namespaces: Vec<String>,
    namespace_filter: String,
    namespace_selected: usize,
    pod_watcher: Option<ResourceWatcher>,  // REMOVE — replaced by active_watchers
    pending_namespace_switch: Option<String>,
    tab_manager: TabManager,
    panes: HashMap<PaneId, Box<dyn Pane>>,
    pods_pane_id: PaneId,

    // --- NEW fields ---
    active_watchers: HashMap<PaneId, CancellationToken>,
    resource_switcher: Option<ResourceSwitcher>,
    pending_confirmation: Option<PendingConfirmation>,
    toasts: Vec<ToastMessage>,
}
```

## Command Routing

Full `handle_command()` with all new variants:

```rust
impl App {
    pub async fn handle_command(&mut self, cmd: Command) {
        match cmd {
            // --- existing handlers (unchanged) ---
            Command::Quit => { self.running = false; }
            Command::ShowHelp => { /* open context-sensitive HelpPane */ }
            Command::FocusNextPane => { /* ... */ }
            // ... all existing pane/tab/mode commands ...

            // --- Resource actions ---
            Command::ViewYaml => {
                if let Some((kind, name, ns)) = self.selected_resource_info() {
                    let client = self.kube_client.as_ref().unwrap().inner_client();
                    let executor = ActionExecutor::new(client);
                    // Spawn async task to fetch YAML
                    // On result: create YamlPane, split focused pane horizontally
                    // On error: push error toast
                }
            }

            Command::ViewDescribe => {
                if let Some((kind, name, ns)) = self.selected_resource_info() {
                    // Similar to ViewYaml but uses executor.describe()
                    // Opens result in a YamlPane (describe output is plain text)
                }
            }

            Command::DeleteResource => {
                if let Some((kind, name, ns)) = self.selected_resource_info() {
                    self.pending_confirmation = Some(PendingConfirmation {
                        message: format!("Delete {} {} in namespace {}?",
                            kind.display_name(), name, ns),
                        action: PendingAction::Delete { kind, name, namespace: ns },
                    });
                    self.dispatcher.set_mode(InputMode::ConfirmDialog);
                }
            }

            Command::ConfirmAction => {
                if let Some(confirmation) = self.pending_confirmation.take() {
                    let client = self.kube_client.as_ref().unwrap().inner_client();
                    let executor = ActionExecutor::new(client);
                    match confirmation.action {
                        PendingAction::Delete { kind, name, namespace } => {
                            // Dispatch to correct delete method based on kind
                            // match kind { Pods => executor.delete::<Pod>(...), ... }
                            // On success: push success toast
                            // On error: push error toast
                        }
                    }
                    self.dispatcher.set_mode(InputMode::Normal);
                }
            }

            Command::DenyAction => {
                self.pending_confirmation = None;
                self.resource_switcher = None;
                self.dispatcher.set_mode(InputMode::Normal);
            }

            Command::ScaleResource => {
                // Show scale input dialog (stretch goal — can start with hardcoded +1/-1)
                // Or: prompt in status bar for replica count
            }

            Command::RestartRollout => {
                if let Some((kind, name, ns)) = self.selected_resource_info() {
                    if kind == ResourceKind::Deployments {
                        let client = self.kube_client.as_ref().unwrap().inner_client();
                        let executor = ActionExecutor::new(client);
                        match executor.restart_rollout(&name, &ns).await {
                            Ok(()) => self.push_toast(
                                ToastMessage::success(format!("Restarted {}", name))),
                            Err(e) => self.push_toast(
                                ToastMessage::error(format!("Restart failed: {}", e))),
                        }
                    }
                }
            }

            Command::ViewLogs => {
                // Forward reference: opens LogsPane in Stage 5
                // For now: push info toast "Logs not yet implemented"
            }

            Command::ExecInto => {
                // Forward reference: opens ExecPane in Stage 5
                // For now: push info toast "Exec not yet implemented"
            }

            Command::ToggleAllNamespaces => {
                // Toggle namespace scope and restart watcher
                // See step 4.5 for details
            }

            // --- Resource switcher ---
            Command::EnterResourceSwitcher => {
                self.resource_switcher = Some(ResourceSwitcher::new());
                self.dispatcher.set_mode(InputMode::ResourceSwitcher);
            }

            Command::ResourceSwitcherInput(ch) => {
                if let Some(ref mut sw) = self.resource_switcher {
                    sw.on_input(ch);
                }
            }

            Command::ResourceSwitcherBackspace => {
                if let Some(ref mut sw) = self.resource_switcher {
                    sw.on_backspace();
                }
            }

            Command::ResourceSwitcherConfirm => {
                if let Some(ref sw) = self.resource_switcher {
                    if let Some(kind) = sw.confirm() {
                        let pane_id = self.focused_pane_id();
                        self.switch_resource(pane_id, kind).await;
                    }
                }
                self.resource_switcher = None;
                self.dispatcher.set_mode(InputMode::Normal);
            }

            // --- Sort ---
            Command::SortByColumn => {
                // Route to focused pane as PaneCommand::SortByColumn(next_col)
            }

            // --- Pane commands (existing, extended) ---
            Command::Pane(pane_cmd) => {
                if let Some(pane) = self.focused_pane_mut() {
                    pane.handle_command(&pane_cmd);
                }
            }
        }
    }
}
```

## Helper Methods

```rust
impl App {
    /// Get the selected resource's kind, name, and namespace from the focused pane.
    /// Returns None if the focused pane is not a resource list or nothing is selected.
    fn selected_resource_info(&self) -> Option<(ResourceKind, String, String)> {
        let pane_id = self.focused_pane_id();
        let pane = self.panes.get(&pane_id)?;
        // Downcast to ResourceListPane, get selected row's name + namespace
        // Return (kind, name, namespace)
    }

    /// Get a mutable reference to the focused pane.
    fn focused_pane_mut(&mut self) -> Option<&mut Box<dyn Pane>> {
        let pane_id = self.focused_pane_id();
        self.panes.get_mut(&pane_id)
    }

    fn focused_pane_id(&self) -> PaneId {
        self.tab_manager.active().focused_pane
    }
}
```

## Event Handling

```rust
// In App::handle_event()
AppEvent::ResourceUpdate { pane_id, headers, rows } => {
    if let Some(pane) = self.panes.get_mut(&pane_id) {
        // Downcast to ResourceListPane, update its ResourceListState
        if let Some(list_pane) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
            list_pane.update_data(headers, rows);
        }
    }
}

AppEvent::ResourceError { pane_id, error } => {
    if let Some(pane) = self.panes.get_mut(&pane_id) {
        if let Some(list_pane) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
            list_pane.set_error(error);
        }
    }
}

AppEvent::Tick => {
    self.cleanup_toasts();
}
```

## Extended RenderContext

```rust
pub struct RenderContext<'a> {
    // --- existing fields ---
    pub cluster_name: Option<&'a str>,
    pub namespace: Option<&'a str>,
    pub namespace_selector: Option<NamespaceSelectorView<'a>>,
    pub pane_tree: &'a PaneTree,
    pub focused_pane: Option<PaneId>,
    pub fullscreen_pane: Option<PaneId>,
    pub panes: &'a HashMap<PaneId, Box<dyn Pane>>,
    pub tab_names: &'a [String],
    pub active_tab: usize,
    pub mode_name: &'a str,
    pub mode_hints: &'a [(String, String)],

    // --- NEW fields ---
    pub resource_switcher: Option<ResourceSwitcherView<'a>>,
    pub confirm_dialog: Option<ConfirmDialogView<'a>>,
    pub toasts: &'a [ToastMessage],
}
```

## Render Flow (Updated)

```
1. Render tab bar (top row)
2. Get active tab's pane tree
3. Calculate pane layout rects
4. Render borders
5. For each pane, render via Pane::render()
6. Render status bar (bottom row)
7. Render overlays (in order, last = on top):
   a. Namespace selector (if active)
   b. Resource switcher (if active)
   c. Confirm dialog (if active)
   d. Toasts (always, if any non-expired)
```

Overlays render on top of everything. Only one modal (namespace selector,
resource switcher, or confirm dialog) can be active at a time.

## Pane Creation Flows

### Open Detail Pane (Enter on resource list)

```rust
Command::Pane(PaneCommand::Select) => {
    if let Some((kind, name, ns)) = self.selected_resource_info() {
        // 1. Get detail sections from the summary (already in ResourceListState)
        //    OR fetch fresh object via ActionExecutor
        // 2. Create ResourceDetailPane with sections
        // 3. Split focused pane horizontally
        // 4. Register new pane in self.panes
    }
}
```

### Open YAML Pane (y on resource)

```rust
// 1. Spawn async: ActionExecutor::get_yaml()
// 2. On completion: create YamlPane with content
// 3. Split focused pane horizontally
// 4. Register new pane
```

### Close Detail/YAML Pane (Esc/q)

```rust
PaneCommand::Back => {
    // If focused pane is Detail or YAML:
    // 1. Close pane in pane tree
    // 2. Remove from self.panes
    // 3. Focus moves to sibling (existing close logic)
}
```

## Watcher Lifecycle

| Event | Action |
|-------|--------|
| App starts | Start pod watcher for initial pane |
| Resource switch (`:deploy`) | Cancel old watcher, start new for deployment |
| Namespace switch | Cancel all watchers, restart with new namespace |
| Pane close | Cancel watcher for that pane |
| Tab close | Cancel all watchers for panes in that tab |
| App quit | Cancel all watchers (via dropping CancellationTokens) |

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Delete fails (permission denied) | Error toast: "Delete failed: Forbidden" |
| Scale fails (invalid replicas) | Error toast: "Scale failed: ..." |
| YAML fetch fails | Error toast + pane not created |
| Watcher disconnects | ResourceError event → error shown in pane |
| Watcher reconnects | Automatic (kube-rs watcher handles reconnection) |

Errors are never modal dialogs — they appear as toasts or inline pane messages.
The app never crashes on K8s API errors.

## Tests

- Full event cycle: KeyEvent → Command → handler → state change → RenderContext updated
- ViewYaml command creates YamlPane in split
- DeleteResource → ConfirmDialog → ConfirmAction → delete called → toast shown
- DenyAction clears confirmation and restores mode
- ResourceSwitcher flow: Enter → type → confirm → watcher switch
- ResourceUpdate event updates correct pane's data
- ResourceError event shows error in correct pane
- Toast cleanup removes expired toasts on tick
- Pane close cancels associated watcher
- Multiple panes with different resource types run independent watchers

## Demo

This is the full Stage 4 integration demo:

- [ ] Launch with single pane showing pods
- [ ] `:deploy` switches to deployment view
- [ ] Select deployment → detail view opens in split
- [ ] `y` on deployment → YAML view opens
- [ ] `Ctrl+d` on pod → confirmation → `y` → deleted, toast shown
- [ ] `S` on deployment → scale changes (if implemented)
- [ ] `R` on deployment → restart toast
- [ ] `/nginx` filters, `s` sorts, `a` toggles all-ns
- [ ] `?` shows context-sensitive help
- [ ] Multiple panes showing different resources simultaneously
- [ ] Error responses show as toasts, never crash the app
