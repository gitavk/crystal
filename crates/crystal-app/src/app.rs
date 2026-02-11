use std::collections::HashMap;
use std::time::Duration;

use crossterm::event::{KeyEvent, KeyEventKind};
use k8s_openapi::api::core::v1::Pod;
use kube::Api;
use ratatui::backend::Backend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crystal_core::informer::{ResourceEvent, ResourceWatcher};
use crystal_core::resource::format_duration;
use crystal_core::{ContextResolver, KubeClient};
use crystal_tui::layout::{NamespaceSelectorView, RenderContext};
use crystal_tui::pane::{
    find_pane_in_direction, Direction, Pane, PaneCommand, PaneId, ResourceKind, SplitDirection, ViewType,
};
use crystal_tui::tab::TabManager;

use crate::command::{Command, InputMode};
use crate::event::{AppEvent, EventHandler};
use crate::keybindings::KeybindingDispatcher;
use crate::panes::{HelpPane, ResourceListPane};

pub struct App {
    running: bool,
    tick_rate: Duration,
    kube_client: Option<KubeClient>,
    context_resolver: ContextResolver,
    dispatcher: KeybindingDispatcher,
    namespaces: Vec<String>,
    namespace_filter: String,
    namespace_selected: usize,
    pod_watcher: Option<ResourceWatcher>,
    pending_namespace_switch: Option<String>,
    tab_manager: TabManager,
    panes: HashMap<PaneId, Box<dyn Pane>>,
    pods_pane_id: PaneId,
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

        Self {
            running: true,
            tick_rate: Duration::from_millis(tick_rate_ms),
            kube_client,
            context_resolver,
            dispatcher,
            namespaces: Vec::new(),
            namespace_filter: String::new(),
            namespace_selected: 0,
            pod_watcher: None,
            pending_namespace_switch: None,
            tab_manager,
            panes,
            pods_pane_id,
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
            self.with_pods_pane(|pane| {
                pane.state.loading = false;
                pane.state.error = Some("No cluster connection".into());
            });
        }

        while self.running {
            if let Some(ns) = self.pending_namespace_switch.take() {
                if let Some(client) = &self.kube_client {
                    let inner = client.inner_client();
                    self.init_pod_watcher(&ns, inner, &events);
                }
            }

            terminal.draw(|frame| {
                let (mut ctx, tab_names, hints) = self.build_render_context();
                ctx.tab_names = &tab_names;
                ctx.mode_hints = &hints;
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
                self.tab_manager.active_mut().pane_tree.resize(focused, 0.05);
            }
            Command::ResizeShrink => {
                let focused = self.tab_manager.active().focused_pane;
                self.tab_manager.active_mut().pane_tree.resize(focused, -0.05);
            }
            Command::Pane(ref pane_cmd) if self.dispatcher.mode() == InputMode::NamespaceSelector => {
                self.handle_namespace_nav(pane_cmd);
            }
            Command::Pane(pane_cmd) => {
                let focused = self.tab_manager.active().focused_pane;
                if let Some(pane) = self.panes.get_mut(&focused) {
                    pane.handle_command(&pane_cmd);
                }
            }
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
                let mut help = HelpPane::new(global, pane_sc);
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
                self.with_pods_pane(|pane| pane.state.set_items(rows));
            }
            ResourceEvent::Error(e) => {
                self.with_pods_pane(|pane| pane.state.set_error(e));
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

    fn select_namespace(&mut self) {
        let filtered = self.filtered_namespaces();
        if let Some(ns) = filtered.get(self.namespace_selected).cloned() {
            let ns = if ns == "All Namespaces" { "default".to_string() } else { ns };

            self.pod_watcher = None;

            if let Some(ref mut client) = self.kube_client {
                client.set_namespace(&ns);
            }
            self.context_resolver.set_namespace(&ns);

            self.with_pods_pane(|pane| {
                let headers = pane.state.headers.clone();
                pane.state = crate::state::ResourceListState::new(headers);
            });
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

    fn mode_name(&self) -> &'static str {
        match self.dispatcher.mode() {
            InputMode::Normal => "Normal",
            InputMode::NamespaceSelector => "Namespace",
            InputMode::Pane => "Pane",
            InputMode::Tab => "Tab",
            InputMode::Search => "Search",
            InputMode::Command => "Command",
            InputMode::Insert => "Insert",
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

        let tab_names = self.tab_manager.tab_names();
        let hints = self.mode_hints();

        let tab = self.tab_manager.active();
        let (pane_tree, focused_pane, fullscreen_pane) = (&tab.pane_tree, tab.focused_pane, tab.fullscreen_pane);

        let ctx = RenderContext {
            cluster_name: self.context_resolver.context_name(),
            namespace: self.context_resolver.namespace(),
            namespace_selector,
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
