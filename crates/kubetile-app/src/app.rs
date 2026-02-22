use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ratatui::backend::Backend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use kubetile_core::informer::ResourceWatcher;
use kubetile_core::{ContextResolver, ForwardId, KubeClient};
use kubetile_tui::pane::{Pane, PaneId, ResourceKind, ViewType};
use kubetile_tui::tab::TabManager;
use kubetile_tui::widgets::toast::ToastMessage;

use crate::command::Command;
use crate::event::{AppEvent, EventHandler};
use crate::keybindings::KeybindingDispatcher;
use crate::panes::ResourceListPane;
use crate::resource_switcher::ResourceSwitcher;

mod actions;
mod context;
mod input;
mod logs_exec;
mod pane_ops;
mod port_forward;
mod render;
mod tabs;
mod watchers;

#[allow(unused_imports)]
use pane_ops::{find_item_index_by_identity, selected_resource_identity};

#[derive(Debug, Clone)]
pub enum PendingAction {
    Delete { kind: ResourceKind, name: String, namespace: String },
    SaveLogs { path: PathBuf, content: String },
    DownloadFullLogs { path: PathBuf, pod_name: String, namespace: String, container: Option<String> },
    ToggleDebugMode { name: String, namespace: String },
    ToggleRootDebugMode { name: String, namespace: String },
    MutateCommand(Command),
}

pub struct PendingConfirmation {
    pub message: String,
    pub action: PendingAction,
}

impl PendingConfirmation {
    pub fn from_command(cmd: Command) -> Self {
        let label = match &cmd {
            Command::DeleteResource => "Delete resource",
            Command::ScaleResource => "Scale resource",
            Command::RestartRollout => "Restart rollout",
            Command::ToggleDebugMode => "Toggle debug mode",
            Command::ToggleRootDebugMode => "Toggle root debug mode",
            other => {
                let msg = format!("{other:?}");
                return Self { message: format!("Confirm: {msg}?"), action: PendingAction::MutateCommand(cmd) };
            }
        };
        Self { message: format!("{label}?"), action: PendingAction::MutateCommand(cmd) }
    }
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
    active_watchers: HashMap<PaneId, ResourceWatcher>,
    watcher_seq_by_pane: HashMap<PaneId, u64>,
    active_forwards: HashMap<ForwardId, kubetile_core::PortForward>,
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
    theme: kubetile_tui::theme::Theme,
    views_config: kubetile_config::ViewsConfig,
}

impl App {
    pub async fn new(
        tick_rate_ms: u64,
        dispatcher: KeybindingDispatcher,
        theme: kubetile_tui::theme::Theme,
        views_config: kubetile_config::ViewsConfig,
    ) -> Self {
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

        let pods_pane = ResourceListPane::new(ResourceKind::Pods, pods_headers());
        let tab_manager = TabManager::new(ViewType::ResourceList(ResourceKind::Pods));
        let pods_pane_id = 1;

        let mut panes: HashMap<PaneId, Box<dyn Pane>> = HashMap::new();
        panes.insert(pods_pane_id, Box::new(pods_pane));

        let (tx, _rx) = mpsc::unbounded_channel();

        let mut toasts = Vec::new();
        if !is_kubectl_available_with_logging() {
            tracing::warn!("kubectl not found in PATH; exec workflows will be unavailable");
            toasts.push(ToastMessage::error("kubectl was not found in PATH. Install kubectl to use exec sessions."));
        }

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
            watcher_seq_by_pane: HashMap::new(),
            active_forwards: HashMap::new(),
            pod_forward_index: HashMap::new(),
            filter_input_buffer: String::new(),
            resource_switcher: None,
            pending_confirmation: None,
            pending_port_forward: None,
            toasts,
            tab_manager,
            panes,
            pods_pane_id,
            app_tx: tx,
            theme,
            views_config,
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
                let (mut ctx, tab_names, keys) = self.build_render_context();
                ctx.tab_names = &tab_names;
                ctx.help_key = keys[0].as_deref();
                ctx.namespace_key = keys[1].as_deref();
                ctx.context_key = keys[2].as_deref();
                ctx.close_pane_key = keys[3].as_deref();
                ctx.new_tab_key = keys[4].as_deref();
                ctx.quit_key = keys[5].as_deref();
                kubetile_tui::layout::render_root(frame, &ctx);
            })?;

            let first = events.next().await?;
            self.handle_event(first);

            for event in events.drain_pending() {
                if !self.running {
                    break;
                }
                self.handle_event(event);
            }
        }

        Ok(())
    }
}

fn pods_headers() -> Vec<String> {
    vec![
        "NAME".into(),
        "NAMESPACE".into(),
        "STATUS".into(),
        "READY".into(),
        "RESTARTS".into(),
        "AGE".into(),
        "NODE".into(),
    ]
}

fn header_value(headers: &[String], row: &[String], header: &str, fallback_idx: usize) -> Option<String> {
    if let Some(idx) = headers.iter().position(|h| h == header) {
        return row.get(idx).cloned();
    }
    row.get(fallback_idx).cloned()
}

fn resource_kind_config_key(kind: &ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Pods => "pods",
        ResourceKind::Deployments => "deployments",
        ResourceKind::Services => "services",
        ResourceKind::StatefulSets => "statefulsets",
        ResourceKind::DaemonSets => "daemonsets",
        ResourceKind::Jobs => "jobs",
        ResourceKind::CronJobs => "cronjobs",
        ResourceKind::ConfigMaps => "configmaps",
        ResourceKind::Secrets => "secrets",
        ResourceKind::Ingresses => "ingresses",
        ResourceKind::Nodes => "nodes",
        ResourceKind::Namespaces => "namespaces",
        ResourceKind::PersistentVolumes | ResourceKind::PersistentVolumeClaims | ResourceKind::Custom(_) => "",
    }
}

fn is_kubectl_available_with_logging() -> bool {
    tracing::info!("Checking kubectl availability in PATH");
    let Some(path_var) = env::var_os("PATH") else {
        tracing::warn!("PATH is not set; kubectl check failed");
        return false;
    };

    let path_entries: Vec<_> = env::split_paths(&path_var).collect();
    tracing::info!("kubectl check: scanning {} PATH entries", path_entries.len());

    for dir in path_entries {
        let candidates = kubectl_binary_candidates(&dir);
        for candidate in candidates {
            tracing::debug!("kubectl check: probing {}", candidate.display());
            if candidate.is_file() {
                tracing::info!("kubectl check: found {}", candidate.display());
                return true;
            }
        }
    }

    tracing::warn!("kubectl check: binary not found in PATH");
    false
}

fn kubectl_binary_candidates(dir: &Path) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        vec![dir.join("kubectl.exe"), dir.join("kubectl.cmd"), dir.join("kubectl.bat"), dir.join("kubectl")]
    }
    #[cfg(not(windows))]
    {
        vec![dir.join("kubectl")]
    }
}

struct EmptyPane(ViewType);

impl Pane for EmptyPane {
    fn render(
        &self,
        frame: &mut ratatui::prelude::Frame,
        area: ratatui::prelude::Rect,
        focused: bool,
        theme: &kubetile_tui::theme::Theme,
    ) {
        use ratatui::widgets::{Block, Borders, Paragraph};

        let border_style = if focused { theme.border_active } else { theme.border };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Empty ")
            .title_style(theme.text_dim);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let msg = Paragraph::new("Empty pane").style(theme.text_dim);
        frame.render_widget(msg, inner);
    }

    fn handle_command(&mut self, _cmd: &kubetile_tui::pane::PaneCommand) {}

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
