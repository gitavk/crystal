use kubetile_tui::pane::{PaneId, ResourceKind, ViewType};

use crate::panes::{AppLogsPane, PortForwardsPane, ResourceListPane};

use super::{App, TabScope};

impl App {
    pub(super) fn new_tab(&mut self) {
        self.sync_active_scope();
        let tab_count = self.tab_manager.tabs().len();
        let name = format!("Tab {}", tab_count + 1);
        let tab_id = self.tab_manager.new_tab(&name, ViewType::ResourceList(ResourceKind::Pods));
        let pane_id = self.tab_manager.tabs().iter().find(|t| t.id == tab_id).unwrap().focused_pane;
        self.panes.insert(pane_id, Box::new(ResourceListPane::new(ResourceKind::Pods, super::pods_headers())));
        let ns = self.context_resolver.namespace().unwrap_or("default").to_string();
        self.start_watcher_for_pane(pane_id, &ResourceKind::Pods, &ns);
        self.sync_active_scope();
        self.update_active_tab_title();
    }

    pub(super) fn close_tab(&mut self) {
        self.sync_active_scope();
        let tab = self.tab_manager.active();
        let tab_id = tab.id;
        let pane_ids: Vec<PaneId> = tab.pane_tree.leaf_ids();

        if self.tab_manager.tabs().len() <= 1 {
            self.reset_last_tab_to_pods(tab_id, pane_ids);
            return;
        }

        if self.tab_manager.close_tab(tab_id) {
            self.tab_scopes.remove(&tab_id);
            for id in pane_ids {
                self.panes.remove(&id);
                self.active_watchers.remove(&id);
                self.watcher_seq_by_pane.remove(&id);
            }
            self.load_active_scope();
            self.update_active_tab_title();
        }
    }

    pub(super) fn toggle_app_logs_tab(&mut self) {
        let active_tab_id = self.tab_manager.active().id;
        if self.is_app_logs_tab(active_tab_id) {
            self.close_tab();
            return;
        }

        if let Some(idx) = self.find_app_logs_tab_index() {
            self.switch_to_tab_index(idx);
            return;
        }

        self.sync_active_scope();
        let tab_id = self.tab_manager.new_tab("App Logs", ViewType::Plugin("AppLogs".into()));
        let pane_id = self.tab_manager.tabs().iter().find(|t| t.id == tab_id).unwrap().focused_pane;
        self.panes.insert(pane_id, Box::new(AppLogsPane::new()));
        self.sync_active_scope();
        self.update_active_tab_title();
    }

    pub(super) fn toggle_port_forwards_tab(&mut self) {
        let active_tab_id = self.tab_manager.active().id;
        if self.is_port_forwards_tab(active_tab_id) {
            self.close_tab();
            return;
        }

        if let Some(idx) = self.find_port_forwards_tab_index() {
            self.switch_to_tab_index(idx);
            return;
        }

        self.sync_active_scope();
        let tab_id = self.tab_manager.new_tab("Port Forwards", ViewType::Plugin("PortForwards".into()));
        let pane_id = self.tab_manager.tabs().iter().find(|t| t.id == tab_id).unwrap().focused_pane;
        self.panes.insert(pane_id, Box::new(PortForwardsPane::new()));
        self.refresh_port_forwards_panes();
        self.sync_active_scope();
        self.update_active_tab_title();
    }

    fn is_app_logs_tab(&self, tab_id: u32) -> bool {
        let Some(tab) = self.tab_manager.tabs().iter().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.pane_tree.leaf_ids().iter().all(|pane_id| {
            self.panes
                .get(pane_id)
                .is_some_and(|p| matches!(p.view_type(), ViewType::Plugin(name) if name == "AppLogs"))
        })
    }

    fn find_app_logs_tab_index(&self) -> Option<usize> {
        self.tab_manager.tabs().iter().position(|tab| self.is_app_logs_tab(tab.id))
    }

    fn is_port_forwards_tab(&self, tab_id: u32) -> bool {
        let Some(tab) = self.tab_manager.tabs().iter().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.pane_tree.leaf_ids().iter().all(|pane_id| {
            self.panes
                .get(pane_id)
                .is_some_and(|p| matches!(p.view_type(), ViewType::Plugin(name) if name == "PortForwards"))
        })
    }

    fn find_port_forwards_tab_index(&self) -> Option<usize> {
        self.tab_manager.tabs().iter().position(|tab| self.is_port_forwards_tab(tab.id))
    }

    fn reset_last_tab_to_pods(&mut self, old_tab_id: u32, old_pane_ids: Vec<PaneId>) {
        let ns = self.context_resolver.namespace().unwrap_or("default").to_string();
        let old_scope = self.tab_scopes.get(&old_tab_id).cloned();

        let new_tab_id = self.tab_manager.new_tab("Main", ViewType::ResourceList(ResourceKind::Pods));
        let new_pane_id = self.tab_manager.tabs().iter().find(|t| t.id == new_tab_id).unwrap().focused_pane;
        self.panes.insert(new_pane_id, Box::new(ResourceListPane::new(ResourceKind::Pods, super::pods_headers())));
        self.start_watcher_for_pane(new_pane_id, &ResourceKind::Pods, &ns);

        let _ = self.tab_manager.close_tab(old_tab_id);
        for id in old_pane_ids {
            self.panes.remove(&id);
            self.active_watchers.remove(&id);
            self.watcher_seq_by_pane.remove(&id);
        }

        self.tab_scopes.remove(&old_tab_id);
        if let Some(scope) = old_scope {
            self.tab_scopes.insert(new_tab_id, scope);
        }
        self.load_active_scope();
        self.update_active_tab_title();
    }

    pub(super) fn switch_to_tab_index(&mut self, index: usize) {
        self.sync_active_scope();
        self.tab_manager.switch_tab(index);
        self.load_active_scope();
        self.update_active_tab_title();
    }

    pub(super) fn switch_to_next_tab(&mut self) {
        self.sync_active_scope();
        self.tab_manager.next_tab();
        self.load_active_scope();
        self.update_active_tab_title();
    }

    pub(super) fn switch_to_prev_tab(&mut self) {
        self.sync_active_scope();
        self.tab_manager.prev_tab();
        self.load_active_scope();
        self.update_active_tab_title();
    }

    pub(super) fn sync_active_scope(&mut self) {
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

    pub(super) fn load_active_scope(&mut self) {
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
}
