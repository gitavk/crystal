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
use crystal_tui::layout::{
    ConfirmDialogView, ContextSelectorView, NamespaceSelectorView, PortForwardDialogView, RenderContext,
    ResourceSwitcherView,
};
use crystal_tui::pane::{
    find_pane_in_direction, Direction, Pane, PaneCommand, PaneId, ResourceKind, SplitDirection, ViewType,
};
use crystal_tui::tab::TabManager;
use crystal_tui::widgets::toast::ToastMessage;

use crate::command::{Command, InputMode};
use crate::event::{AppEvent, EventHandler};
use crate::keybindings::KeybindingDispatcher;
use crate::panes::{ExecPane, HelpPane, LogsPane, ResourceDetailPane, ResourceListPane, YamlPane};
use crate::resource_switcher::ResourceSwitcher;

#[derive(Debug, Clone)]
pub enum PendingAction {
    Delete { kind: ResourceKind, name: String, namespace: String },
}

pub struct PendingConfirmation {
    pub message: String,
    pub action: PendingAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PortForwardField {
    Local,
    Remote,
}

impl PortForwardField {
    fn toggle(self) -> Self {
        match self {
            Self::Local => Self::Remote,
            Self::Remote => Self::Local,
        }
    }
}

struct PendingPortForward {
    pod: String,
    namespace: String,
    local_input: String,
    remote_input: String,
    active_field: PortForwardField,
}

#[derive(Clone)]
struct TabScope {
    kube_client: Option<KubeClient>,
    context_resolver: ContextResolver,
    contexts: Vec<String>,
    namespaces: Vec<String>,
    namespace_filter: String,
    namespace_selected: usize,
    context_filter: String,
    context_selected: usize,
}

pub struct App {
    running: bool,
    tick_rate: Duration,
    kube_client: Option<KubeClient>,
    context_resolver: ContextResolver,
    dispatcher: KeybindingDispatcher,
    contexts: Vec<String>,
    namespaces: Vec<String>,
    namespace_filter: String,
    namespace_selected: usize,
    context_filter: String,
    context_selected: usize,
    tab_scopes: HashMap<u32, TabScope>,
    /// Active watchers keyed by pane ID.
    /// Each pane showing a resource list has its own watcher.
    /// When a pane switches resource type, its watcher is cancelled and a new one spawned.
    /// Dropping a ResourceWatcher cancels its background task automatically.
    active_watchers: HashMap<PaneId, ResourceWatcher>,
    active_forwards: HashMap<ForwardId, crystal_core::PortForward>,
    pod_forward_index: HashMap<(String, String), ForwardId>,
    filter_input_buffer: String,
    resource_switcher: Option<ResourceSwitcher>,
    pending_confirmation: Option<PendingConfirmation>,
    pending_port_forward: Option<PendingPortForward>,
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
        let contexts = KubeClient::list_contexts().unwrap_or_default();

        let pod_headers = vec![
            "PF".into(),
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

        let mut app = Self {
            running: true,
            tick_rate: Duration::from_millis(tick_rate_ms),
            kube_client,
            context_resolver,
            dispatcher,
            contexts,
            namespaces: Vec::new(),
            namespace_filter: String::new(),
            namespace_selected: 0,
            context_filter: String::new(),
            context_selected: 0,
            tab_scopes: HashMap::new(),
            active_watchers: HashMap::new(),
            active_forwards: HashMap::new(),
            pod_forward_index: HashMap::new(),
            filter_input_buffer: String::new(),
            resource_switcher: None,
            pending_confirmation: None,
            pending_port_forward: None,
            toasts: Vec::new(),
            tab_manager,
            panes,
            pods_pane_id,
            app_tx: tx,
        };
        app.sync_active_scope();
        app.update_active_tab_title();
        app
    }

    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        let mut events = EventHandler::new(self.tick_rate);
        self.app_tx = events.app_tx();

