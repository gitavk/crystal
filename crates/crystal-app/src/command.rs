use crystal_tui::pane::{Direction, PaneCommand};

pub use crate::keybindings::InputMode;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum Command {
    Quit,
    ShowHelp,
    FocusNextPane,
    FocusPrevPane,
    FocusDirection(Direction),
    SplitVertical,
    SplitHorizontal,
    ClosePane,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    GoToTab(usize),
    ToggleFullscreen,
    ResizeGrow,
    ResizeShrink,
    EnterMode(InputMode),
    ExitMode,
    NamespaceConfirm,
    NamespaceInput(char),
    NamespaceBackspace,
    Pane(PaneCommand),
}

#[cfg(test)]
mod tests;
