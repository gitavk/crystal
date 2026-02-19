use crystal_core::{ForwardId, SessionId};
use crystal_tui::pane::{Direction, PaneCommand};

pub use crate::keybindings::InputMode;

pub type StreamId = u64;

#[derive(Debug, Clone, PartialEq)]
pub struct LogRequest {
    pub pod: String,
    pub namespace: String,
    pub container: Option<String>,
    pub follow: bool,
    pub tail_lines: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum Command {
    Quit,
    ShowHelp,
    ToggleAppLogsTab,
    TogglePortForwardsTab,
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
    ContextConfirm,
    ContextInput(char),
    ContextBackspace,
    Pane(PaneCommand),

    // Resource actions
    ViewYaml,
    ViewDescribe,
    SaveLogsToFile,
    DeleteResource,
    ScaleResource,
    RestartRollout,
    ViewLogs,
    ExecInto,
    PortForward,
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

    // Filter input
    FilterInput(char),
    FilterBackspace,
    FilterCancel,
    PortForwardInput(char),
    PortForwardBackspace,
    PortForwardToggleField,
    PortForwardConfirm,
    PortForwardCancel,

    // Terminal lifecycle
    TerminalSpawn,
    TerminalClose { session_id: SessionId },
    TerminalResize { session_id: SessionId, cols: u16, rows: u16 },
    TerminalInput { session_id: SessionId, bytes: Vec<u8> },

    // Exec lifecycle
    ExecStart { pod: String, namespace: String, container: Option<String>, command: Vec<String> },
    ExecClose { session_id: SessionId },

    // Logs
    LogsStart { request: LogRequest },
    LogsStop { stream_id: StreamId },

    // Port forwarding
    PortForwardStart { pod: String, namespace: String, local_port: u16, remote_port: u16 },
    PortForwardStop { forward_id: ForwardId },
}

#[cfg(test)]
mod tests;
