use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};

use kubetile_config::KeybindingsConfig;
use kubetile_tui::pane::PaneCommand;

use crate::command::Command;

mod commands;
mod parsing;

pub use parsing::parse_key_string;

use commands::{
    browse_command_description, browse_command_from_name, completion_command_description, completion_command_from_name,
    global_command_description, global_command_from_name, interact_command_description, interact_command_from_name,
    mutate_command_description, mutate_command_from_name, navigation_command_description, navigation_command_from_name,
    query_browse_command_description, query_browse_command_from_name, query_editor_command_description,
    query_editor_command_from_name, query_history_command_description, query_history_command_from_name,
    saved_queries_command_description, saved_queries_command_from_name, tui_command_description, tui_command_from_name,
};
use parsing::{format_key_display, key_to_input_string, normalize_key_event};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum InputMode {
    Normal,
    Pane,
    Tab,
    Search,
    Command,
    Insert,
    NamespaceSelector,
    ContextSelector,
    ResourceSwitcher,
    ConfirmDialog,
    FilterInput,
    PortForwardInput,
    QueryDialog,
    QueryEditor,
    QueryBrowse,
    QueryHistory,
    SaveQueryName,
    SavedQueries,
    ExportDialog,
    Completion,
    PaneHelp,
}

#[allow(dead_code)]
pub struct KeybindingDispatcher {
    mode: InputMode,
    global_bindings: HashMap<KeyEvent, Command>,
    mutate_bindings: HashMap<KeyEvent, Command>,
    interact_bindings: HashMap<KeyEvent, Command>,
    browse_bindings: HashMap<KeyEvent, Command>,
    navigation_bindings: HashMap<KeyEvent, Command>,
    tui_bindings: HashMap<KeyEvent, Command>,
    query_editor_bindings: HashMap<KeyEvent, Command>,
    query_browse_bindings: HashMap<KeyEvent, Command>,
    query_history_bindings: HashMap<KeyEvent, Command>,
    saved_queries_bindings: HashMap<KeyEvent, Command>,
    completion_bindings: HashMap<KeyEvent, Command>,
    reverse_global: Vec<(String, String, String)>,
    reverse_mutate: Vec<(String, String, String)>,
    reverse_interact: Vec<(String, String, String)>,
    reverse_browse: Vec<(String, String, String)>,
    reverse_navigation: Vec<(String, String, String)>,
    reverse_tui: Vec<(String, String, String)>,
    reverse_query_editor: Vec<(String, String, String)>,
    reverse_query_browse: Vec<(String, String, String)>,
    reverse_query_history: Vec<(String, String, String)>,
    reverse_saved_queries: Vec<(String, String, String)>,
    reverse_completion: Vec<(String, String, String)>,
}

type GroupResult = (HashMap<KeyEvent, Command>, Vec<(String, String, String)>);

