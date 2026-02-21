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
}

#[cfg(test)]
mod tests;
