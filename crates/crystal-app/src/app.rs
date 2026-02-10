use std::collections::HashMap;
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
use crystal_tui::layout::{NamespaceSelectorView, RenderContext};
use crystal_tui::pane::{Pane, PaneId, PaneTree, ResourceKind, SplitDirection, ViewType};

use crate::command::{map_key_to_command, Command, InputMode};
use crate::event::{AppEvent, EventHandler};
use crate::panes::{HelpPane, ResourceListPane};

pub struct App {
    running: bool,
    tick_rate: Duration,
    kube_client: Option<KubeClient>,
    context_resolver: ContextResolver,
    input_mode: InputMode,
    namespaces: Vec<String>,
    namespace_filter: String,
    namespace_selected: usize,
    pod_watcher: Option<ResourceWatcher>,
    pending_namespace_switch: Option<String>,
    pane_tree: PaneTree,
    focused_pane: PaneId,
    panes: HashMap<PaneId, Box<dyn Pane>>,
    pods_pane_id: PaneId,
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

        let pods_pane = ResourceListPane::new(ResourceKind::Pods, pod_headers);
        let pane_tree = PaneTree::new(ViewType::ResourceList(ResourceKind::Pods));
        let pods_pane_id = 1; // first leaf in a new PaneTree

        let mut panes: HashMap<PaneId, Box<dyn Pane>> = HashMap::new();
        panes.insert(pods_pane_id, Box::new(pods_pane));

