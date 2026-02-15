use std::collections::HashMap;
use std::time::Duration;

use crossterm::event::{KeyEvent, KeyEventKind};
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, StatefulSet};
use k8s_openapi::api::batch::v1::CronJob;
use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::{
    ConfigMap, Namespace, Node, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service,
};
use k8s_openapi::api::networking::v1::Ingress;
use kube::Api;
use ratatui::backend::Backend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crystal_core::informer::{ResourceEvent, ResourceWatcher};
use crystal_core::resource::{DetailSection, ResourceSummary};
use crystal_core::*;
use crystal_core::{ContextResolver, KubeClient};
use crystal_tui::layout::{ConfirmDialogView, NamespaceSelectorView, RenderContext, ResourceSwitcherView};
use crystal_tui::pane::{
    find_pane_in_direction, Direction, Pane, PaneCommand, PaneId, ResourceKind, SplitDirection, ViewType,
};
use crystal_tui::tab::TabManager;
use crystal_tui::widgets::toast::ToastMessage;

use crate::command::{Command, InputMode};
use crate::event::{AppEvent, EventHandler};
use crate::keybindings::KeybindingDispatcher;
use crate::panes::{HelpPane, LogsPane, ResourceDetailPane, ResourceListPane, YamlPane};
use crate::resource_switcher::ResourceSwitcher;

#[derive(Debug, Clone)]
pub enum PendingAction {
    Delete { kind: ResourceKind, name: String, namespace: String },
}

pub struct PendingConfirmation {
    pub message: String,
    pub action: PendingAction,
}

pub struct App {
    running: bool,
    tick_rate: Duration,
    kube_client: Option<KubeClient>,
    context_resolver: ContextResolver,
    dispatcher: KeybindingDispatcher,
    namespaces: Vec<String>,
    namespace_filter: String,
    namespace_selected: usize,
    /// Active watchers keyed by pane ID.
    /// Each pane showing a resource list has its own watcher.
    /// When a pane switches resource type, its watcher is cancelled and a new one spawned.
    /// Dropping a ResourceWatcher cancels its background task automatically.
    active_watchers: HashMap<PaneId, ResourceWatcher>,
    filter_input_buffer: String,
    resource_switcher: Option<ResourceSwitcher>,
    pending_confirmation: Option<PendingConfirmation>,
    toasts: Vec<ToastMessage>,
    tab_manager: TabManager,
    panes: HashMap<PaneId, Box<dyn Pane>>,
    pods_pane_id: PaneId,
    app_tx: mpsc::UnboundedSender<AppEvent>,
}

impl App {
    pub async fn new(tick_rate_ms: u64, dispatcher: KeybindingDispatcher) -> Self {
        let mut context_resolver = ContextResolver::new();
        let kube_client = match KubeClient::from_kubeconfig().await {
            Ok(client) => {
                let ctx = client.cluster_context();
                context_resolver.set_context(ctx);
                Some(client)
            }
            Err(e) => {
                tracing::warn!("Failed to connect to cluster: {e}");
                None
            }
        };

        let pod_headers = vec![
            "NAME".into(),
            "NAMESPACE".into(),
            "STATUS".into(),
            "READY".into(),
            "RESTARTS".into(),
            "AGE".into(),
            "NODE".into(),
        ];

        let pods_pane = ResourceListPane::new(ResourceKind::Pods, pod_headers);
        let tab_manager = TabManager::new(ViewType::ResourceList(ResourceKind::Pods));
        let pods_pane_id = 1;

        let mut panes: HashMap<PaneId, Box<dyn Pane>> = HashMap::new();
        panes.insert(pods_pane_id, Box::new(pods_pane));

        // Create a temporary channel to get the sender
        let (tx, _rx) = mpsc::unbounded_channel();

        Self {
            running: true,
            tick_rate: Duration::from_millis(tick_rate_ms),
            kube_client,
            context_resolver,
            dispatcher,
            namespaces: Vec::new(),
            namespace_filter: String::new(),
            namespace_selected: 0,
            active_watchers: HashMap::new(),
            filter_input_buffer: String::new(),
            resource_switcher: None,
            pending_confirmation: None,
            toasts: Vec::new(),
            tab_manager,
            panes,
            pods_pane_id,
            app_tx: tx,
        }
    }

    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        let mut events = EventHandler::new(self.tick_rate);
        self.app_tx = events.app_tx();

        if let Some(client) = &self.kube_client {
            let ns = client.namespace().to_string();
            self.start_watcher_for_pane(self.pods_pane_id, &ResourceKind::Pods, &ns);

            if let Some(client) = &self.kube_client {
                match client.list_namespaces().await {
                    Ok(ns_list) => self.namespaces = ns_list,
                    Err(e) => tracing::warn!("Failed to list namespaces: {e}"),
                }
            }
        } else {
            self.with_pods_pane(|pane| {
                pane.state.loading = false;
                pane.state.error = Some("No cluster connection".into());
            });
        }

        while self.running {
            terminal.draw(|frame| {
                let (mut ctx, tab_names, hints) = self.build_render_context();
                ctx.tab_names = &tab_names;
                ctx.mode_hints = &hints;
                crystal_tui::layout::render_root(frame, &ctx);
            })?;

            match events.next().await? {
                AppEvent::Key(key) => self.handle_key(key),
                AppEvent::Tick => {
                    self.poll_runtime_panes();
                    self.toasts.retain(|t| !t.is_expired());
                }
                AppEvent::Resize(_, _) => {}
                AppEvent::ResourceUpdate { pane_id, headers: _, rows } => self.handle_resource_update(pane_id, rows),
                AppEvent::ResourceError { pane_id, error } => self.handle_resource_error(pane_id, error),
                AppEvent::Toast(toast) => self.toasts.push(toast),
                AppEvent::YamlReady { pane_id, kind, name, content } => {
                    self.open_yaml_pane(pane_id, kind, name, content);
                }
                AppEvent::LogsStreamReady { pane_id, stream } => {
                    self.attach_logs_stream(pane_id, stream);
                }
                AppEvent::LogsSnapshotReady { pane_id, lines } => {
                    self.attach_logs_snapshot(pane_id, lines);
                }
                AppEvent::LogsStreamError { pane_id, error } => {
                    self.attach_logs_error(pane_id, error);
                }
            }
        }

