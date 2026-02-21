use kubetile_tui::pane::PaneCommand;

use crate::command::InputMode;
use crate::event::AppEvent;
use crate::panes::ResourceListPane;

use super::App;

impl App {
    pub(super) fn handle_namespace_confirm(&mut self) {
        self.select_namespace();
        self.dispatcher.set_mode(InputMode::Normal);
    }

    pub(super) fn handle_namespace_input(&mut self, c: char) {
        self.namespace_filter.push(c);
        self.namespace_selected = 0;
    }

    pub(super) fn handle_namespace_backspace(&mut self) {
        self.namespace_filter.pop();
        self.namespace_selected = 0;
    }

    pub(super) fn handle_context_confirm(&mut self) {
        self.select_context();
    }

    pub(super) fn handle_context_input(&mut self, c: char) {
        self.context_filter.push(c);
        self.context_selected = 0;
    }

    pub(super) fn handle_context_backspace(&mut self) {
        self.context_filter.pop();
        self.context_selected = 0;
    }

    pub(super) fn handle_namespace_nav(&mut self, cmd: &PaneCommand) {
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

    pub(super) fn handle_context_nav(&mut self, cmd: &PaneCommand) {
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

    pub(super) fn select_namespace(&mut self) {
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

    pub(super) fn filtered_namespaces(&self) -> Vec<String> {
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

    pub(super) fn filtered_contexts(&self) -> Vec<String> {
        let filter_lower = self.context_filter.to_lowercase();
        self.contexts
            .iter()
            .filter(|ctx| filter_lower.is_empty() || ctx.to_lowercase().contains(&filter_lower))
            .cloned()
            .collect()
    }

    pub(super) fn select_context(&mut self) {
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
            match kubetile_core::KubeClient::from_context(&context).await {
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

    pub(super) fn refresh_namespaces(&self) {
        let Some(client) = self.kube_client.clone() else { return };
        let app_tx = self.app_tx.clone();
        tokio::spawn(async move {
            match client.list_namespaces().await {
                Ok(namespaces) => {
                    let _ = app_tx.send(AppEvent::NamespacesUpdated { namespaces });
                }
                Err(e) => tracing::warn!("Failed to refresh namespaces: {e}"),
            }
        });
    }

    pub(super) fn apply_context_switch(&mut self, client: kubetile_core::KubeClient, namespaces: Vec<String>) {
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

    pub(super) fn restart_watchers_for_active_panes(&mut self) {
        let pane_ids: Vec<_> = self.tab_manager.active().pane_tree.leaf_ids();
        for pane_id in &pane_ids {
            self.active_watchers.remove(pane_id);
            self.watcher_seq_by_pane.remove(pane_id);
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
}