        Self {
            running: true,
            tick_rate: Duration::from_millis(tick_rate_ms),
            kube_client,
            context_resolver,
            input_mode: InputMode::Normal,
            namespaces: Vec::new(),
            namespace_filter: String::new(),
            namespace_selected: 0,
            pod_watcher: None,
            pending_namespace_switch: None,
            pane_tree,
            focused_pane: pods_pane_id,
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
            InputMode::Normal => {
                if let Some(cmd) = map_key_to_command(key, InputMode::Normal) {
                    self.handle_command(cmd);
                }
            }
            InputMode::NamespaceSelector => self.handle_namespace_key(key),
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
                self.input_mode = mode;
                if mode == InputMode::NamespaceSelector {
                    self.namespace_filter.clear();
                    self.namespace_selected = 0;
                }
            }
            Command::ExitMode => self.input_mode = InputMode::Normal,
            Command::Pane(pane_cmd) => {
                if let Some(pane) = self.panes.get_mut(&self.focused_pane) {
                    pane.handle_command(&pane_cmd);
                }
            }
            _ => {}
        }
    }

    fn toggle_help(&mut self) {
        let help_pane_id =
            self.panes.iter().find_map(
                |(id, p)| {
                    if matches!(p.view_type(), ViewType::Help) {
                        Some(*id)
                    } else {
                        None
                    }
                },
            );

        if let Some(id) = help_pane_id {
            self.close_pane(id);
        } else {
            let prev_view = self.panes.get(&self.focused_pane).map(|p| p.view_type().clone());
            if let Some(new_id) = self.pane_tree.split(self.focused_pane, SplitDirection::Vertical, ViewType::Help) {
                let mut help = HelpPane::new();
                help.on_focus_change(prev_view.as_ref());
                self.panes.insert(new_id, Box::new(help));
                self.set_focus(new_id);
            }
        }
    }

    fn focus_next(&mut self) {
        let ids = self.pane_tree.leaf_ids();
        if ids.is_empty() {
            return;
        }
        let pos = ids.iter().position(|&id| id == self.focused_pane).unwrap_or(0);
        let next = ids[(pos + 1) % ids.len()];
        self.set_focus(next);
    }

    fn focus_prev(&mut self) {
        let ids = self.pane_tree.leaf_ids();
        if ids.is_empty() {
            return;
        }
        let pos = ids.iter().position(|&id| id == self.focused_pane).unwrap_or(0);
        let prev = ids[(pos + ids.len() - 1) % ids.len()];
        self.set_focus(prev);
    }

    fn set_focus(&mut self, new_id: PaneId) {
        let prev_view = self.panes.get(&self.focused_pane).map(|p| p.view_type().clone());
        self.focused_pane = new_id;
        if let Some(pane) = self.panes.get_mut(&new_id) {
            pane.on_focus_change(prev_view.as_ref());
        }
    }

    fn split_focused(&mut self, direction: SplitDirection) {
        let view = ViewType::Empty;
        if let Some(new_id) = self.pane_tree.split(self.focused_pane, direction, view.clone()) {
            self.panes.insert(new_id, Box::new(EmptyPane(view)));
            self.set_focus(new_id);
        }
    }

    fn close_focused(&mut self) {
        self.close_pane(self.focused_pane);
    }

    fn close_pane(&mut self, target: PaneId) {
        let ids = self.pane_tree.leaf_ids();
        if ids.len() <= 1 {
            return;
        }
        let was_focused = target == self.focused_pane;
        if self.pane_tree.close(target) {
            self.panes.remove(&target);
            if was_focused {
                let remaining = self.pane_tree.leaf_ids();
                if let Some(&first) = remaining.first() {
                    self.set_focus(first);
                }
            }
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

    fn build_render_context(&self) -> RenderContext<'_> {
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
            namespace_selector,
            pane_tree: &self.pane_tree,
            focused_pane: Some(self.focused_pane),
            panes: &self.panes,
        }
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
mod tests {
    use super::*;
    use crystal_tui::pane::PaneCommand;

    fn make_test_app() -> (HashMap<PaneId, Box<dyn Pane>>, PaneTree, PaneId) {
        let headers = vec!["NAME".into(), "STATUS".into()];
        let mut pane1 = ResourceListPane::new(ResourceKind::Pods, headers.clone());
        pane1.state.set_items(vec![vec!["pod-a".into(), "Running".into()], vec!["pod-b".into(), "Pending".into()]]);

        let mut pane2 = ResourceListPane::new(ResourceKind::Services, headers);
        pane2.state.set_items(vec![vec!["svc-a".into(), "Active".into()]]);

        let mut tree = PaneTree::new(ViewType::ResourceList(ResourceKind::Pods));
        let pane2_id = tree.split(1, SplitDirection::Vertical, ViewType::ResourceList(ResourceKind::Services)).unwrap();

        let mut panes: HashMap<PaneId, Box<dyn Pane>> = HashMap::new();
        panes.insert(1, Box::new(pane1));
        panes.insert(pane2_id, Box::new(pane2));

        (panes, tree, 1)
    }

    #[test]
    fn pane_command_dispatched_to_focused_only() {
        let (mut panes, _tree, focused) = make_test_app();

        // focused pane (1) starts at selection 0
        assert_eq!(
            panes.get(&focused).unwrap().as_any().downcast_ref::<ResourceListPane>().unwrap().state.selected,
            Some(0)
        );

        // dispatch SelectNext to focused pane
        if let Some(pane) = panes.get_mut(&focused) {
            pane.handle_command(&PaneCommand::SelectNext);
        }

        // focused pane moved to selection 1
        assert_eq!(
            panes.get(&focused).unwrap().as_any().downcast_ref::<ResourceListPane>().unwrap().state.selected,
            Some(1)
        );

        // unfocused pane (2) still at selection 0
        assert_eq!(panes.get(&2).unwrap().as_any().downcast_ref::<ResourceListPane>().unwrap().state.selected, Some(0));
    }

    #[test]
    fn unfocused_pane_receives_no_commands() {
        let (mut panes, _tree, focused) = make_test_app();
        let unfocused = 2;
        assert_ne!(focused, unfocused);

        // send multiple commands — only to focused
        for _ in 0..3 {
            if let Some(pane) = panes.get_mut(&focused) {
                pane.handle_command(&PaneCommand::SelectNext);
            }
        }

        // unfocused pane unchanged
        let unfocused_pane = panes.get(&unfocused).unwrap().as_any().downcast_ref::<ResourceListPane>().unwrap();
        assert_eq!(unfocused_pane.state.selected, Some(0));
    }

    #[test]
    fn global_command_takes_precedence() {
        // In the key mapping, 'q' always maps to Command::Quit (global),
        // never to a pane command, regardless of focus state.
        use crossterm::event::{KeyCode, KeyModifiers};

        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        let cmd = map_key_to_command(key, InputMode::Normal);
        assert_eq!(cmd, Some(Command::Quit));

        // 'j' maps to pane command, not global
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let cmd = map_key_to_command(key, InputMode::Normal);
        assert!(matches!(cmd, Some(Command::Pane(PaneCommand::SelectNext))));
    }

    #[test]
    fn focus_cycling_wraps_around() {
        let (_panes, tree, _) = make_test_app();
        let ids = tree.leaf_ids();
        assert_eq!(ids, vec![1, 2]);

        // simulate focus_next from pane 1
        let focused = 1;
        let pos = ids.iter().position(|&id| id == focused).unwrap();
        let next = ids[(pos + 1) % ids.len()];
        assert_eq!(next, 2);

        // simulate focus_next from pane 2 (wraps to 1)
        let focused = 2;
        let pos = ids.iter().position(|&id| id == focused).unwrap();
        let next = ids[(pos + 1) % ids.len()];
        assert_eq!(next, 1);
    }

    #[test]
    fn help_pane_updates_context_on_focus() {
        let mut help = HelpPane::new();
        let resource_view = ViewType::ResourceList(ResourceKind::Pods);
        help.on_focus_change(Some(&resource_view));

        // Verify help pane tracks the previous view type
        let help_ref = help.as_any().downcast_ref::<HelpPane>().unwrap();
        assert_eq!(help_ref.view_type(), &ViewType::Help);
    }
}