        Ok(())
    }

    /// Start watching a resource type for a specific pane.
    /// Cancels any existing watcher for that pane first.
    fn start_watcher_for_pane(&mut self, pane_id: PaneId, kind: &ResourceKind, namespace: &str) {
        // Cancel existing watcher if any (dropping it cancels the background task)
        self.active_watchers.remove(&pane_id);

        let Some(client) = &self.kube_client else {
            return;
        };

        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();

        // Helper to bridge ResourceEvent<S> to AppEvent::ResourceUpdate
        fn spawn_bridge<S>(
            pane_id: PaneId,
            mut rx: mpsc::Receiver<ResourceEvent<S>>,
            app_tx: mpsc::UnboundedSender<AppEvent>,
        ) where
            S: ResourceSummary + 'static,
        {
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let app_event = match event {
                        ResourceEvent::Updated(items) => {
                            let headers = if items.is_empty() {
                                vec![]
                            } else {
                                items[0].columns().into_iter().map(|(h, _)| h.to_string()).collect()
                            };
                            let rows = items.iter().map(|item| item.row()).collect();
                            AppEvent::ResourceUpdate { pane_id, headers, rows }
                        }
                        ResourceEvent::Error(error) => AppEvent::ResourceError { pane_id, error },
                    };
                    if app_tx.send(app_event).is_err() {
                        break;
                    }
                }
            });
        }

        let all_ns = namespace.is_empty();

        macro_rules! spawn_watcher {
            ($k8s_type:ty, $summary_type:ty) => {{
                let api: Api<$k8s_type> = if all_ns {
                    Api::all(kube_client.clone())
                } else {
                    Api::namespaced(kube_client.clone(), namespace)
                };
                let (tx, rx) = mpsc::channel(16);
                let watcher = ResourceWatcher::watch::<$k8s_type, $summary_type>(api, tx);
                self.active_watchers.insert(pane_id, watcher);
                spawn_bridge(pane_id, rx, app_tx);
            }};
            (cluster $k8s_type:ty, $summary_type:ty) => {{
                let api: Api<$k8s_type> = Api::all(kube_client.clone());
                let (tx, rx) = mpsc::channel(16);
                let watcher = ResourceWatcher::watch::<$k8s_type, $summary_type>(api, tx);
                self.active_watchers.insert(pane_id, watcher);
                spawn_bridge(pane_id, rx, app_tx);
            }};
        }

        match kind {
            ResourceKind::Pods => spawn_watcher!(Pod, PodSummary),
            ResourceKind::Deployments => spawn_watcher!(Deployment, DeploymentSummary),
            ResourceKind::Services => spawn_watcher!(Service, ServiceSummary),
            ResourceKind::StatefulSets => spawn_watcher!(StatefulSet, StatefulSetSummary),
            ResourceKind::DaemonSets => spawn_watcher!(DaemonSet, DaemonSetSummary),
            ResourceKind::Jobs => spawn_watcher!(Job, JobSummary),
            ResourceKind::CronJobs => spawn_watcher!(CronJob, CronJobSummary),
            ResourceKind::ConfigMaps => spawn_watcher!(ConfigMap, ConfigMapSummary),
            ResourceKind::Secrets => spawn_watcher!(Secret, SecretSummary),
            ResourceKind::Ingresses => spawn_watcher!(Ingress, IngressSummary),
            ResourceKind::Nodes => spawn_watcher!(cluster Node, NodeSummary),
            ResourceKind::Namespaces => spawn_watcher!(cluster Namespace, NamespaceSummary),
            ResourceKind::PersistentVolumes => spawn_watcher!(cluster PersistentVolume, PersistentVolumeSummary),
            ResourceKind::PersistentVolumeClaims => {
                spawn_watcher!(PersistentVolumeClaim, PersistentVolumeClaimSummary)
            }
            ResourceKind::Custom(_) => {
                tracing::warn!("Custom resource kinds are not yet supported");
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if let Some(cmd) = self.dispatcher.dispatch(key) {
            self.handle_command(cmd);
        }
    }

    fn handle_command(&mut self, cmd: Command) {
        match cmd {
            Command::Quit => self.running = false,
            Command::ShowHelp => self.toggle_help(),
            Command::FocusNextPane => self.focus_next(),
            Command::FocusPrevPane => self.focus_prev(),
            Command::SplitVertical => self.split_focused(SplitDirection::Vertical),
            Command::SplitHorizontal => self.split_focused(SplitDirection::Horizontal),
            Command::ClosePane => self.close_focused(),
            Command::EnterMode(mode) => {
                self.dispatcher.set_mode(mode);
                if mode == InputMode::NamespaceSelector {
                    self.namespace_filter.clear();
                    self.namespace_selected = 0;
                }
                if mode == InputMode::FilterInput {
                    self.filter_input_buffer.clear();
                    let focused = self.tab_manager.active().focused_pane;
                    if let Some(pane) = self.panes.get(&focused) {
                        if let Some(rp) = pane.as_any().downcast_ref::<ResourceListPane>() {
                            self.filter_input_buffer = rp.filter_text.clone();
                        }
                    }
                }
            }
            Command::ExitMode => self.dispatcher.set_mode(InputMode::Normal),
            Command::NamespaceConfirm => self.handle_namespace_confirm(),
            Command::NamespaceInput(c) => self.handle_namespace_input(c),
            Command::NamespaceBackspace => self.handle_namespace_backspace(),
            Command::FocusDirection(dir) => self.focus_direction(dir),
            Command::NewTab => self.new_tab(),
            Command::CloseTab => self.close_tab(),
            Command::NextTab => self.tab_manager.next_tab(),
            Command::PrevTab => self.tab_manager.prev_tab(),
            Command::GoToTab(n) => {
                if n > 0 {
                    self.tab_manager.switch_tab(n - 1);
                }
            }
            Command::ToggleFullscreen => self.toggle_fullscreen(),
            Command::ResizeGrow => {
                let focused = self.tab_manager.active().focused_pane;
                self.tab_manager.active_mut().pane_tree.resize(focused, 0.05, true);
            }
            Command::ResizeShrink => {
                let focused = self.tab_manager.active().focused_pane;
                self.tab_manager.active_mut().pane_tree.resize(focused, 0.05, false);
            }
            Command::Pane(ref pane_cmd) if self.dispatcher.mode() == InputMode::NamespaceSelector => {
                self.handle_namespace_nav(pane_cmd);
            }
            Command::Pane(ref pane_cmd) if self.dispatcher.mode() == InputMode::ResourceSwitcher => {
                if let Some(ref mut sw) = self.resource_switcher {
                    match pane_cmd {
                        PaneCommand::SelectNext => sw.select_next(),
                        PaneCommand::SelectPrev => sw.select_prev(),
                        _ => {}
                    }
                }
            }
            Command::Pane(pane_cmd) => {
                let focused = self.tab_manager.active().focused_pane;
                match &pane_cmd {
                    PaneCommand::Select => {
                        if let Some((kind, name, ns)) = self.selected_resource_info() {
                            self.open_detail_pane(kind, name, ns);
                            return;
                        }
                    }
                    PaneCommand::Back => {
                        if let Some(pane) = self.panes.get(&focused) {
                            let is_detail_or_yaml =
                                matches!(pane.view_type(), ViewType::Detail(..) | ViewType::Yaml(..));
                            if is_detail_or_yaml {
                                self.close_pane(focused);
                                return;
                            }
                        }
                    }
                    _ => {}
                }
                if let Some(pane) = self.panes.get_mut(&focused) {
                    pane.handle_command(&pane_cmd);
                }
            }

            Command::FilterInput(c) => {
                self.filter_input_buffer.push(c);
                let text = self.filter_input_buffer.clone();
                let focused = self.tab_manager.active().focused_pane;
                if let Some(pane) = self.panes.get_mut(&focused) {
                    pane.handle_command(&PaneCommand::Filter(text));
                }
            }
            Command::FilterBackspace => {
                self.filter_input_buffer.pop();
                if self.filter_input_buffer.is_empty() {
                    let focused = self.tab_manager.active().focused_pane;
                    if let Some(pane) = self.panes.get_mut(&focused) {
                        pane.handle_command(&PaneCommand::ClearFilter);
                    }
                } else {
                    let text = self.filter_input_buffer.clone();
                    let focused = self.tab_manager.active().focused_pane;
                    if let Some(pane) = self.panes.get_mut(&focused) {
                        pane.handle_command(&PaneCommand::Filter(text));
                    }
                }
            }
            Command::FilterCancel => {
                self.filter_input_buffer.clear();
                let focused = self.tab_manager.active().focused_pane;
                if let Some(pane) = self.panes.get_mut(&focused) {
                    pane.handle_command(&PaneCommand::ClearFilter);
                }
                self.dispatcher.set_mode(InputMode::Normal);
            }
            Command::SortByColumn => {
                let focused = self.tab_manager.active().focused_pane;
                if let Some(pane) = self.panes.get_mut(&focused) {
                    if let Some(rp) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                        let next_col = match rp.sort_column {
                            None => 0,
                            Some(c) => {
                                let num_cols = rp.state.headers.len();
                                if num_cols == 0 {
                                    0
                                } else {
                                    (c + 1) % num_cols
                                }
                            }
                        };
                        rp.sort_by_column(next_col);
                    }
                }
            }
            Command::ToggleAllNamespaces => {
                let focused = self.tab_manager.active().focused_pane;
                if let Some(pane) = self.panes.get_mut(&focused) {
                    if let Some(rp) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                        rp.all_namespaces = !rp.all_namespaces;
                        let kind = rp.kind().cloned();
                        let is_all = rp.all_namespaces;

                        if let Some(kind) = kind {
                            if kind.is_namespaced() {
                                if is_all {
                                    self.start_watcher_for_pane(focused, &kind, "");
                                } else {
                                    let ns = self.context_resolver.namespace().unwrap_or("default").to_string();
                                    self.start_watcher_for_pane(focused, &kind, &ns);
                                }
                                if let Some(pane) = self.panes.get_mut(&focused) {
                                    if let Some(rp) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                                        let headers = rp.state.headers.clone();
                                        rp.state = crate::state::ResourceListState::new(headers);
                                        rp.filtered_indices.clear();
                                    }
                                }
                            }
                        }
                    }
                }
            }

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
                let kind = self.resource_switcher.as_ref().and_then(|sw| sw.confirm());
                if let Some(kind) = kind {
                    self.switch_resource(kind);
                }
                self.resource_switcher = None;
                self.dispatcher.set_mode(InputMode::Normal);
            }
            Command::DenyAction => {
                self.resource_switcher = None;
                self.pending_confirmation = None;
                self.dispatcher.set_mode(InputMode::Normal);
            }

            Command::DeleteResource => {
                self.initiate_delete();
            }
            Command::ConfirmAction => {
                self.execute_confirmed_action();
            }

            Command::ViewYaml => {
                if let Some((kind, name, ns)) = self.selected_resource_info() {
                    let Some(client) = &self.kube_client else {
                        self.toasts.push(ToastMessage::error("No cluster connection"));
                        return;
                    };
                    let kube_client = client.inner_client();
                    let app_tx = self.app_tx.clone();
                    let focused = self.tab_manager.active().focused_pane;
                    let kind_clone = kind.clone();
                    let name_clone = name.clone();

                    tokio::spawn(async move {
                        let executor = crystal_core::ActionExecutor::new(kube_client);
                        let result = dispatch_get_yaml(&executor, &kind, &name, &ns).await;
                        let event = match result {
                            Ok(yaml) => AppEvent::YamlReady {
                                pane_id: focused,
                                kind: kind_clone,
                                name: name_clone,
                                content: yaml,
                            },
                            Err(e) => AppEvent::Toast(ToastMessage::error(format!("YAML fetch failed: {e}"))),
                        };
                        let _ = app_tx.send(event);
                    });
                }
            }

            Command::ViewDescribe => {
                if let Some((kind, name, ns)) = self.selected_resource_info() {
                    let Some(client) = &self.kube_client else {
                        self.toasts.push(ToastMessage::error("No cluster connection"));
                        return;
                    };
                    let kube_client = client.inner_client();
                    let app_tx = self.app_tx.clone();
                    let focused = self.tab_manager.active().focused_pane;
                    let kind_clone = kind.clone();
                    let name_clone = name.clone();

                    tokio::spawn(async move {
                        let executor = crystal_core::ActionExecutor::new(kube_client);
                        let result = dispatch_describe(&executor, &kind, &name, &ns).await;
                        let event = match result {
                            Ok(text) => AppEvent::YamlReady {
                                pane_id: focused,
                                kind: kind_clone,
                                name: name_clone,
                                content: text,
                            },
                            Err(e) => AppEvent::Toast(ToastMessage::error(format!("Describe failed: {e}"))),
                        };
                        let _ = app_tx.send(event);
                    });
                }
            }

            Command::RestartRollout => {
                if let Some((kind, name, ns)) = self.selected_resource_info() {
                    if kind == ResourceKind::Deployments {
                        let Some(client) = &self.kube_client else {
                            self.toasts.push(ToastMessage::error("No cluster connection"));
                            return;
                        };
                        let kube_client = client.inner_client();
                        let app_tx = self.app_tx.clone();

                        tokio::spawn(async move {
                            let executor = crystal_core::ActionExecutor::new(kube_client);
                            let toast = match executor.restart_rollout(&name, &ns).await {
                                Ok(()) => ToastMessage::success(format!("Restarted {name}")),
                                Err(e) => ToastMessage::error(format!("Restart failed: {e}")),
                            };
                            let _ = app_tx.send(AppEvent::Toast(toast));
                        });
                    } else {
                        self.toasts.push(ToastMessage::info("Restart rollout is only available for Deployments"));
                    }
                }
            }

            Command::ScaleResource => {
                self.toasts.push(ToastMessage::info("Scale not yet implemented"));
            }

            Command::ViewLogs => {
                self.open_logs_pane();
            }

            Command::ExecInto => {
                self.toasts.push(ToastMessage::info("Exec not yet implemented"));
            }

            // Terminal lifecycle (handled in future step)
            Command::TerminalSpawn
            | Command::TerminalClose { .. }
            | Command::TerminalResize { .. }
            | Command::TerminalInput { .. }
            | Command::ExecStart { .. }
            | Command::ExecClose { .. }
            | Command::LogsStart { .. }
            | Command::LogsStop { .. }
            | Command::PortForwardStart { .. }
            | Command::PortForwardStop { .. } => {}
        }
    }

    fn new_tab(&mut self) {
        let tab_count = self.tab_manager.tabs().len();
        let name = format!("Tab {}", tab_count + 1);
        let tab_id = self.tab_manager.new_tab(&name, ViewType::Empty);
        let pane_id = self.tab_manager.tabs().iter().find(|t| t.id == tab_id).unwrap().focused_pane;
        self.panes.insert(pane_id, Box::new(EmptyPane(ViewType::Empty)));
    }

    fn close_tab(&mut self) {
        let tab = self.tab_manager.active();
        let tab_id = tab.id;
        let pane_ids: Vec<PaneId> = tab.pane_tree.leaf_ids();

        if self.tab_manager.close_tab(tab_id) {
            for id in pane_ids {
                self.panes.remove(&id);
                self.active_watchers.remove(&id);
            }
        }
    }

    fn toggle_help(&mut self) {
        let active_pane_ids = self.tab_manager.active().pane_tree.leaf_ids();
        let help_pane_id = active_pane_ids
            .iter()
            .find(|id| self.panes.get(id).is_some_and(|p| matches!(p.view_type(), ViewType::Help)))
            .copied();

        if let Some(id) = help_pane_id {
            self.close_pane(id);
        } else {
            let focused = self.tab_manager.active().focused_pane;
            let prev_view = self.panes.get(&focused).map(|p| p.view_type().clone());
            if let Some(new_id) = self.tab_manager.split_pane(focused, SplitDirection::Vertical, ViewType::Help) {
                let global = self.dispatcher.global_shortcuts();
                let pane_sc = self.dispatcher.pane_shortcuts();
                let resource_sc = self.dispatcher.resource_shortcuts();
                let mut help = HelpPane::new(global, pane_sc, resource_sc);
                help.on_focus_change(prev_view.as_ref());
                self.panes.insert(new_id, Box::new(help));
                self.set_focus(new_id);
            }
        }
    }

    fn focus_next(&mut self) {
        let ids = self.tab_manager.active().pane_tree.leaf_ids();
        if ids.is_empty() {
            return;
        }
        let focused = self.tab_manager.active().focused_pane;
        let pos = ids.iter().position(|&id| id == focused).unwrap_or(0);
        let next = ids[(pos + 1) % ids.len()];
        self.set_focus(next);
    }

    fn focus_prev(&mut self) {
        let ids = self.tab_manager.active().pane_tree.leaf_ids();
        if ids.is_empty() {
            return;
        }
        let focused = self.tab_manager.active().focused_pane;
        let pos = ids.iter().position(|&id| id == focused).unwrap_or(0);
        let prev = ids[(pos + ids.len() - 1) % ids.len()];
        self.set_focus(prev);
    }

    fn set_focus(&mut self, new_id: PaneId) {
        let focused = self.tab_manager.active().focused_pane;
        let prev_view = self.panes.get(&focused).map(|p| p.view_type().clone());
        self.tab_manager.active_mut().focused_pane = new_id;
        if let Some(pane) = self.panes.get_mut(&new_id) {
            pane.on_focus_change(prev_view.as_ref());
        }
    }

    fn split_focused(&mut self, direction: SplitDirection) {
        let focused = self.tab_manager.active().focused_pane;
        let view = ViewType::Empty;
        if let Some(new_id) = self.tab_manager.split_pane(focused, direction, view.clone()) {
            self.panes.insert(new_id, Box::new(EmptyPane(view)));
            self.set_focus(new_id);
        }
    }

    fn close_focused(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        self.close_pane(focused);
    }

    fn close_pane(&mut self, target: PaneId) {
        let tab = self.tab_manager.active();
        let ids = tab.pane_tree.leaf_ids();
        if ids.len() <= 1 {
            return;
        }
        let focused = tab.focused_pane;
        let was_focused = target == focused;
        if self.tab_manager.active_mut().pane_tree.close(target) {
            self.panes.remove(&target);
            self.active_watchers.remove(&target);
            if let Some(ref mut fs) = self.tab_manager.active_mut().fullscreen_pane {
                if *fs == target {
                    self.tab_manager.active_mut().fullscreen_pane = None;
                }
            }
            if was_focused {
                let remaining = self.tab_manager.active().pane_tree.leaf_ids();
                if let Some(&first) = remaining.first() {
                    self.set_focus(first);
                }
            }
        }
    }

    fn focus_direction(&mut self, dir: Direction) {
        if self.tab_manager.active().fullscreen_pane.is_some() {
            return;
        }

        let area = ratatui::prelude::Rect::new(0, 0, 200, 50);
        let layout = self.tab_manager.active().pane_tree.layout(area);
        let focused = self.tab_manager.active().focused_pane;

        let current = layout.iter().find(|(id, _)| *id == focused).map(|(id, r)| (*id, *r));
        let Some(current) = current else { return };

        if let Some(target) = find_pane_in_direction(current, &layout, dir) {
            self.set_focus(target);
        }
    }

    fn toggle_fullscreen(&mut self) {
        let tab = self.tab_manager.active_mut();
        if tab.fullscreen_pane.is_some() {
            tab.fullscreen_pane = None;
        } else {
            tab.fullscreen_pane = Some(tab.focused_pane);
        }
    }

    fn switch_resource(&mut self, kind: ResourceKind) {
        let focused = self.tab_manager.active().focused_pane;
        let headers: Vec<String> = Vec::new();
        let new_pane = ResourceListPane::new(kind.clone(), headers);
        self.panes.insert(focused, Box::new(new_pane));

        let ns = if kind.is_namespaced() {
            self.context_resolver.namespace().unwrap_or("default").to_string()
        } else {
            String::new()
        };
        self.start_watcher_for_pane(focused, &kind, &ns);
    }

    fn handle_namespace_confirm(&mut self) {
        self.select_namespace();
        self.dispatcher.set_mode(InputMode::Normal);
    }

    fn handle_namespace_input(&mut self, c: char) {
        self.namespace_filter.push(c);
        self.namespace_selected = 0;
    }

    fn handle_namespace_backspace(&mut self) {
        self.namespace_filter.pop();
        self.namespace_selected = 0;
    }

    fn handle_namespace_nav(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::SelectPrev => {
                self.namespace_selected = self.namespace_selected.saturating_sub(1);
            }
            PaneCommand::SelectNext => {
                let count = self.filtered_namespaces().len();
                if self.namespace_selected + 1 < count {
                    self.namespace_selected += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_resource_update(&mut self, pane_id: PaneId, rows: Vec<Vec<String>>) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(resource_pane) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                resource_pane.state.set_items(rows);
                resource_pane.refresh_filter_and_sort();
            }
        }
    }

    fn handle_resource_error(&mut self, pane_id: PaneId, error: String) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(resource_pane) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                resource_pane.state.set_error(error);
            }
        }
    }

    fn with_pods_pane(&mut self, f: impl FnOnce(&mut ResourceListPane)) {
        if let Some(pane) = self.panes.get_mut(&self.pods_pane_id) {
            if let Some(resource_pane) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                f(resource_pane);
            }
        }
    }

    fn open_detail_pane(&mut self, kind: ResourceKind, name: String, namespace: String) {
        let sections = vec![DetailSection {
            title: "Metadata".into(),
            fields: vec![
                ("Name".into(), name.clone()),
                ("Namespace".into(), namespace.clone()),
                ("Kind".into(), kind.display_name().into()),
            ],
        }];

        let detail = ResourceDetailPane::new(kind.clone(), name.clone(), Some(namespace), sections);
        let focused = self.tab_manager.active().focused_pane;
        let view = ViewType::Detail(kind, name);
        if let Some(new_id) = self.tab_manager.split_pane(focused, SplitDirection::Horizontal, view) {
            self.panes.insert(new_id, Box::new(detail));
            self.set_focus(new_id);
        }
    }

    fn open_yaml_pane(&mut self, pane_id: PaneId, kind: ResourceKind, name: String, content: String) {
        let yaml_pane = YamlPane::new(kind.clone(), name.clone(), content);
        let view = ViewType::Yaml(kind, name);
        if let Some(new_id) = self.tab_manager.split_pane(pane_id, SplitDirection::Horizontal, view) {
            self.panes.insert(new_id, Box::new(yaml_pane));
            self.set_focus(new_id);
        }
    }

    fn open_logs_pane(&mut self) {
        let Some((kind, name, namespace)) = self.selected_resource_info() else {
            return;
        };
        if kind != ResourceKind::Pods {
            self.toasts.push(ToastMessage::info("Logs are only available for Pods"));
            return;
        }

        let focused = self.tab_manager.active().focused_pane;
        let pane = LogsPane::new(name.clone(), namespace.clone());
        let view = ViewType::Logs(name.clone());
        let Some(new_id) = self.tab_manager.split_pane(focused, SplitDirection::Horizontal, view) else {
            return;
        };
        self.panes.insert(new_id, Box::new(pane));
        self.set_focus(new_id);

        let Some(client) = &self.kube_client else {
            self.toasts.push(ToastMessage::error("No cluster connection"));
            return;
        };
        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();

        tokio::spawn(async move {
            let mut request = crystal_core::LogRequest {
                pod_name: name.clone(),
                namespace: namespace.clone(),
                container: None,
                follow: true,
                tail_lines: None,
                since_seconds: None,
                previous: false,
                timestamps: false,
            };

            let pods: Api<Pod> = Api::namespaced(kube_client.clone(), &namespace);
            let mut snapshot_params = kube::api::LogParams {
                follow: false,
                previous: request.previous,
                timestamps: false,
                tail_lines: None,
                container: request.container.clone(),
                ..Default::default()
            };
            let mut snapshot_result = pods.logs(&name, &snapshot_params).await;
            if let Err(err) = &snapshot_result {
                let msg = err.to_string();
                if msg.contains("container") && msg.contains("must be specified") {
                    let detected_container = detect_container_name(&pods, &name, &msg).await;
                    if let Some(container_name) = detected_container {
                        snapshot_params.container = Some(container_name.clone());
                        request.container = Some(container_name);
                        snapshot_result = pods.logs(&name, &snapshot_params).await;
                    }
                }
            }
            if let Ok(snapshot) = snapshot_result {
                let lines = snapshot.lines().map(ToString::to_string).collect::<Vec<_>>();
                let _ = app_tx.send(AppEvent::LogsSnapshotReady { pane_id: new_id, lines });
            } else if let Err(e) = snapshot_result {
                let _ =
                    app_tx.send(AppEvent::LogsStreamError { pane_id: new_id, error: format!("snapshot failed: {e}") });
                return;
            }

            let mut start_result = crystal_core::LogStream::start(&kube_client, request.clone()).await;

            // Multi-container pods can require an explicit container for logs.
            if let Err(err) = &start_result {
                let msg = err.to_string();
                if msg.contains("container") && msg.contains("must be specified") {
                    let pods: Api<Pod> = Api::namespaced(kube_client.clone(), &namespace);
                    let detected_container = detect_container_name(&pods, &name, &msg).await;
                    if let Some(container_name) = detected_container {
                        request.container = Some(container_name);
                        start_result = crystal_core::LogStream::start(&kube_client, request).await;
                    }
                }
            }

            match start_result {
                Ok(stream) => {
                    let _ = app_tx.send(AppEvent::LogsStreamReady { pane_id: new_id, stream });
                }
                Err(e) => {
                    let error = e.to_string();
                    let _ = app_tx.send(AppEvent::LogsStreamError { pane_id: new_id, error: error.clone() });
                    let _ = app_tx.send(AppEvent::Toast(ToastMessage::error(format!("Failed to start logs: {error}"))));
                }
            }
        });
    }

    fn attach_logs_stream(&mut self, pane_id: PaneId, stream: crystal_core::LogStream) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(logs_pane) = pane.as_any_mut().downcast_mut::<LogsPane>() {
                logs_pane.attach_stream(stream);
            }
        }
    }

    fn attach_logs_snapshot(&mut self, pane_id: PaneId, lines: Vec<String>) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(logs_pane) = pane.as_any_mut().downcast_mut::<LogsPane>() {
                logs_pane.append_snapshot(lines);
            }
        }
    }

    fn attach_logs_error(&mut self, pane_id: PaneId, error: String) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(logs_pane) = pane.as_any_mut().downcast_mut::<LogsPane>() {
                logs_pane.set_error(error);
            }
        }
    }

    fn poll_runtime_panes(&mut self) {
        for pane in self.panes.values_mut() {
            if let Some(logs_pane) = pane.as_any_mut().downcast_mut::<LogsPane>() {
                logs_pane.poll();
            }
        }
    }

    fn selected_resource_info(&self) -> Option<(ResourceKind, String, String)> {
        let focused = self.tab_manager.active().focused_pane;
        let pane = self.panes.get(&focused)?;
        let rp = pane.as_any().downcast_ref::<ResourceListPane>()?;

        let kind = rp.kind()?.clone();

        let selected_idx = match rp.state.selected {
            Some(s) => {
                if rp.filtered_indices.is_empty() {
                    s
                } else {
                    *rp.filtered_indices.get(s)?
                }
            }
            None => return None,
        };

        let row = rp.state.items.get(selected_idx)?;
        let name = row.first().cloned().unwrap_or_default();
        let namespace = if rp.state.headers.iter().any(|h| h == "NAMESPACE") {
            let ns_idx = rp.state.headers.iter().position(|h| h == "NAMESPACE").unwrap_or(1);
            row.get(ns_idx).cloned().unwrap_or_default()
        } else {
            self.context_resolver.namespace().unwrap_or("default").to_string()
        };

        Some((kind, name, namespace))
    }

    fn select_namespace(&mut self) {
        let filtered = self.filtered_namespaces();
        if let Some(ns) = filtered.get(self.namespace_selected).cloned() {
            let ns = if ns == "All Namespaces" { "default".to_string() } else { ns };

            // Cancel all active watchers
            self.active_watchers.drain();

            if let Some(ref mut client) = self.kube_client {
                client.set_namespace(&ns);
            }
            self.context_resolver.set_namespace(&ns);

            // Reset all resource list panes and restart their watchers
            let pane_ids: Vec<PaneId> = self.panes.keys().copied().collect();
            for pane_id in pane_ids {
                let kind = {
                    let Some(pane) = self.panes.get(&pane_id) else { continue };
                    let Some(rp) = pane.as_any().downcast_ref::<ResourceListPane>() else { continue };
                    rp.kind().cloned()
                };
                if let Some(kind) = kind {
                    if let Some(pane) = self.panes.get_mut(&pane_id) {
                        if let Some(rp) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                            let headers = rp.state.headers.clone();
                            rp.state = crate::state::ResourceListState::new(headers);
                            rp.filtered_indices.clear();
                        }
                    }
                    let watcher_ns = if kind.is_namespaced() { &ns } else { "" };
                    self.start_watcher_for_pane(pane_id, &kind, watcher_ns);
                }
            }
        }
    }

    fn filtered_namespaces(&self) -> Vec<String> {
        let filter_lower = self.namespace_filter.to_lowercase();
        let mut result = Vec::new();

        if filter_lower.is_empty() || "all namespaces".contains(&filter_lower) {
            result.push("All Namespaces".to_string());
        }

        for ns in &self.namespaces {
            if filter_lower.is_empty() || ns.to_lowercase().contains(&filter_lower) {
                result.push(ns.clone());
            }
        }

        result
    }

    fn initiate_delete(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let Some(pane) = self.panes.get(&focused) else { return };
        let Some(rp) = pane.as_any().downcast_ref::<ResourceListPane>() else { return };

        let kind = match rp.view_type() {
            ViewType::ResourceList(k) => k.clone(),
            _ => return,
        };

        let selected_idx = match rp.state.selected {
            Some(s) => {
                if rp.filtered_indices.is_empty() {
                    s
                } else {
                    match rp.filtered_indices.get(s) {
                        Some(&i) => i,
                        None => return,
                    }
                }
            }
            None => return,
        };

        let row = match rp.state.items.get(selected_idx) {
            Some(r) => r,
            None => return,
        };

        let name = row.first().cloned().unwrap_or_default();
        let namespace = if rp.state.headers.iter().any(|h| h == "NAMESPACE") {
            let ns_idx = rp.state.headers.iter().position(|h| h == "NAMESPACE").unwrap_or(1);
            row.get(ns_idx).cloned().unwrap_or_default()
        } else {
            self.context_resolver.namespace().unwrap_or("default").to_string()
        };

        let message = format!("Delete {} {}\nin namespace {}?", kind.display_name(), name, namespace);

        self.pending_confirmation =
            Some(PendingConfirmation { message, action: PendingAction::Delete { kind, name, namespace } });
        self.dispatcher.set_mode(InputMode::ConfirmDialog);
    }

    fn execute_confirmed_action(&mut self) {
        let confirmation = match self.pending_confirmation.take() {
            Some(c) => c,
            None => return,
        };
        self.dispatcher.set_mode(InputMode::Normal);

        match confirmation.action {
            PendingAction::Delete { kind, name, namespace } => {
                let Some(client) = &self.kube_client else {
                    self.toasts.push(ToastMessage::error("No cluster connection"));
                    return;
                };
                let kube_client = client.inner_client();
                let app_tx = self.app_tx.clone();
                let display_name = format!("{} {}", kind.short_name(), name);

                tokio::spawn(async move {
                    let executor = crystal_core::ActionExecutor::new(kube_client);
                    let result = match kind {
                        ResourceKind::Pods => {
                            executor.delete::<k8s_openapi::api::core::v1::Pod>(&name, &namespace).await
                        }
                        ResourceKind::Deployments => {
                            executor.delete::<k8s_openapi::api::apps::v1::Deployment>(&name, &namespace).await
                        }
                        ResourceKind::Services => {
                            executor.delete::<k8s_openapi::api::core::v1::Service>(&name, &namespace).await
                        }
                        ResourceKind::StatefulSets => {
                            executor.delete::<k8s_openapi::api::apps::v1::StatefulSet>(&name, &namespace).await
                        }
                        ResourceKind::DaemonSets => {
                            executor.delete::<k8s_openapi::api::apps::v1::DaemonSet>(&name, &namespace).await
                        }
                        ResourceKind::Jobs => {
                            executor.delete::<k8s_openapi::api::batch::v1::Job>(&name, &namespace).await
                        }
                        ResourceKind::CronJobs => {
                            executor.delete::<k8s_openapi::api::batch::v1::CronJob>(&name, &namespace).await
                        }
                        ResourceKind::ConfigMaps => {
                            executor.delete::<k8s_openapi::api::core::v1::ConfigMap>(&name, &namespace).await
                        }
                        ResourceKind::Secrets => {
                            executor.delete::<k8s_openapi::api::core::v1::Secret>(&name, &namespace).await
                        }
                        ResourceKind::Ingresses => {
                            executor.delete::<k8s_openapi::api::networking::v1::Ingress>(&name, &namespace).await
                        }
                        ResourceKind::PersistentVolumeClaims => {
                            executor
                                .delete::<k8s_openapi::api::core::v1::PersistentVolumeClaim>(&name, &namespace)
                                .await
                        }
                        _ => Err(anyhow::anyhow!("Delete not supported for this resource type")),
                    };

                    let toast_event = match result {
                        Ok(()) => AppEvent::Toast(ToastMessage::success(format!("Deleted {display_name}"))),
                        Err(e) => AppEvent::Toast(ToastMessage::error(format!("Failed to delete {display_name}: {e}"))),
                    };
                    let _ = app_tx.send(toast_event);
                });
            }
        }
    }

    fn mode_name(&self) -> &'static str {
        match self.dispatcher.mode() {
            InputMode::Normal => "Normal",
            InputMode::NamespaceSelector => "Namespace",
            InputMode::Pane => "Pane",
            InputMode::Tab => "Tab",
            InputMode::Search => "Search",
            InputMode::Command => "Command",
            InputMode::Insert => "Insert",
            InputMode::ResourceSwitcher => "Resource",
            InputMode::ConfirmDialog => "Confirm",
            InputMode::FilterInput => "Filter",
        }
    }

    fn mode_hints(&self) -> Vec<(String, String)> {
        match self.dispatcher.mode() {
            InputMode::Normal => self.dispatcher.global_hints(),
            InputMode::NamespaceSelector => {
                vec![
                    ("Up/Down".into(), "Navigate".into()),
                    ("Enter".into(), "Select".into()),
                    ("Esc".into(), "Cancel".into()),
                ]
            }
            InputMode::ResourceSwitcher => {
                vec![
                    ("Up/Down".into(), "Navigate".into()),
                    ("Enter".into(), "Select".into()),
                    ("Esc".into(), "Cancel".into()),
                ]
            }
            InputMode::ConfirmDialog => {
                vec![("y".into(), "Confirm".into()), ("n/Esc".into(), "Cancel".into())]
            }
            InputMode::FilterInput => {
                vec![("Enter".into(), "Keep filter".into()), ("Esc".into(), "Clear & exit".into())]
            }
            InputMode::Insert => {
                vec![("Esc".into(), "Normal mode".into())]
            }
            _ => vec![],
        }
    }

    fn build_render_context(&self) -> (RenderContext<'_>, Vec<String>, Vec<(String, String)>) {
        let namespace_selector = if self.dispatcher.mode() == InputMode::NamespaceSelector {
            Some(NamespaceSelectorView {
                namespaces: &self.namespaces,
                filter: &self.namespace_filter,
                selected: self.namespace_selected,
            })
        } else {
            None
        };

        let resource_switcher = self.resource_switcher.as_ref().map(|sw| ResourceSwitcherView {
            input: sw.input(),
            items: sw.filtered(),
            selected: sw.selected(),
        });

        let confirm_dialog = self.pending_confirmation.as_ref().map(|pc| ConfirmDialogView { message: &pc.message });

        let tab_names = self.tab_manager.tab_names();
        let hints = self.mode_hints();

        let tab = self.tab_manager.active();
        let (pane_tree, focused_pane, fullscreen_pane) = (&tab.pane_tree, tab.focused_pane, tab.fullscreen_pane);

        let ctx = RenderContext {
            cluster_name: self.context_resolver.context_name(),
            namespace: self.context_resolver.namespace(),
            namespace_selector,
            resource_switcher,
            confirm_dialog,
            toasts: &self.toasts,
            pane_tree,
            focused_pane: Some(focused_pane),
            fullscreen_pane,
            panes: &self.panes,
            tab_names: &[],
            active_tab: self.tab_manager.active_index(),
            mode_name: self.mode_name(),
            mode_hints: &[],
        };

        (ctx, tab_names, hints)
    }
}

