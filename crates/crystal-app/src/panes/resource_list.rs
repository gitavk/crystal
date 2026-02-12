use std::any::Any;

use ratatui::prelude::{Frame, Rect};

use crystal_tui::pane::{Pane, PaneCommand, ResourceKind, ViewType};
use crystal_tui::widgets::resource_list::ResourceListWidget;

use crate::state::ResourceListState;

pub struct ResourceListPane {
    view_type: ViewType,
    pub state: ResourceListState,
}

impl ResourceListPane {
    pub fn new(kind: ResourceKind, headers: Vec<String>) -> Self {
        Self { view_type: ViewType::ResourceList(kind), state: ResourceListState::new(headers) }
    }
}

impl Pane for ResourceListPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let title = match &self.view_type {
            ViewType::ResourceList(kind) => kind.display_name(),
            _ => "Resources",
        };

        let widget = ResourceListWidget {
            title,
            headers: &self.state.headers,
            items: &self.state.items,
            selected: self.state.selected,
            scroll_offset: self.state.scroll_offset,
            loading: self.state.loading,
            error: self.state.error.as_deref(),
            focused,
        };
        widget.render(frame, area);
    }

    fn handle_command(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::SelectNext | PaneCommand::ScrollDown => self.state.next(),
            PaneCommand::SelectPrev | PaneCommand::ScrollUp => self.state.previous(),
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