        if let Some(client) = &self.kube_client {
            let ns = client.namespace().to_string();
            self.start_watcher_for_pane(self.pods_pane_id, &ResourceKind::Pods, &ns);

            if let Some(client) = &self.kube_client {
                match client.list_namespaces().await {
                    Ok(ns_list) => {
                        self.namespaces = ns_list;
                        self.sync_active_scope();
                    }
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
                AppEvent::ResourceUpdate { pane_id, headers, rows } => {
                    self.handle_resource_update(pane_id, headers, rows)
                }
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
                AppEvent::ExecSessionReady { pane_id, session } => {
                    self.attach_exec_session(pane_id, session);
                    self.dispatcher.set_mode(InputMode::Insert);
                }
                AppEvent::ExecSessionError { pane_id, error } => {
                    self.attach_exec_error(pane_id, error);
                    self.dispatcher.set_mode(InputMode::Normal);
                }
                AppEvent::PortForwardReady { forward } => {
                    self.attach_port_forward(forward);
                }
                AppEvent::PortForwardPromptReady { pod, namespace, suggested_remote } => {
                    self.open_port_forward_prompt(pod, namespace, suggested_remote);
                }
                AppEvent::ContextSwitchReady { client, namespaces } => {
                    self.apply_context_switch(client, namespaces);
                }
                AppEvent::ContextSwitchError { context, error } => {
                    self.toasts.push(ToastMessage::error(format!("Failed to switch context {context}: {error}")));
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
            Command::Quit => {
                self.stop_all_port_forwards();
                self.running = false;
            }
            Command::ShowHelp => self.toggle_help(),
            Command::FocusNextPane => self.focus_next(),
            Command::FocusPrevPane => self.focus_prev(),
            Command::SplitVertical => self.split_focused(SplitDirection::Vertical),
            Command::SplitHorizontal => self.split_focused(SplitDirection::Horizontal),
            Command::ClosePane => self.close_focused(),
            Command::EnterMode(mode) => {
                if mode == InputMode::Insert && !self.focused_supports_insert_mode() {
                    return;
                }
                self.dispatcher.set_mode(mode);
                if mode == InputMode::NamespaceSelector {
                    self.namespace_filter.clear();
                    self.namespace_selected = 0;
                }
                if mode == InputMode::ContextSelector {
                    self.context_filter.clear();
                    self.context_selected = 0;
                    self.contexts = KubeClient::list_contexts().unwrap_or_default();
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
            Command::ContextConfirm => self.handle_context_confirm(),
            Command::ContextInput(c) => self.handle_context_input(c),
            Command::ContextBackspace => self.handle_context_backspace(),
            Command::FocusDirection(dir) => self.focus_direction(dir),
            Command::NewTab => self.new_tab(),
            Command::CloseTab => self.close_tab(),
            Command::NextTab => self.switch_to_next_tab(),
            Command::PrevTab => self.switch_to_prev_tab(),
            Command::GoToTab(n) => {
                if n > 0 {
                    self.switch_to_tab_index(n - 1);
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
            Command::Pane(ref pane_cmd) if self.dispatcher.mode() == InputMode::ContextSelector => {
                self.handle_context_nav(pane_cmd);
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
            Command::PortForwardInput(c) => {
                if let Some(ref mut pending) = self.pending_port_forward {
                    let target = match pending.active_field {
                        PortForwardField::Local => &mut pending.local_input,
                        PortForwardField::Remote => &mut pending.remote_input,
                    };
                    if target == "0" {
                        target.clear();
                    }
                    target.push(c);
                }
            }
            Command::PortForwardBackspace => {
                if let Some(ref mut pending) = self.pending_port_forward {
                    let target = match pending.active_field {
                        PortForwardField::Local => &mut pending.local_input,
                        PortForwardField::Remote => &mut pending.remote_input,
                    };
                    target.pop();
                }
            }
            Command::PortForwardToggleField => {
                if let Some(ref mut pending) = self.pending_port_forward {
                    pending.active_field = pending.active_field.toggle();
                }
            }
            Command::PortForwardConfirm => {
                self.confirm_port_forward();
            }
            Command::PortForwardCancel => {
                self.pending_port_forward = None;
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
                self.update_active_tab_title();
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
                self.pending_port_forward = None;
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
                self.open_exec_pane();
            }
            Command::PortForward => {
                self.toggle_port_forward_for_selected();
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
        self.sync_active_scope();
        let tab_count = self.tab_manager.tabs().len();
        let name = format!("Tab {}", tab_count + 1);
        let pod_headers = vec![
            "PF".into(),
            "NAME".into(),
            "NAMESPACE".into(),
            "STATUS".into(),
            "READY".into(),
            "RESTARTS".into(),
            "AGE".into(),
            "NODE".into(),
        ];
        let tab_id = self.tab_manager.new_tab(&name, ViewType::ResourceList(ResourceKind::Pods));
        let pane_id = self.tab_manager.tabs().iter().find(|t| t.id == tab_id).unwrap().focused_pane;
        self.panes.insert(pane_id, Box::new(ResourceListPane::new(ResourceKind::Pods, pod_headers)));
        let ns = self.context_resolver.namespace().unwrap_or("default").to_string();
        self.start_watcher_for_pane(pane_id, &ResourceKind::Pods, &ns);
        self.sync_active_scope();
        self.update_active_tab_title();
    }

    fn close_tab(&mut self) {
        self.sync_active_scope();
        let tab = self.tab_manager.active();
        let tab_id = tab.id;
        let pane_ids: Vec<PaneId> = tab.pane_tree.leaf_ids();

        if self.tab_manager.close_tab(tab_id) {
            self.tab_scopes.remove(&tab_id);
            for id in pane_ids {
                self.panes.remove(&id);
                self.active_watchers.remove(&id);
            }
            self.load_active_scope();
            self.update_active_tab_title();
        }
    }

    fn switch_to_tab_index(&mut self, index: usize) {
        self.sync_active_scope();
        self.tab_manager.switch_tab(index);
        self.load_active_scope();
        self.update_active_tab_title();
    }

    fn switch_to_next_tab(&mut self) {
        self.sync_active_scope();
        self.tab_manager.next_tab();
        self.load_active_scope();
        self.update_active_tab_title();
    }

    fn switch_to_prev_tab(&mut self) {
        self.sync_active_scope();
        self.tab_manager.prev_tab();
        self.load_active_scope();
        self.update_active_tab_title();
    }

    fn sync_active_scope(&mut self) {
        let tab_id = self.tab_manager.active().id;
        self.tab_scopes.insert(
            tab_id,
            TabScope {
                kube_client: self.kube_client.clone(),
                context_resolver: self.context_resolver.clone(),
                contexts: self.contexts.clone(),
                namespaces: self.namespaces.clone(),
                namespace_filter: self.namespace_filter.clone(),
                namespace_selected: self.namespace_selected,
                context_filter: self.context_filter.clone(),
                context_selected: self.context_selected,
            },
        );
    }

    fn load_active_scope(&mut self) {
        let tab_id = self.tab_manager.active().id;
        if let Some(scope) = self.tab_scopes.get(&tab_id).cloned() {
            self.kube_client = scope.kube_client;
            self.context_resolver = scope.context_resolver;
            self.contexts = scope.contexts;
            self.namespaces = scope.namespaces;
            self.namespace_filter = scope.namespace_filter;
            self.namespace_selected = scope.namespace_selected;
            self.context_filter = scope.context_filter;
            self.context_selected = scope.context_selected;
        } else {
            self.sync_active_scope();
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
        self.update_active_tab_title();
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
        self.update_active_tab_title();
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

    fn handle_context_confirm(&mut self) {
        self.select_context();
    }

    fn handle_context_input(&mut self, c: char) {
        self.context_filter.push(c);
        self.context_selected = 0;
    }

    fn handle_context_backspace(&mut self) {
        self.context_filter.pop();
        self.context_selected = 0;
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

    fn handle_context_nav(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::SelectPrev => {
                self.context_selected = self.context_selected.saturating_sub(1);
            }
            PaneCommand::SelectNext => {
                let count = self.filtered_contexts().len();
                if self.context_selected + 1 < count {
                    self.context_selected += 1;
                }
            }
            _ => {}
        }
    }

    fn handle_resource_update(&mut self, pane_id: PaneId, headers: Vec<String>, rows: Vec<Vec<String>>) {
        let pod_forward_index = self.pod_forward_index.clone();
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(resource_pane) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                let mut effective_headers = headers;
                let mut effective_rows = rows;

                if matches!(resource_pane.kind(), Some(ResourceKind::Pods)) {
                    effective_headers = with_pod_forward_header(&effective_headers);
                    effective_rows = effective_rows
                        .into_iter()
                        .map(|row| with_pod_forward_cell(row, &pod_forward_index, &effective_headers))
                        .collect();
                }

                if !effective_headers.is_empty() {
                    resource_pane.state.headers = effective_headers;
                }
                resource_pane.state.set_items(effective_rows);
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

    fn open_exec_pane(&mut self) {
        let Some((kind, name, namespace)) = self.selected_resource_info() else {
            return;
        };
        if kind != ResourceKind::Pods {
            self.toasts.push(ToastMessage::info("Exec is only available for Pods"));
            return;
        }
        let kube_client = match &self.kube_client {
            Some(client) => client.inner_client(),
            None => {
                self.toasts.push(ToastMessage::error("No cluster connection"));
                return;
            }
        };

        let focused = self.tab_manager.active().focused_pane;
        let pane = ExecPane::new(name.clone(), "auto".into(), namespace.clone());
        let view = ViewType::Exec(name.clone());
        let Some(new_id) = self.tab_manager.split_pane(focused, SplitDirection::Horizontal, view) else {
            return;
        };
        self.panes.insert(new_id, Box::new(pane));
        self.set_focus(new_id);

        let app_tx = self.app_tx.clone();
        tokio::spawn(async move {
            // Kubernetes exec with TTY already provides interactive mode.
            // Run shell fallback chain similar to: zsh || bash || sh, but without noisy errors.
            let shell_fallback = r#"if command -v zsh >/dev/null 2>&1; then exec zsh -i; fi; if command -v bash >/dev/null 2>&1; then exec bash -i; fi; exec sh -i"#;
            let shell_candidates: [Vec<String>; 2] = [
                vec!["/bin/sh".to_string(), "-c".to_string(), shell_fallback.to_string()],
                vec!["sh".to_string(), "-c".to_string(), shell_fallback.to_string()],
            ];

            let mut last_error: Option<anyhow::Error> = None;
            let mut started = None;

            for command in shell_candidates {
                match crystal_core::ExecSession::start(kube_client.clone(), &name, &namespace, None, command).await {
                    Ok(session) => {
                        started = Some(session);
                        break;
                    }
                    Err(err) => {
                        last_error = Some(err);
                    }
                }
            }

            match started {
                Some(session) => {
                    let _ = app_tx.send(AppEvent::ExecSessionReady { pane_id: new_id, session });
                }
                None => {
                    let error = match last_error {
                        Some(err) => format!("no supported shell found (zsh, bash, sh): {err}"),
                        None => "no supported shell found (zsh, bash, sh)".to_string(),
                    };
                    let _ = app_tx.send(AppEvent::ExecSessionError { pane_id: new_id, error: error.clone() });
                    let _ = app_tx.send(AppEvent::Toast(ToastMessage::error(format!("Failed to start exec: {error}"))));
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

    fn attach_exec_session(&mut self, pane_id: PaneId, session: crystal_core::ExecSession) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(exec_pane) = pane.as_any_mut().downcast_mut::<ExecPane>() {
                exec_pane.attach_session(session);
            }
        }
    }

    fn attach_exec_error(&mut self, pane_id: PaneId, error: String) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(exec_pane) = pane.as_any_mut().downcast_mut::<ExecPane>() {
                exec_pane.set_error(error);
            }
        }
    }

    fn poll_runtime_panes(&mut self) {
        let mut exited_exec_panes: Vec<PaneId> = Vec::new();
        let focused_before = self.tab_manager.active().focused_pane;

        for (pane_id, pane) in self.panes.iter_mut() {
            if let Some(logs_pane) = pane.as_any_mut().downcast_mut::<LogsPane>() {
                logs_pane.poll();
            }
            if let Some(exec_pane) = pane.as_any_mut().downcast_mut::<ExecPane>() {
                exec_pane.poll();
                if exec_pane.exited() {
                    exited_exec_panes.push(*pane_id);
                }
            }
        }

        let focused_exec_exited = exited_exec_panes.contains(&focused_before);
        for pane_id in exited_exec_panes {
            self.close_pane(pane_id);
        }

        if focused_exec_exited && self.dispatcher.mode() == InputMode::Insert {
            self.dispatcher.set_mode(InputMode::Normal);
        }
    }

    fn toggle_port_forward_for_selected(&mut self) {
        let Some((kind, pod, namespace)) = self.selected_resource_info() else {
            return;
        };
        if kind != ResourceKind::Pods {
            self.toasts.push(ToastMessage::info("Port forward is only available for Pods"));
            return;
        }

        let key = (namespace.clone(), pod.clone());
        if let Some(forward_id) = self.pod_forward_index.remove(&key) {
            if let Some(forward) = self.active_forwards.remove(&forward_id) {
                let app_tx = self.app_tx.clone();
                let pod_name = pod.clone();
                tokio::spawn(async move {
                    let _ = forward.stop().await;
                    let _ = app_tx
                        .send(AppEvent::Toast(ToastMessage::success(format!("Stopped port-forward for {pod_name}"))));
                });
                self.refresh_pod_forward_indicators();
                return;
            }
        }

        let Some(client) = &self.kube_client else {
            self.toasts.push(ToastMessage::error("No cluster connection"));
            return;
        };
        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();

        tokio::spawn(async move {
            let suggested_remote = detect_remote_port(&kube_client, &pod, &namespace).await.unwrap_or(80);
            let _ = app_tx.send(AppEvent::PortForwardPromptReady { pod, namespace, suggested_remote });
        });
    }

    fn open_port_forward_prompt(&mut self, pod: String, namespace: String, suggested_remote: u16) {
        self.pending_port_forward = Some(PendingPortForward {
            pod,
            namespace,
            local_input: "0".into(),
            remote_input: suggested_remote.to_string(),
            active_field: PortForwardField::Local,
        });
        self.dispatcher.set_mode(InputMode::PortForwardInput);
    }

    fn confirm_port_forward(&mut self) {
        let Some(pending) = self.pending_port_forward.take() else {
            return;
        };

        let local_input = pending.local_input.trim();
        let remote_input = pending.remote_input.trim();

        let local_port = if local_input.is_empty() {
            0
        } else {
            match local_input.parse::<u16>() {
                Ok(port) => port,
                Err(_) => {
                    self.toasts.push(ToastMessage::error("Local port must be 0-65535"));
                    self.pending_port_forward = Some(pending);
                    return;
                }
            }
        };

        let remote_port = match remote_input.parse::<u16>() {
            Ok(0) | Err(_) => {
                self.toasts.push(ToastMessage::error("Remote port must be 1-65535"));
                self.pending_port_forward = Some(pending);
                return;
            }
            Ok(port) => port,
        };

        let pod = pending.pod;
        let namespace = pending.namespace;
        self.dispatcher.set_mode(InputMode::Normal);

        let Some(client) = &self.kube_client else {
            self.toasts.push(ToastMessage::error("No cluster connection"));
            return;
        };
        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();

        tokio::spawn(async move {
            match crystal_core::PortForward::start(&kube_client, &pod, &namespace, local_port, remote_port).await {
                Ok(forward) => {
                    let _ = app_tx.send(AppEvent::PortForwardReady { forward });
                }
                Err(e) => {
                    let _ = app_tx
                        .send(AppEvent::Toast(ToastMessage::error(format!("Port-forward failed for {pod}: {e}"))));
                }
            }
        });
    }

    fn attach_port_forward(&mut self, forward: crystal_core::PortForward) {
        let pod = forward.pod_name().to_string();
        let ns = forward.namespace().to_string();
        let remote = forward.remote_port();
        let local = forward.local_port();
        let id = forward.id();
        self.pod_forward_index.insert((ns, pod.clone()), id);
        self.active_forwards.insert(id, forward);
        self.refresh_pod_forward_indicators();
        self.toasts.push(ToastMessage::success(format!("Forwarding {pod}:{remote} -> 127.0.0.1:{local}")));
    }

    fn stop_all_port_forwards(&mut self) {
        let forwards: Vec<crystal_core::PortForward> = self.active_forwards.drain().map(|(_, f)| f).collect();
        self.pod_forward_index.clear();
        self.refresh_pod_forward_indicators();
        for forward in forwards {
            tokio::spawn(async move {
                let _ = forward.stop().await;
            });
        }
    }

    fn refresh_pod_forward_indicators(&mut self) {
        let pod_forward_index = self.pod_forward_index.clone();
        if let Some(pane) = self.panes.get_mut(&self.pods_pane_id) {
            if let Some(rp) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                if rp.state.headers.is_empty() {
                    return;
                }
                let headers = with_pod_forward_header(&rp.state.headers);
                rp.state.headers = headers.clone();
                for row in &mut rp.state.items {
                    let updated = with_pod_forward_cell(std::mem::take(row), &pod_forward_index, &headers);
                    *row = updated;
                }
                rp.refresh_filter_and_sort();
            }
        }
    }

    fn focused_supports_insert_mode(&self) -> bool {
        let focused = self.tab_manager.active().focused_pane;
        self.panes.get(&focused).is_some_and(|pane| matches!(pane.view_type(), ViewType::Exec(_) | ViewType::Terminal))
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
        let name = header_value(&rp.state.headers, row, "NAME", 0).unwrap_or_default();
        let namespace = header_value(&rp.state.headers, row, "NAMESPACE", 1)
            .unwrap_or_else(|| self.context_resolver.namespace().unwrap_or("default").to_string());

        Some((kind, name, namespace))
    }

    fn select_namespace(&mut self) {
        let filtered = self.filtered_namespaces();
        if let Some(ns) = filtered.get(self.namespace_selected).cloned() {
            let ns = if ns == "All Namespaces" { "default".to_string() } else { ns };

            if let Some(ref mut client) = self.kube_client {
                client.set_namespace(&ns);
            }
            self.context_resolver.set_namespace(&ns);
            self.restart_watchers_for_active_panes();
            self.sync_active_scope();
            self.update_active_tab_title();
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

    fn filtered_contexts(&self) -> Vec<String> {
        let filter_lower = self.context_filter.to_lowercase();
        self.contexts
            .iter()
            .filter(|ctx| filter_lower.is_empty() || ctx.to_lowercase().contains(&filter_lower))
            .cloned()
            .collect()
    }

    fn select_context(&mut self) {
        let filtered = self.filtered_contexts();
        let Some(context) = filtered.get(self.context_selected).cloned() else {
            self.dispatcher.set_mode(InputMode::Normal);
            return;
        };
        if self.context_resolver.context_name() == Some(context.as_str()) {
            self.dispatcher.set_mode(InputMode::Normal);
            self.sync_active_scope();
            return;
        }

        let app_tx = self.app_tx.clone();
        tokio::spawn(async move {
            match KubeClient::from_context(&context).await {
                Ok(client) => {
                    let namespaces = client.list_namespaces().await.unwrap_or_default();
                    let _ = app_tx.send(AppEvent::ContextSwitchReady { client, namespaces });
                }
                Err(e) => {
                    let _ =
                        app_tx.send(AppEvent::ContextSwitchError { context: context.clone(), error: e.to_string() });
                }
            }
        });
        self.dispatcher.set_mode(InputMode::Normal);
    }

    fn apply_context_switch(&mut self, client: KubeClient, namespaces: Vec<String>) {
        self.stop_all_port_forwards();
        self.context_resolver.set_context(client.cluster_context());
        self.kube_client = Some(client);
        self.namespaces = namespaces;
        self.namespace_filter.clear();
        self.namespace_selected = 0;
        self.restart_watchers_for_active_panes();
        self.sync_active_scope();
        self.update_active_tab_title();
    }

    fn restart_watchers_for_active_panes(&mut self) {
        let pane_ids: Vec<PaneId> = self.tab_manager.active().pane_tree.leaf_ids();
        for pane_id in &pane_ids {
            self.active_watchers.remove(pane_id);
        }
        for pane_id in pane_ids {
            let (kind, all_namespaces, headers) = {
                let Some(pane) = self.panes.get(&pane_id) else { continue };
                let Some(rp) = pane.as_any().downcast_ref::<ResourceListPane>() else { continue };
                (rp.kind().cloned(), rp.all_namespaces, rp.state.headers.clone())
            };
            let Some(kind) = kind else { continue };

            if let Some(pane) = self.panes.get_mut(&pane_id) {
                if let Some(rp) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                    rp.state = crate::state::ResourceListState::new(headers);
                    rp.filtered_indices.clear();
                }
            }

            let ns = if kind.is_namespaced() {
                if all_namespaces {
                    String::new()
                } else {
                    self.context_resolver.namespace().unwrap_or("default").to_string()
                }
            } else {
                String::new()
            };
            self.start_watcher_for_pane(pane_id, &kind, &ns);
        }
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

        let name = header_value(&rp.state.headers, row, "NAME", 0).unwrap_or_default();
        let namespace = header_value(&rp.state.headers, row, "NAMESPACE", 1)
            .unwrap_or_else(|| self.context_resolver.namespace().unwrap_or("default").to_string());

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
            InputMode::ContextSelector => "Context",
            InputMode::Pane => "Pane",
            InputMode::Tab => "Tab",
            InputMode::Search => "Search",
            InputMode::Command => "Command",
            InputMode::Insert => "Insert",
            InputMode::ResourceSwitcher => "Resource",
            InputMode::ConfirmDialog => "Confirm",
            InputMode::FilterInput => "Filter",
            InputMode::PortForwardInput => "PortForward",
        }
    }

    fn mode_hints(&self) -> Vec<(String, String)> {
        let mut hints = match self.dispatcher.mode() {
            InputMode::Normal => self.dispatcher.global_hints(),
            InputMode::NamespaceSelector => {
                vec![
                    ("Up/Down".into(), "Navigate".into()),
                    ("Enter".into(), "Select".into()),
                    ("Esc".into(), "Cancel".into()),
                ]
            }
            InputMode::ContextSelector => {
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
            InputMode::PortForwardInput => {
                vec![
                    ("Tab".into(), "Switch field".into()),
                    ("Enter".into(), "Start".into()),
                    ("Esc".into(), "Cancel".into()),
                ]
            }
            InputMode::Insert => {
                vec![("Esc".into(), "Normal mode".into())]
            }
            _ => vec![],
        };

        if let Some((_pod, local, remote)) = self.selected_pod_forward_mapping() {
            hints.push(("PF".into(), format!("127.0.0.1:{local}->{remote}")));
        }

        hints
    }

    fn selected_pod_forward_mapping(&self) -> Option<(String, u16, u16)> {
        let (kind, pod, namespace) = self.selected_resource_info()?;
        if kind != ResourceKind::Pods {
            return None;
        }
        let forward_id = self.pod_forward_index.get(&(namespace, pod.clone()))?;
        let forward = self.active_forwards.get(forward_id)?;
        Some((pod, forward.local_port(), forward.remote_port()))
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
        let context_selector = if self.dispatcher.mode() == InputMode::ContextSelector {
            Some(ContextSelectorView {
                contexts: &self.contexts,
                filter: &self.context_filter,
                selected: self.context_selected,
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
        let port_forward_dialog = self.pending_port_forward.as_ref().map(|pf| PortForwardDialogView {
            pod: &pf.pod,
            namespace: &pf.namespace,
            local_port: &pf.local_input,
            remote_port: &pf.remote_input,
            active_field: match pf.active_field {
                PortForwardField::Local => crystal_tui::layout::PortForwardFieldView::Local,
                PortForwardField::Remote => crystal_tui::layout::PortForwardFieldView::Remote,
            },
        });

        let tab_names = self.tab_manager.tab_names();
        let hints = self.mode_hints();

        let tab = self.tab_manager.active();
        let (pane_tree, focused_pane, fullscreen_pane) = (&tab.pane_tree, tab.focused_pane, tab.fullscreen_pane);

        let ctx = RenderContext {
            cluster_name: self.context_resolver.context_name(),
            namespace: self.context_resolver.namespace(),
            namespace_selector,
            context_selector,
            resource_switcher,
            confirm_dialog,
            port_forward_dialog,
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

    fn update_active_tab_title(&mut self) {
        let tab_id = self.tab_manager.active().id;
        let ctx = self.context_resolver.context_name().unwrap_or("noctx");
        let ns = self.active_namespace_label();
        let alias = self.active_view_alias();
        let title = format!("{ctx}:{ns}|{alias}");
        self.tab_manager.rename_tab(tab_id, &title);
    }

    fn active_namespace_label(&self) -> String {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get(&focused) {
            if let Some(rp) = pane.as_any().downcast_ref::<ResourceListPane>() {
                if rp.all_namespaces {
                    return "*".into();
                }
            }
        }
        self.context_resolver.namespace().unwrap_or("n/a").to_string()
    }

    fn active_view_alias(&self) -> String {
        let focused = self.tab_manager.active().focused_pane;
        let Some(pane) = self.panes.get(&focused) else { return "UNK".into() };
        match pane.view_type() {
            ViewType::ResourceList(kind) => resource_alias(kind),
            ViewType::Detail(kind, _) => resource_alias(kind),
            ViewType::Yaml(kind, _) => resource_alias(kind),
            ViewType::Logs(_) => "LOG".into(),
            ViewType::Exec(_) => "EXE".into(),
            ViewType::Terminal => "TER".into(),
            ViewType::Help => "HLP".into(),
            ViewType::Empty => "EMP".into(),
            ViewType::Plugin(_) => "PLG".into(),
        }
    }
}

fn resource_alias(kind: &ResourceKind) -> String {
    match kind {
        ResourceKind::Pods => "POD".into(),
        ResourceKind::Deployments => "DEP".into(),
        ResourceKind::Services => "SVC".into(),
        ResourceKind::StatefulSets => "STS".into(),
        ResourceKind::DaemonSets => "DMS".into(),
        ResourceKind::Jobs => "JOB".into(),
        ResourceKind::CronJobs => "CRN".into(),
        ResourceKind::ConfigMaps => "CFG".into(),
        ResourceKind::Secrets => "SEC".into(),
        ResourceKind::Ingresses => "ING".into(),
        ResourceKind::Nodes => "NOD".into(),
        ResourceKind::Namespaces => "NSP".into(),
        ResourceKind::PersistentVolumes => "PVS".into(),
        ResourceKind::PersistentVolumeClaims => "PVC".into(),
        ResourceKind::Custom(name) => {
            let up = name.to_uppercase();
            up.chars().take(3).collect()
        }
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

fn with_pod_forward_header(headers: &[String]) -> Vec<String> {
    if headers.iter().any(|h| h == "PF") {
        return headers.to_vec();
    }
    let mut updated = Vec::with_capacity(headers.len() + 1);
    updated.push("PF".to_string());
    updated.extend(headers.iter().cloned());
    updated
}

fn with_pod_forward_cell(
    mut row: Vec<String>,
    pod_forward_index: &HashMap<(String, String), ForwardId>,
    headers: &[String],
) -> Vec<String> {
    let name = header_value(headers, &row, "NAME", 0).unwrap_or_default();
    let namespace = header_value(headers, &row, "NAMESPACE", 1).unwrap_or_default();
    let active = pod_forward_index.contains_key(&(namespace, name));
    set_pod_forward_cell(&mut row, active);
    row
}

fn set_pod_forward_cell(row: &mut Vec<String>, active: bool) {
    if row.is_empty() {
        row.push(if active { "ON".into() } else { String::new() });
        return;
    }
    if row[0] == "ON" || row[0].is_empty() {
        row[0] = if active { "ON".into() } else { String::new() };
    } else {
        row.insert(0, if active { "ON".into() } else { String::new() });
    }
}

fn header_value(headers: &[String], row: &[String], header: &str, fallback_idx: usize) -> Option<String> {
    if let Some(idx) = headers.iter().position(|h| h == header) {
        return row.get(idx).cloned();
    }
    row.get(fallback_idx).cloned()
}

async fn detect_remote_port(client: &kube::Client, pod_name: &str, namespace: &str) -> Option<u16> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let pod = pods.get(pod_name).await.ok()?;
    let spec = pod.spec?;

    let mut all_ports: Vec<(u16, Option<String>)> = Vec::new();
    for container in spec.containers {
        if let Some(ports) = container.ports {
            for p in ports {
                if p.container_port <= 0 {
                    continue;
                }
                if let Ok(port) = u16::try_from(p.container_port) {
                    all_ports.push((port, p.name));
                }
            }
        }
    }

    if all_ports.is_empty() {
        return None;
    }

    // Prefer explicitly named HTTP ports first.
    for (port, name) in &all_ports {
        if name.as_deref().is_some_and(|n| n.contains("http") || n.contains("web")) {
            return Some(*port);
        }
    }

    // Then prefer common HTTP ports.
    for preferred in [80u16, 8080, 8000, 3000, 5000] {
        if all_ports.iter().any(|(p, _)| *p == preferred) {
            return Some(preferred);
        }
    }

    // Fall back to the first declared container port.
    Some(all_ports[0].0)
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