impl KeybindingDispatcher {
    pub fn from_config(config: &KeybindingsConfig) -> Self {
        fn build_group<'a, I, F, G>(entries: I, from_name: F, description: G) -> GroupResult
        where
            I: IntoIterator<Item = (&'a String, &'a String)>,
            F: Fn(&str) -> Option<Command>,
            G: Fn(&str) -> String,
        {
            let mut bindings = HashMap::new();
            let mut reverse = Vec::new();
            for (name, key_str) in entries {
                if let Some(cmd) = from_name(name) {
                    if let Some(key) = parse_key_string(key_str) {
                        bindings.insert(key, cmd);
                        reverse.push((name.clone(), key_str.clone(), description(name)));
                    }
                }
            }
            (bindings, reverse)
        }

        let (global_bindings, reverse_global) =
            build_group(config.global.iter(), global_command_from_name, global_command_description);
        let (mutate_bindings, reverse_mutate) =
            build_group(config.mutate.iter(), mutate_command_from_name, mutate_command_description);
        let (interact_bindings, reverse_interact) =
            build_group(config.interact.iter(), interact_command_from_name, interact_command_description);
        let (browse_bindings, reverse_browse) =
            build_group(config.browse.iter(), browse_command_from_name, browse_command_description);
        let (navigation_bindings, reverse_navigation) =
            build_group(config.navigation.iter(), navigation_command_from_name, navigation_command_description);
        let (tui_bindings, reverse_tui) =
            build_group(config.tui.iter(), tui_command_from_name, tui_command_description);
        let (query_editor_bindings, reverse_query_editor) =
            build_group(config.query_editor.iter(), query_editor_command_from_name, query_editor_command_description);
        let (query_browse_bindings, reverse_query_browse) =
            build_group(config.query_browse.iter(), query_browse_command_from_name, query_browse_command_description);
        let (query_history_bindings, reverse_query_history) = build_group(
            config.query_history.iter(),
            query_history_command_from_name,
            query_history_command_description,
        );
        let (saved_queries_bindings, reverse_saved_queries) = build_group(
            config.saved_queries.iter(),
            saved_queries_command_from_name,
            saved_queries_command_description,
        );
        let (completion_bindings, reverse_completion) =
            build_group(config.completion.iter(), completion_command_from_name, completion_command_description);

        Self {
            mode: InputMode::Normal,
            global_bindings,
            mutate_bindings,
            interact_bindings,
            browse_bindings,
            navigation_bindings,
            tui_bindings,
            query_editor_bindings,
            query_browse_bindings,
            query_history_bindings,
            saved_queries_bindings,
            completion_bindings,
            reverse_global,
            reverse_mutate,
            reverse_interact,
            reverse_browse,
            reverse_navigation,
            reverse_tui,
            reverse_query_editor,
            reverse_query_browse,
            reverse_query_history,
            reverse_saved_queries,
            reverse_completion,
        }
    }