async fn dispatch_get_yaml(
    executor: &crystal_core::ActionExecutor,
    kind: &ResourceKind,
    name: &str,
    ns: &str,
) -> anyhow::Result<String> {
    match kind {
        ResourceKind::Pods => executor.get_yaml::<Pod>(name, ns).await,
        ResourceKind::Deployments => executor.get_yaml::<Deployment>(name, ns).await,
        ResourceKind::Services => executor.get_yaml::<Service>(name, ns).await,
        ResourceKind::StatefulSets => executor.get_yaml::<StatefulSet>(name, ns).await,
        ResourceKind::DaemonSets => executor.get_yaml::<DaemonSet>(name, ns).await,
        ResourceKind::Jobs => executor.get_yaml::<Job>(name, ns).await,
        ResourceKind::CronJobs => executor.get_yaml::<CronJob>(name, ns).await,
        ResourceKind::ConfigMaps => executor.get_yaml::<ConfigMap>(name, ns).await,
        ResourceKind::Secrets => executor.get_yaml::<Secret>(name, ns).await,
        ResourceKind::Ingresses => executor.get_yaml::<Ingress>(name, ns).await,
        ResourceKind::PersistentVolumeClaims => executor.get_yaml::<PersistentVolumeClaim>(name, ns).await,
        ResourceKind::Nodes => executor.get_yaml_cluster::<Node>(name).await,
        ResourceKind::Namespaces => executor.get_yaml_cluster::<Namespace>(name).await,
        ResourceKind::PersistentVolumes => executor.get_yaml_cluster::<PersistentVolume>(name).await,
        ResourceKind::Custom(_) => Err(anyhow::anyhow!("YAML view not supported for custom resources")),
    }
}

