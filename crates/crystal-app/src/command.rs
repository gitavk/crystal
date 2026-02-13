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

    // Resource actions
    ViewYaml,
    ViewDescribe,
    DeleteResource,
    ScaleResource,
    RestartRollout,
    ViewLogs,
    ExecInto,
    ToggleAllNamespaces,

    // Resource switcher
    EnterResourceSwitcher,
    ResourceSwitcherInput(char),
    ResourceSwitcherBackspace,
    ResourceSwitcherConfirm,

    // Confirmation dialog
    ConfirmAction,
    DenyAction,

    // Sort
    SortByColumn,
}

#[cfg(test)]
mod tests;
