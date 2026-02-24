use kubetile_tui::pane::{Direction, PaneCommand};

use super::InputMode;
use crate::command::Command;

pub(super) fn global_command_from_name(name: &str) -> Option<Command> {
    match name {
        "quit" => Some(Command::Quit),
        "help" => Some(Command::ShowHelp),
        "app_logs" => Some(Command::ToggleAppLogsTab),
        "port_forwards" => Some(Command::TogglePortForwardsTab),
        "enter_insert" => Some(Command::EnterMode(InputMode::Insert)),
        "namespace_selector" => Some(Command::EnterMode(InputMode::NamespaceSelector)),
        "context_selector" => Some(Command::EnterMode(InputMode::ContextSelector)),
        _ => None,
    }
}

pub(super) fn global_command_description(name: &str) -> String {
    match name {
        "quit" => "Quit",
        "help" => "Help",
        "app_logs" => "App logs",
        "port_forwards" => "Port forwards",
        "enter_insert" => "Insert mode",
        "namespace_selector" => "Namespace",
        "context_selector" => "Context",
        _ => "Unknown",
    }
    .into()
}

pub(super) fn mutate_command_from_name(name: &str) -> Option<Command> {
    match name {
        "delete" => Some(Command::DeleteResource),
        "scale" => Some(Command::ScaleResource),
        "restart_rollout" => Some(Command::RestartRollout),
        "debug_mode" => Some(Command::ToggleDebugMode),
        "root_debug_mode" => Some(Command::ToggleRootDebugMode),
        _ => None,
    }
}

pub(super) fn mutate_command_description(name: &str) -> String {
    match name {
        "delete" => "Delete",
        "scale" => "Scale",
        "restart_rollout" => "Restart",
        "debug_mode" => "Debug mode",
        "root_debug_mode" => "Root debug mode",
        _ => "Unknown",
    }
    .into()
}

pub(super) fn interact_command_from_name(name: &str) -> Option<Command> {
    match name {
        "exec" => Some(Command::ExecInto),
        "open_query" => Some(Command::OpenQueryPane),
        "port_forward" => Some(Command::PortForward),
        "view_logs" => Some(Command::ViewLogs),
        _ => None,
    }
}

pub(super) fn interact_command_description(name: &str) -> String {
    match name {
        "exec" => "Exec",
        "open_query" => "Query DB",
        "port_forward" => "Port Forward",
        "view_logs" => "Logs",
        _ => "Unknown",
    }
    .into()
}

pub(super) fn browse_command_from_name(name: &str) -> Option<Command> {
    match name {
        "view_yaml" => Some(Command::ViewYaml),
        "view_describe" => Some(Command::ViewDescribe),
        "view_logs" => Some(Command::ViewLogs),
        "save_logs" => Some(Command::SaveLogsToFile),
        "download_logs" => Some(Command::DownloadFullLogs),
        "filter" => Some(Command::EnterMode(InputMode::FilterInput)),
        "resource_switcher" => Some(Command::EnterResourceSwitcher),
        "sort_column" => Some(Command::SortByColumn),
        "toggle_sort_order" => Some(Command::Pane(PaneCommand::ToggleSortOrder)),
        "toggle_all_namespaces" => Some(Command::ToggleAllNamespaces),
        "toggle_follow" => Some(Command::Pane(PaneCommand::ToggleFollow)),
        "toggle_wrap" => Some(Command::Pane(PaneCommand::ToggleWrap)),
        _ => None,
    }
}

pub(super) fn browse_command_description(name: &str) -> String {
    match name {
        "view_yaml" => "View YAML",
        "view_describe" => "Describe",
        "view_logs" => "Logs",
        "save_logs" => "Save Logs",
        "download_logs" => "Download All Logs",
        "filter" => "Filter",
        "resource_switcher" => "Resources",
        "sort_column" => "Sort",
        "toggle_sort_order" => "Sort Order",
        "toggle_all_namespaces" => "All NS",
        "toggle_follow" => "Follow",
        "toggle_wrap" => "Wrap",
        _ => "Unknown",
    }
    .into()
}

pub(super) fn navigation_command_from_name(name: &str) -> Option<Command> {
    match name {
        "scroll_up" | "select_prev" => Some(Command::Pane(PaneCommand::SelectPrev)),
        "scroll_down" | "select_next" => Some(Command::Pane(PaneCommand::SelectNext)),
        "select" => Some(Command::Pane(PaneCommand::Select)),
        "back" => Some(Command::Pane(PaneCommand::Back)),
        "go_to_top" => Some(Command::Pane(PaneCommand::GoToTop)),
        "go_to_bottom" => Some(Command::Pane(PaneCommand::GoToBottom)),
        "page_up" | "page_up_key" => Some(Command::Pane(PaneCommand::PageUp)),
        "page_down" | "page_down_key" => Some(Command::Pane(PaneCommand::PageDown)),
        "scroll_left" => Some(Command::Pane(PaneCommand::ScrollLeft)),
        "scroll_right" => Some(Command::Pane(PaneCommand::ScrollRight)),
        _ => None,
    }
}

pub(super) fn navigation_command_description(name: &str) -> String {
    match name {
        "scroll_up" | "select_prev" => "Up",
        "scroll_down" | "select_next" => "Down",
        "select" => "Select",
        "back" => "Back",
        "go_to_top" => "Go to top",
        "go_to_bottom" => "Go to bottom",
        "page_up" | "page_up_key" => "Page up",
        "page_down" | "page_down_key" => "Page down",
        "scroll_left" => "Left",
        "scroll_right" => "Right",
        _ => "Unknown",
    }
    .into()
}

pub(super) fn tui_command_from_name(name: &str) -> Option<Command> {
    match name {
        "split_vertical" => Some(Command::SplitVertical),
        "split_horizontal" => Some(Command::SplitHorizontal),
        "close_pane" => Some(Command::ClosePane),
        "toggle_fullscreen" => Some(Command::ToggleFullscreen),
        "focus_up" => Some(Command::FocusDirection(Direction::Up)),
        "focus_down" => Some(Command::FocusDirection(Direction::Down)),
        "focus_left" => Some(Command::FocusDirection(Direction::Left)),
        "focus_right" => Some(Command::FocusDirection(Direction::Right)),
        "resize_grow" => Some(Command::ResizeGrow),
        "resize_shrink" => Some(Command::ResizeShrink),
        "new_tab" => Some(Command::NewTab),
        "close_tab" => Some(Command::CloseTab),
        "open_terminal" => Some(Command::TerminalSpawn),
        "focus_next" => Some(Command::FocusNextPane),
        "focus_prev" => Some(Command::FocusPrevPane),
        s if s.starts_with("goto_tab_") => s["goto_tab_".len()..].parse::<usize>().ok().map(Command::GoToTab),
        _ => None,
    }
}

pub(super) fn tui_command_description(name: &str) -> String {
    match name {
        "split_vertical" => "Split V",
        "split_horizontal" => "Split H",
        "close_pane" => "Close pane",
        "toggle_fullscreen" => "Fullscreen",
        "focus_up" => "Focus up",
        "focus_down" => "Focus down",
        "focus_left" => "Focus left",
        "focus_right" => "Focus right",
        "resize_grow" => "Grow",
        "resize_shrink" => "Shrink",
        "new_tab" => "New tab",
        "close_tab" => "Close tab",
        "open_terminal" => "Terminal",
        "focus_next" => "Focus next",
        "focus_prev" => "Focus prev",
        s if s.starts_with("goto_tab_") => "Go to tab",
        _ => "Unknown",
    }
    .into()
}