async fn dispatch_describe(
    executor: &crystal_core::ActionExecutor,
    kind: &ResourceKind,
    name: &str,
    ns: &str,
) -> anyhow::Result<String> {
    match kind {
        ResourceKind::Pods => executor.describe::<Pod>(name, ns).await,
        ResourceKind::Deployments => executor.describe::<Deployment>(name, ns).await,
        ResourceKind::Services => executor.describe::<Service>(name, ns).await,
        ResourceKind::StatefulSets => executor.describe::<StatefulSet>(name, ns).await,
        ResourceKind::DaemonSets => executor.describe::<DaemonSet>(name, ns).await,
        ResourceKind::Jobs => executor.describe::<Job>(name, ns).await,
        ResourceKind::CronJobs => executor.describe::<CronJob>(name, ns).await,
        ResourceKind::ConfigMaps => executor.describe::<ConfigMap>(name, ns).await,
        ResourceKind::Secrets => executor.describe::<Secret>(name, ns).await,
        ResourceKind::Ingresses => executor.describe::<Ingress>(name, ns).await,
        ResourceKind::PersistentVolumeClaims => executor.describe::<PersistentVolumeClaim>(name, ns).await,
        _ => Err(anyhow::anyhow!("Describe not supported for this resource type")),
    }
}

