use std::any::Any;
use std::time::Duration;

use kubetile_core::ForwardId;
use kubetile_tui::pane::{Pane, PaneCommand, ViewType};
use kubetile_tui::widgets::resource_list::ResourceListWidget;
use ratatui::prelude::{Frame, Rect};

use crate::state::ResourceListState;

pub struct PortForwardsPane {
    view_type: ViewType,
    state: ResourceListState,
    ids: Vec<ForwardId>,
}

impl PortForwardsPane {
    pub fn new() -> Self {
        Self {
            view_type: ViewType::Plugin("PortForwards".into()),
            state: ResourceListState::new(vec![
                "POD".into(),
                "NAMESPACE".into(),
                "LOCAL".into(),
                "REMOTE".into(),
                "AGE".into(),
            ]),
            ids: Vec::new(),
        }
    }

    pub fn set_items(&mut self, items: Vec<(ForwardId, String, String, u16, u16, Duration)>) {
        self.ids = items.iter().map(|(id, ..)| *id).collect();
        let rows = items
            .into_iter()
            .map(|(_, pod, namespace, local, remote, age)| {
                vec![
                    pod,
                    namespace,
                    local.to_string(),
                    remote.to_string(),
                    kubetile_core::resource::format_duration(age),
                ]
            })
            .collect();
        self.state.set_items(rows);
    }

    pub fn selected_forward_id(&self) -> Option<ForwardId> {
        let selected = self.state.selected?;
        self.ids.get(selected).copied()
    }

    fn nav_next(&mut self) {
        if self.state.items.is_empty() {
            return;
        }
        self.state.selected = Some(match self.state.selected {
            Some(i) => (i + 1) % self.state.items.len(),
            None => 0,
        });
    }

    fn nav_prev(&mut self) {
        if self.state.items.is_empty() {
            return;
        }
        self.state.selected = Some(match self.state.selected {
            Some(0) | None => self.state.items.len().saturating_sub(1),
            Some(i) => i - 1,
        });
    }
}

impl Pane for PortForwardsPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &kubetile_tui::theme::Theme) {
        let items: Vec<&Vec<String>> = self.state.items.iter().collect();
        let widget = ResourceListWidget {
            title: "Port Forwards",
            headers: &self.state.headers,
            items: &items,
            selected: self.state.selected,
            scroll_offset: self.state.scroll_offset,
            loading: self.state.loading,
            error: self.state.error.as_deref(),
            focused,
            filter_text: None,
            sort_column: None,
            sort_ascending: true,
            total_count: self.state.items.len(),
            all_namespaces: false,
            theme,
        };
        widget.render(frame, area);
    }

    fn handle_command(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::SelectNext | PaneCommand::ScrollDown => self.nav_next(),
            PaneCommand::SelectPrev | PaneCommand::ScrollUp => self.nav_prev(),
            _ => {}
        }
    }

    fn view_type(&self) -> &ViewType {
        &self.view_type
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
