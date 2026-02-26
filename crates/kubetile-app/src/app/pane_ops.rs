use kubetile_tui::pane::{find_pane_in_direction, Direction, PaneId, ResourceKind, SplitDirection, ViewType};

use crate::command::InputMode;
use crate::panes::ResourceListPane;

use super::App;

impl App {
    pub(super) fn toggle_help(&mut self) {
        let active_pane_ids = self.tab_manager.active().pane_tree.leaf_ids();
        let help_pane_id = active_pane_ids
            .iter()
            .find(|id| self.panes.get(id).is_some_and(|p| matches!(p.view_type(), ViewType::Help)))
            .copied();

        if let Some(id) = help_pane_id {
            self.close_pane(id);
        } else {
            let focused = self.tab_manager.active().focused_pane;
            if let Some(new_id) = self.tab_manager.split_pane(focused, SplitDirection::Vertical, ViewType::Help) {
                let help = crate::panes::HelpPane::new(
                    self.dispatcher.global_shortcuts(),
                    self.dispatcher.navigation_shortcuts(),
                    self.dispatcher.browse_shortcuts(),
                    self.dispatcher.tui_shortcuts(),
                    self.dispatcher.interact_shortcuts(),
                    self.dispatcher.mutate_shortcuts(),
                );
                self.panes.insert(new_id, Box::new(help));
                self.set_focus(new_id);
            }
        }
    }

    pub(super) fn focus_next(&mut self) {
        let ids = self.tab_manager.active().pane_tree.leaf_ids();
        if ids.is_empty() {
            return;
        }
        let focused = self.tab_manager.active().focused_pane;
        let pos = ids.iter().position(|&id| id == focused).unwrap_or(0);
        let next = ids[(pos + 1) % ids.len()];
        self.set_focus(next);
    }

    pub(super) fn focus_prev(&mut self) {
        let ids = self.tab_manager.active().pane_tree.leaf_ids();
        if ids.is_empty() {
            return;
        }
        let focused = self.tab_manager.active().focused_pane;
        let pos = ids.iter().position(|&id| id == focused).unwrap_or(0);
        let prev = ids[(pos + ids.len() - 1) % ids.len()];
        self.set_focus(prev);
    }

    pub(super) fn set_focus(&mut self, new_id: PaneId) {
        let focused = self.tab_manager.active().focused_pane;
        let prev_view = self.panes.get(&focused).map(|p| p.view_type().clone());
        self.tab_manager.active_mut().focused_pane = new_id;
        if let Some(pane) = self.panes.get_mut(&new_id) {
            pane.on_focus_change(prev_view.as_ref());
        }
        self.update_active_tab_title();

        let new_is_query = self.panes.get(&new_id).is_some_and(|p| matches!(p.view_type(), ViewType::Query(_)));
        match (new_is_query, self.dispatcher.mode()) {
            (true, InputMode::Normal) => self.dispatcher.set_mode(InputMode::QueryEditor),
            (false, InputMode::QueryEditor | InputMode::QueryBrowse | InputMode::QueryHistory) => {
                self.dispatcher.set_mode(InputMode::Normal)
            }
            _ => {}
        }
    }

    pub(super) fn split_focused(&mut self, direction: SplitDirection) {
        let focused = self.tab_manager.active().focused_pane;
        let view = ViewType::Empty;
        if let Some(new_id) = self.tab_manager.split_pane(focused, direction, view.clone()) {
            self.panes.insert(new_id, Box::new(super::EmptyPane(view)));
            self.set_focus(new_id);
        }
    }

    pub(super) fn close_focused(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let pane_count = self.tab_manager.active().pane_tree.leaf_ids().len();
        if pane_count <= 1 {
            self.close_tab();
        } else {
            self.close_pane(focused);
        }
    }

    pub(super) fn close_pane(&mut self, target: PaneId) {
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
            self.watcher_seq_by_pane.remove(&target);
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

    pub(super) fn focus_direction(&mut self, dir: Direction) {
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

    pub(super) fn toggle_fullscreen(&mut self) {
        let tab = self.tab_manager.active_mut();
        if tab.fullscreen_pane.is_some() {
            tab.fullscreen_pane = None;
        } else {
            tab.fullscreen_pane = Some(tab.focused_pane);
        }
    }

    pub(super) fn switch_resource(&mut self, kind: ResourceKind) {
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

    pub(super) fn handle_resource_update(&mut self, pane_id: PaneId, headers: Vec<String>, rows: Vec<Vec<String>>) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(resource_pane) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                let previous_selected_resource = selected_resource_identity(resource_pane);
                let configured_columns = resource_pane
                    .kind()
                    .map(|k| self.views_config.columns_for(super::resource_kind_config_key(k)))
                    .unwrap_or(&[]);

                let (effective_headers, effective_rows) =
                    kubetile_config::views::filter_columns(configured_columns, &headers, &rows);

                if !effective_headers.is_empty() {
                    resource_pane.state.headers = effective_headers;
                }
                resource_pane.state.set_items(effective_rows);
                resource_pane.refresh_filter_and_sort();
                if let Some((name, namespace)) = previous_selected_resource {
                    if let Some(item_idx) = find_item_index_by_identity(
                        &resource_pane.state.headers,
                        &resource_pane.state.items,
                        &name,
                        &namespace,
                    ) {
                        let _ = resource_pane.select_item_index(item_idx);
                    }
                }
            }
        }
    }

    pub(super) fn handle_resource_error(&mut self, pane_id: PaneId, error: String) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(resource_pane) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                resource_pane.state.set_error(error);
            }
        }
    }

    pub(super) fn with_pods_pane(&mut self, f: impl FnOnce(&mut ResourceListPane)) {
        if let Some(pane) = self.panes.get_mut(&self.pods_pane_id) {
            if let Some(resource_pane) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                f(resource_pane);
            }
        }
    }
}

pub(super) fn selected_resource_identity(resource_pane: &ResourceListPane) -> Option<(String, String)> {
    let selected_idx = match resource_pane.state.selected {
        Some(s) => {
            if resource_pane.filtered_indices.is_empty() {
                s
            } else {
                *resource_pane.filtered_indices.get(s)?
            }
        }
        None => return None,
    };
    let row = resource_pane.state.items.get(selected_idx)?;
    let name = super::header_value(&resource_pane.state.headers, row, "NAME", 0)?;
    let namespace = super::header_value(&resource_pane.state.headers, row, "NAMESPACE", usize::MAX).unwrap_or_default();
    Some((name, namespace))
}

pub(super) fn find_item_index_by_identity(
    headers: &[String],
    items: &[Vec<String>],
    selected_name: &str,
    selected_namespace: &str,
) -> Option<usize> {
    items.iter().position(|row| {
        let name = super::header_value(headers, row, "NAME", 0);
        let namespace = super::header_value(headers, row, "NAMESPACE", usize::MAX).unwrap_or_default();
        name.as_deref() == Some(selected_name) && namespace == selected_namespace
    })
}
