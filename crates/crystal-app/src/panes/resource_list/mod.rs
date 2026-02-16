use std::any::Any;

use ratatui::prelude::{Frame, Rect};

use crystal_tui::pane::{Pane, PaneCommand, ResourceKind, ViewType};
use crystal_tui::widgets::resource_list::ResourceListWidget;

use crate::state::ResourceListState;

pub struct ResourceListPane {
    view_type: ViewType,
    pub state: ResourceListState,
    pub filter_text: String,
    pub filtered_indices: Vec<usize>,
    pub sort_column: Option<usize>,
    pub sort_ascending: bool,
    pub all_namespaces: bool,
}

impl ResourceListPane {
    pub fn new(kind: ResourceKind, headers: Vec<String>) -> Self {
        Self {
            view_type: ViewType::ResourceList(kind),
            state: ResourceListState::new(headers),
            filter_text: String::new(),
            filtered_indices: Vec::new(),
            sort_column: None,
            sort_ascending: true,
            all_namespaces: false,
        }
    }

    pub fn apply_filter(&mut self) {
        if self.filter_text.is_empty() {
            self.filtered_indices = (0..self.state.items.len()).collect();
        } else {
            let query = self.filter_text.to_lowercase();
            self.filtered_indices = self
                .state
                .items
                .iter()
                .enumerate()
                .filter(|(_, row)| row.iter().any(|cell| cell.to_lowercase().contains(&query)))
                .map(|(i, _)| i)
                .collect();
        }
        self.state.selected = if self.filtered_indices.is_empty() { None } else { Some(0) };
    }

    pub fn apply_sort(&mut self) {
        let Some(col) = self.sort_column else { return };
        let asc = self.sort_ascending;
        let items = &self.state.items;

        self.filtered_indices.sort_by(|&a, &b| {
            let va = items[a].get(col).map(|s| s.as_str()).unwrap_or("");
            let vb = items[b].get(col).map(|s| s.as_str()).unwrap_or("");
            let ord = va.cmp(vb);
            if asc {
                ord
            } else {
                ord.reverse()
            }
        });
    }

    pub fn sort_by_column(&mut self, col: usize) {
        if self.sort_column == Some(col) {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = Some(col);
            self.sort_ascending = true;
        }
        self.apply_sort();
    }

    pub fn refresh_filter_and_sort(&mut self) {
        self.apply_filter();
        self.apply_sort();
    }

    fn filtered_items(&self) -> Vec<&Vec<String>> {
        self.filtered_indices.iter().map(|&i| &self.state.items[i]).collect()
    }

    fn nav_next(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.state.selected = Some(match self.state.selected {
            Some(i) => (i + 1) % self.filtered_indices.len(),
            None => 0,
        });
    }

    fn nav_prev(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.state.selected = Some(match self.state.selected {
            Some(0) | None => self.filtered_indices.len().saturating_sub(1),
            Some(i) => i - 1,
        });
    }

    pub fn kind(&self) -> Option<&ResourceKind> {
        match &self.view_type {
            ViewType::ResourceList(k) => Some(k),
            _ => None,
        }
    }
}

impl Pane for ResourceListPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &crystal_tui::theme::Theme) {
        let title = match &self.view_type {
            ViewType::ResourceList(kind) => kind.display_name(),
            _ => "Resources",
        };

        let filtered = self.filtered_items();

        let widget = ResourceListWidget {
            title,
            headers: &self.state.headers,
            items: &filtered,
            selected: self.state.selected,
            scroll_offset: self.state.scroll_offset,
            loading: self.state.loading,
            error: self.state.error.as_deref(),
            focused,
            filter_text: if self.filter_text.is_empty() { None } else { Some(&self.filter_text) },
            sort_column: self.sort_column,
            sort_ascending: self.sort_ascending,
            total_count: self.state.items.len(),
            all_namespaces: self.all_namespaces,
            theme,
        };
        widget.render(frame, area);
    }

    fn handle_command(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::SelectNext | PaneCommand::ScrollDown => self.nav_next(),
            PaneCommand::SelectPrev | PaneCommand::ScrollUp => self.nav_prev(),
            PaneCommand::Filter(text) => {
                self.filter_text = text.clone();
                self.apply_filter();
                self.apply_sort();
            }
            PaneCommand::ClearFilter => {
                self.filter_text.clear();
                self.apply_filter();
                self.apply_sort();
            }
            PaneCommand::SortByColumn(col) => {
                self.sort_by_column(*col);
            }
            PaneCommand::ToggleSortOrder => {
                self.sort_ascending = !self.sort_ascending;
                self.apply_sort();
            }
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

#[cfg(test)]
mod tests;