    pub fn dispatch(&self, key: KeyEvent) -> Option<(Command, bool)> {
        let key = normalize_key_event(key);

        match self.mode {
            InputMode::Insert => {
                if key.code == KeyCode::Esc {
                    return Some((Command::ExitMode, false));
                }
                let s = key_to_input_string(key);
                if s.is_empty() {
                    return None;
                }
                return Some((Command::Pane(PaneCommand::SendInput(s)), false));
            }
            InputMode::ResourceSwitcher => match key.code {
                KeyCode::Enter => return Some((Command::ResourceSwitcherConfirm, false)),
                KeyCode::Esc => return Some((Command::DenyAction, false)),
                KeyCode::Up => return Some((Command::Pane(PaneCommand::SelectPrev), false)),
                KeyCode::Down => return Some((Command::Pane(PaneCommand::SelectNext), false)),
                KeyCode::Char(c) => return Some((Command::ResourceSwitcherInput(c), false)),
                KeyCode::Backspace => return Some((Command::ResourceSwitcherBackspace, false)),
                _ => return None,
            },
            InputMode::ConfirmDialog => match key.code {
                KeyCode::Char('y') => return Some((Command::ConfirmAction, false)),
                KeyCode::Char('n') | KeyCode::Esc => return Some((Command::DenyAction, false)),
                _ => return None,
            },
            InputMode::FilterInput => match key.code {
                KeyCode::Esc => return Some((Command::FilterCancel, false)),
                KeyCode::Enter => return Some((Command::ExitMode, false)),
                KeyCode::Char(c) => return Some((Command::FilterInput(c), false)),
                KeyCode::Backspace => return Some((Command::FilterBackspace, false)),
                _ => return None,
            },
            InputMode::PortForwardInput => match key.code {
                KeyCode::Esc => return Some((Command::PortForwardCancel, false)),
                KeyCode::Enter => return Some((Command::PortForwardConfirm, false)),
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                    return Some((Command::PortForwardToggleField, false));
                }
                KeyCode::Char(c) if c.is_ascii_digit() => return Some((Command::PortForwardInput(c), false)),
                KeyCode::Backspace => return Some((Command::PortForwardBackspace, false)),
                _ => return None,
            },
            InputMode::QueryEditor => {
                // Configurable action bindings take precedence.
                if let Some(cmd) = self.query_editor_bindings.get(&key) {
                    return Some((cmd.clone(), false));
                }
                // Non-char keys (function keys, etc.) pass through to global bindings.
                if !matches!(key.code, KeyCode::Char(_)) {
                    if let Some(cmd) = self.global_bindings.get(&key) {
                        return Some((cmd.clone(), false));
                    }
                }
                // Hardcoded raw input / cursor movement (not configurable).
                match (key.code, key.modifiers) {
                    (KeyCode::Enter, _) => return Some((Command::QueryEditorNewLine, false)),
                    (KeyCode::Char(c), _) => return Some((Command::QueryEditorInput(c), false)),
                    (KeyCode::Backspace, _) => return Some((Command::QueryEditorBackspace, false)),
                    (KeyCode::Up, _) => return Some((Command::QueryEditorCursorUp, false)),
                    (KeyCode::Down, _) => return Some((Command::QueryEditorCursorDown, false)),
                    (KeyCode::Left, _) => return Some((Command::QueryEditorCursorLeft, false)),
                    (KeyCode::Right, _) => return Some((Command::QueryEditorCursorRight, false)),
                    (KeyCode::Home, _) => return Some((Command::QueryEditorHome, false)),
                    (KeyCode::End, _) => return Some((Command::QueryEditorEnd, false)),
                    (KeyCode::PageUp, _) => return Some((Command::QueryEditorScrollDown, false)),
                    (KeyCode::PageDown, _) => return Some((Command::QueryEditorScrollUp, false)),
                    _ => return None,
                }
            }
            InputMode::QueryBrowse => {
                // Configurable action bindings take precedence.
                if let Some(cmd) = self.query_browse_bindings.get(&key) {
                    return Some((cmd.clone(), false));
                }
                // Non-char keys pass through to global bindings.
                if !matches!(key.code, KeyCode::Char(_)) {
                    if let Some(cmd) = self.global_bindings.get(&key) {
                        return Some((cmd.clone(), false));
                    }
                }
                // Hardcoded aliases: arrow keys and Enter as universal fallbacks.
                match (key.code, key.modifiers) {
                    (KeyCode::Down, _) => return Some((Command::QueryBrowseNext, false)),
                    (KeyCode::Up, _) => return Some((Command::QueryBrowsePrev, false)),
                    (KeyCode::Left, _) => return Some((Command::QueryBrowseScrollLeft, false)),
                    (KeyCode::Right, _) => return Some((Command::QueryBrowseScrollRight, false)),
                    (KeyCode::Enter, _) => return Some((Command::EnterMode(InputMode::QueryEditor), false)),
                    (KeyCode::PageDown, _) => return Some((Command::QueryEditorScrollUp, false)),
                    (KeyCode::PageUp, _) => return Some((Command::QueryEditorScrollDown, false)),
                    _ => return None,
                }
            }
            InputMode::QueryHistory => {
                if let Some(cmd) = self.query_history_bindings.get(&key) {
                    return Some((cmd.clone(), false));
                }
                // Arrow key aliases.
                match key.code {
                    KeyCode::Down => return Some((Command::QueryHistoryNext, false)),
                    KeyCode::Up => return Some((Command::QueryHistoryPrev, false)),
                    _ => return None,
                }
            }
            InputMode::ExportDialog => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => return Some((Command::ExportDialogCancel, false)),
                (KeyCode::Enter, _) => return Some((Command::ExportDialogConfirm, false)),
                (KeyCode::Char(c), _) => return Some((Command::ExportDialogInput(c), false)),
                (KeyCode::Backspace, _) => return Some((Command::ExportDialogBackspace, false)),
                _ => return None,
            },
            InputMode::SaveQueryName => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => return Some((Command::SaveQueryNameCancel, false)),
                (KeyCode::Enter, _) => return Some((Command::SaveQueryNameConfirm, false)),
                (KeyCode::Char(c), _) => return Some((Command::SaveQueryNameInput(c), false)),
                (KeyCode::Backspace, _) => return Some((Command::SaveQueryNameBackspace, false)),
                _ => return None,
            },
            InputMode::SavedQueries => {
                if let Some(cmd) = self.saved_queries_bindings.get(&key) {
                    return Some((cmd.clone(), false));
                }
                // Arrow key aliases and raw text input.
                match (key.code, key.modifiers) {
                    (KeyCode::Down, _) => return Some((Command::SavedQueriesNext, false)),
                    (KeyCode::Up, _) => return Some((Command::SavedQueriesPrev, false)),
                    (KeyCode::Char(c), _) => return Some((Command::SavedQueriesInput(c), false)),
                    (KeyCode::Backspace, _) => return Some((Command::SavedQueriesBackspace, false)),
                    _ => return None,
                }
            }
            InputMode::Completion => {
                if let Some(cmd) = self.completion_bindings.get(&key) {
                    return Some((cmd.clone(), false));
                }
                // Arrow key aliases, Tab accept alias, and raw text input.
                match (key.code, key.modifiers) {
                    (KeyCode::Tab, _) => return Some((Command::CompleteAccept, false)),
                    (KeyCode::Up, _) => return Some((Command::CompletePrev, false)),
                    (KeyCode::Down, _) => return Some((Command::CompleteNext, false)),
                    (KeyCode::Char(c), _) => return Some((Command::CompleteInput(c), false)),
                    (KeyCode::Backspace, _) => return Some((Command::CompleteBackspace, false)),
                    _ => return None,
                }
            }
            InputMode::PaneHelp => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Some((Command::ClosePaneHelp, false)),
                _ => return None,
            },
            InputMode::QueryDialog => match key.code {
                KeyCode::Esc => return Some((Command::QueryDialogCancel, false)),
                KeyCode::Enter => return Some((Command::QueryDialogConfirm, false)),
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Up | KeyCode::Down => {
                    return Some((Command::QueryDialogNextField, false));
                }
                KeyCode::Char(c) => return Some((Command::QueryDialogInput(c), false)),
                KeyCode::Backspace => return Some((Command::QueryDialogBackspace, false)),
                _ => return None,
            },
            _ => {}
        }

        if let Some(cmd) = self.global_bindings.get(&key) {
            return Some((cmd.clone(), false));
        }

        match self.mode {
            InputMode::Insert => unreachable!("handled above"),
            InputMode::Normal => {
                if let Some(cmd) = self.mutate_bindings.get(&key) {
                    return Some((cmd.clone(), true));
                }
                self.interact_bindings
                    .get(&key)
                    .or_else(|| self.browse_bindings.get(&key))
                    .or_else(|| self.navigation_bindings.get(&key))
                    .or_else(|| self.tui_bindings.get(&key))
                    .cloned()
                    .map(|cmd| (cmd, false))
            }
            InputMode::NamespaceSelector => match key.code {
                KeyCode::Enter => Some((Command::NamespaceConfirm, false)),
                KeyCode::Esc => Some((Command::ExitMode, false)),
                KeyCode::Up => Some((Command::Pane(PaneCommand::SelectPrev), false)),
                KeyCode::Down => Some((Command::Pane(PaneCommand::SelectNext), false)),
                KeyCode::Char(c) => Some((Command::NamespaceInput(c), false)),
                KeyCode::Backspace => Some((Command::NamespaceBackspace, false)),
                _ => None,
            },
            InputMode::ContextSelector => match key.code {
                KeyCode::Enter => Some((Command::ContextConfirm, false)),
                KeyCode::Esc => Some((Command::ExitMode, false)),
                KeyCode::Up => Some((Command::Pane(PaneCommand::SelectPrev), false)),
                KeyCode::Down => Some((Command::Pane(PaneCommand::SelectNext), false)),
                KeyCode::Char(c) => Some((Command::ContextInput(c), false)),
                KeyCode::Backspace => Some((Command::ContextBackspace, false)),
                _ => None,
            },
            InputMode::Search | InputMode::Command => None,
            InputMode::Pane | InputMode::Tab => None,
            InputMode::ResourceSwitcher
            | InputMode::ConfirmDialog
            | InputMode::FilterInput
            | InputMode::PortForwardInput
            | InputMode::QueryDialog
            | InputMode::QueryEditor
            | InputMode::QueryBrowse
            | InputMode::QueryHistory
            | InputMode::SaveQueryName
            | InputMode::SavedQueries
            | InputMode::ExportDialog
            | InputMode::Completion
            | InputMode::PaneHelp => {
                unreachable!("handled above")
            }
        }
    }

    #[allow(dead_code)]
    pub fn all_bindings(&self) -> Vec<(String, String, String)> {
        let mut result = Vec::new();
        fn collect(result: &mut Vec<(String, String, String)>, group: &str, reverse: &[(String, String, String)]) {
            for (_, key_str, desc) in reverse {
                result.push((group.to_string(), format_key_display(key_str), desc.clone()));
            }
        }
        collect(&mut result, "Global", &self.reverse_global);
        collect(&mut result, "Mutate", &self.reverse_mutate);
        collect(&mut result, "Interact", &self.reverse_interact);
        collect(&mut result, "Browse", &self.reverse_browse);
        collect(&mut result, "Navigation", &self.reverse_navigation);
        collect(&mut result, "TUI", &self.reverse_tui);
        result
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }

    pub fn mode(&self) -> InputMode {
        self.mode
    }

    pub fn key_for(&self, name: &str) -> Option<String> {
        let all: Vec<_> = self
            .reverse_global
            .iter()
            .chain(&self.reverse_tui)
            .chain(&self.reverse_browse)
            .chain(&self.reverse_interact)
            .chain(&self.reverse_navigation)
            .chain(&self.reverse_mutate)
            .collect();
        all.iter().find(|(n, _, _)| n == name).map(|(_, key_str, _)| format_key_display(key_str))
    }

    #[allow(dead_code)]
    pub fn key_for_mode(&self, group: &str, name: &str) -> Option<String> {
        let reverse: &[(String, String, String)] = match group {
            "query_editor" => &self.reverse_query_editor,
            "query_browse" => &self.reverse_query_browse,
            "query_history" => &self.reverse_query_history,
            "saved_queries" => &self.reverse_saved_queries,
            "completion" => &self.reverse_completion,
            _ => return None,
        };
        reverse.iter().find(|(n, _, _)| n == name).map(|(_, key_str, _)| format_key_display(key_str))
    }

    pub fn global_shortcuts(&self) -> Vec<(String, String)> {
        self.reverse_global.iter().map(|(_, key_str, desc)| (format_key_display(key_str), desc.clone())).collect()
    }

    pub fn navigation_shortcuts(&self) -> Vec<(String, String)> {
        self.reverse_navigation.iter().map(|(_, key_str, desc)| (format_key_display(key_str), desc.clone())).collect()
    }

    pub fn browse_shortcuts(&self) -> Vec<(String, String)> {
        self.reverse_browse.iter().map(|(_, key_str, desc)| (format_key_display(key_str), desc.clone())).collect()
    }

    pub fn tui_shortcuts(&self) -> Vec<(String, String)> {
        self.reverse_tui.iter().map(|(_, key_str, desc)| (format_key_display(key_str), desc.clone())).collect()
    }

    pub fn interact_shortcuts(&self) -> Vec<(String, String)> {
        self.reverse_interact.iter().map(|(_, key_str, desc)| (format_key_display(key_str), desc.clone())).collect()
    }

    pub fn mutate_shortcuts(&self) -> Vec<(String, String)> {
        self.reverse_mutate.iter().map(|(_, key_str, desc)| (format_key_display(key_str), desc.clone())).collect()
    }

    pub fn query_editor_shortcuts(&self) -> Vec<(String, String)> {
        self.reverse_query_editor.iter().map(|(_, key_str, desc)| (format_key_display(key_str), desc.clone())).collect()
    }

    pub fn query_browse_shortcuts(&self) -> Vec<(String, String)> {
        self.reverse_query_browse.iter().map(|(_, key_str, desc)| (format_key_display(key_str), desc.clone())).collect()
    }

    #[allow(dead_code)]
    pub fn query_history_shortcuts(&self) -> Vec<(String, String)> {
        self.reverse_query_history
            .iter()
            .map(|(_, key_str, desc)| (format_key_display(key_str), desc.clone()))
            .collect()
    }

    #[allow(dead_code)]
    pub fn saved_queries_shortcuts(&self) -> Vec<(String, String)> {
        self.reverse_saved_queries
            .iter()
            .map(|(_, key_str, desc)| (format_key_display(key_str), desc.clone()))
            .collect()
    }

    #[allow(dead_code)]
    pub fn completion_shortcuts(&self) -> Vec<(String, String)> {
        self.reverse_completion.iter().map(|(_, key_str, desc)| (format_key_display(key_str), desc.clone())).collect()
    }
}

#[cfg(test)]
mod tests;