async fn detect_container_name(pods: &Api<Pod>, pod_name: &str, error_msg: &str) -> Option<String> {
    if let Some(name) = first_container_from_logs_error(error_msg) {
        return Some(name);
    }
    pods.get(pod_name)
        .await
        .ok()
        .and_then(|pod| pod.spec.as_ref().and_then(|s| s.containers.first()).map(|c| c.name.clone()))
}

fn first_container_from_logs_error(error_msg: &str) -> Option<String> {
    let marker = "choose one of:";
    let (_, right) = error_msg.split_once(marker)?;
    let candidates = if let (Some(start), Some(end_rel)) = (right.find('['), right.find(']')) {
        &right[start + 1..end_rel]
    } else {
        right
    };
    candidates
        .split([',', ' ', '\n', '\t'])
        .map(str::trim)
        .find(|s| !s.is_empty() && *s != "or")
        .map(|s| s.trim_matches('"').to_string())
}

struct EmptyPane(ViewType);

impl Pane for EmptyPane {
    fn render(&self, frame: &mut ratatui::prelude::Frame, area: ratatui::prelude::Rect, focused: bool) {
        use ratatui::prelude::*;
        use ratatui::widgets::{Block, Borders, Paragraph};

        let border_color = if focused { crystal_tui::theme::ACCENT } else { crystal_tui::theme::BORDER_COLOR };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Empty ")
            .title_style(Style::default().fg(crystal_tui::theme::TEXT_DIM));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let msg = Paragraph::new("Empty pane").style(Style::default().fg(crystal_tui::theme::TEXT_DIM));
        frame.render_widget(msg, inner);
    }

    fn handle_command(&mut self, _cmd: &crystal_tui::pane::PaneCommand) {}

    fn view_type(&self) -> &ViewType {
        &self.0
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests;
