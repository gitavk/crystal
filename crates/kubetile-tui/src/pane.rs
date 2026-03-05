use std::any::Any;

use ratatui::prelude::{Frame, Rect};

use crate::theme::Theme;

mod navigation;
mod resource;
mod tree;
mod types;

pub use navigation::find_pane_in_direction;
pub use resource::{ResourceKind, ViewType};
pub use tree::{PaneNode, PaneTree};
pub use types::{Direction, PaneCommand, PaneId, SplitDirection};

/// Every pane must satisfy this contract:
/// - Render itself within a given Rect
/// - React to focus state (styling only — no behavior change)
/// - Accept PaneCommands and update internal state
/// - Never affect other panes or access global state directly
pub trait Pane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme);
    fn handle_command(&mut self, cmd: &PaneCommand);
    fn view_type(&self) -> &ViewType;
    fn on_focus_change(&mut self, _previous: Option<&ViewType>) {}
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// List panes return `(data_rows_rect, first_visible_row)` after each render.
    /// Used by mouse hit-testing to map click coordinates to row indices.
    fn list_row_geometry(&self) -> Option<(Rect, usize)> {
        None
    }
    /// List panes return `(header_row_y, col_spans)` for column-header click detection.
    fn list_header_geometry(&self) -> Option<(u16, Vec<(u16, u16)>)> {
        None
    }
    /// D1: Panes that support mouse row-range selection return `(data_rect, first_visible_row)`.
    fn text_selection_geometry(&self) -> Option<(Rect, usize)> {
        None
    }
    /// D1: True when a row-range selection is active.
    fn has_selection(&self) -> bool {
        false
    }
    /// D1: Returns the selected text (plain, no ANSI) or None if no selection.
    fn selection_text(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests;
