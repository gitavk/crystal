use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use k8s_openapi::api::core::v1::Pod;
use kube::Api;
use ratatui::backend::Backend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crystal_core::informer::{ResourceEvent, ResourceWatcher};
use crystal_core::resource::format_duration;
use crystal_core::{ContextResolver, KubeClient};
use crystal_tui::layout::{NamespaceSelectorView, RenderContext, ResourceListView};

use crate::event::{AppEvent, EventHandler};
use crate::state::ResourceListState;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View {
    Pods,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum InputMode {
    Normal,
    NamespaceSelector,
}

pub struct App {
    running: bool,
    tick_rate: Duration,
    kube_client: Option<KubeClient>,
    context_resolver: ContextResolver,
    pod_list: ResourceListState,
    current_view: View,
    input_mode: InputMode,
    namespaces: Vec<String>,
    namespace_filter: String,
    namespace_selected: usize,
    pod_watcher: Option<ResourceWatcher>,
    pending_namespace_switch: Option<String>,
}

impl App {
    pub async fn new(tick_rate_ms: u64) -> Self {
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

        Self {
            running: true,
            tick_rate: Duration::from_millis(tick_rate_ms),
            kube_client,
            context_resolver,
            pod_list: ResourceListState::new(pod_headers),
            current_view: View::Pods,
            input_mode: InputMode::Normal,
            namespaces: Vec::new(),
            namespace_filter: String::new(),
            namespace_selected: 0,
            pod_watcher: None,
            pending_namespace_switch: None,
        }
    }

    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        let mut events = EventHandler::new(self.tick_rate);

        if let Some(client) = &self.kube_client {
            let ns = client.namespace().to_string();
            let inner = client.inner_client();
            self.init_pod_watcher(&ns, inner, &events);

            if let Some(client) = &self.kube_client {
                match client.list_namespaces().await {
                    Ok(ns_list) => self.namespaces = ns_list,
                    Err(e) => tracing::warn!("Failed to list namespaces: {e}"),
                }
            }
        } else {
            self.pod_list.loading = false;
            self.pod_list.error = Some("No cluster connection".into());
        }

        while self.running {
            if let Some(ns) = self.pending_namespace_switch.take() {
                if let Some(client) = &self.kube_client {
                    let inner = client.inner_client();
                    self.init_pod_watcher(&ns, inner, &events);
                }
            }

            terminal.draw(|frame| {
                let ctx = self.build_render_context();
                crystal_tui::layout::render_root(frame, &ctx);
            })?;

            match events.next().await? {
                AppEvent::Key(key) => self.handle_key(key),
                AppEvent::Tick => {}
                AppEvent::Resize(_, _) => {}
                AppEvent::KubeUpdate(update) => self.handle_kube_update(update),
            }
        }

        Ok(())
    }

    fn init_pod_watcher(&mut self, namespace: &str, client: kube::Client, events: &EventHandler) {
        self.pod_watcher = None;
        let pod_api: Api<Pod> = Api::namespaced(client, namespace);
        let (tx, rx) = mpsc::channel(16);
        self.pod_watcher = Some(ResourceWatcher::watch_pods(pod_api, tx));
        events.forward_kube_events(rx);
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::NamespaceSelector => self.handle_namespace_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.running = false,
            KeyCode::Char('j') | KeyCode::Down => self.pod_list.next(),
            KeyCode::Char('k') | KeyCode::Up => self.pod_list.previous(),
            KeyCode::Char(':') => {
                self.input_mode = InputMode::NamespaceSelector;
                self.namespace_filter.clear();
                self.namespace_selected = 0;
            }
            KeyCode::Char('1') => self.current_view = View::Pods,
            _ => {}
        }
    }

    fn handle_namespace_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.input_mode = InputMode::Normal,
            KeyCode::Enter => {
                self.select_namespace();
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Up => {
                self.namespace_selected = self.namespace_selected.saturating_sub(1);
            }
            KeyCode::Down => {
                let count = self.filtered_namespaces().len();
                if self.namespace_selected + 1 < count {
                    self.namespace_selected += 1;
                }
            }
            KeyCode::Char(c) => {
                self.namespace_filter.push(c);
                self.namespace_selected = 0;
            }
            KeyCode::Backspace => {
                self.namespace_filter.pop();
                self.namespace_selected = 0;
            }
            _ => {}
        }
    }

    fn handle_kube_update(&mut self, update: ResourceEvent<crystal_core::PodSummary>) {
        match update {
            ResourceEvent::Updated(pods) => {
                let rows: Vec<Vec<String>> = pods
                    .iter()
                    .map(|p| {
                        vec![
                            p.name.clone(),
                            p.namespace.clone(),
                            p.status.to_string(),
                            p.ready.clone(),
                            p.restarts.to_string(),
                            format_duration(p.age),
                            p.node.clone().unwrap_or_default(),
                        ]
                    })
                    .collect();
                self.pod_list.set_items(rows);
            }
            ResourceEvent::Error(e) => {
                self.pod_list.set_error(e);
            }
        }
    }

    fn select_namespace(&mut self) {
        let filtered = self.filtered_namespaces();
        if let Some(ns) = filtered.get(self.namespace_selected).cloned() {
            let ns = if ns == "All Namespaces" { "default".to_string() } else { ns };

            self.pod_watcher = None;

            if let Some(ref mut client) = self.kube_client {
                client.set_namespace(&ns);
            }
            self.context_resolver.set_namespace(&ns);

            self.pod_list = ResourceListState::new(self.pod_list.headers.clone());
            self.pending_namespace_switch = Some(ns);
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

    fn build_render_context(&self) -> RenderContext<'_> {
        let resource_list = match self.current_view {
            View::Pods => Some(ResourceListView {
                title: "Pods",
                headers: &self.pod_list.headers,
                items: &self.pod_list.items,
                selected: self.pod_list.selected,
                scroll_offset: self.pod_list.scroll_offset,
                loading: self.pod_list.loading,
                error: self.pod_list.error.as_deref(),
            }),
        };

        let namespace_selector = if self.input_mode == InputMode::NamespaceSelector {
            Some(NamespaceSelectorView {
                namespaces: &self.namespaces,
                filter: &self.namespace_filter,
                selected: self.namespace_selected,
            })
        } else {
            None
        };

        RenderContext {
            cluster_name: self.context_resolver.context_name(),
            namespace: self.context_resolver.namespace(),
            resource_list,
            namespace_selector,
        }
    }
}
