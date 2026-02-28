use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use kubetile_config::KeybindingsConfig;
use kubetile_tui::pane::PaneCommand;

use crate::command::Command;

mod commands;
mod parsing;

pub use parsing::parse_key_string;

use commands::{
    browse_command_description, browse_command_from_name, global_command_description, global_command_from_name,
    interact_command_description, interact_command_from_name, mutate_command_description, mutate_command_from_name,
    navigation_command_description, navigation_command_from_name, tui_command_description, tui_command_from_name,
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
    reverse_global: Vec<(String, String, String)>,
    reverse_mutate: Vec<(String, String, String)>,
    reverse_interact: Vec<(String, String, String)>,
    reverse_browse: Vec<(String, String, String)>,
    reverse_navigation: Vec<(String, String, String)>,
    reverse_tui: Vec<(String, String, String)>,
}

impl KeybindingDispatcher {
    pub fn from_config(config: &KeybindingsConfig) -> Self {
        let mut global_bindings = HashMap::new();
        let mut reverse_global = Vec::new();
        for (name, key_str) in &config.global {
            if let Some(cmd) = global_command_from_name(name) {
                if let Some(key) = parse_key_string(key_str) {
                    global_bindings.insert(key, cmd);
                    reverse_global.push((name.clone(), key_str.clone(), global_command_description(name)));
                }
            }
        }

        let mut mutate_bindings = HashMap::new();
        let mut reverse_mutate = Vec::new();
        for (name, key_str) in &config.mutate {
            if let Some(cmd) = mutate_command_from_name(name) {
                if let Some(key) = parse_key_string(key_str) {
                    mutate_bindings.insert(key, cmd);
                    reverse_mutate.push((name.clone(), key_str.clone(), mutate_command_description(name)));
                }
            }
        }

        let mut interact_bindings = HashMap::new();
        let mut reverse_interact = Vec::new();
        for (name, key_str) in &config.interact {
            if let Some(cmd) = interact_command_from_name(name) {
                if let Some(key) = parse_key_string(key_str) {
                    interact_bindings.insert(key, cmd);
                    reverse_interact.push((name.clone(), key_str.clone(), interact_command_description(name)));
                }
            }
        }

        let mut browse_bindings = HashMap::new();
        let mut reverse_browse = Vec::new();
        for (name, key_str) in &config.browse {
            if let Some(cmd) = browse_command_from_name(name) {
                if let Some(key) = parse_key_string(key_str) {
                    browse_bindings.insert(key, cmd);
                    reverse_browse.push((name.clone(), key_str.clone(), browse_command_description(name)));
                }
            }
        }

        let mut navigation_bindings = HashMap::new();
        let mut reverse_navigation = Vec::new();
        for (name, key_str) in &config.navigation {
            if let Some(cmd) = navigation_command_from_name(name) {
                if let Some(key) = parse_key_string(key_str) {
                    navigation_bindings.insert(key, cmd);
                    reverse_navigation.push((name.clone(), key_str.clone(), navigation_command_description(name)));
                }
            }
        }

        let mut tui_bindings = HashMap::new();
        let mut reverse_tui = Vec::new();
        for (name, key_str) in &config.tui {
            if let Some(cmd) = tui_command_from_name(name) {
                if let Some(key) = parse_key_string(key_str) {
                    tui_bindings.insert(key, cmd);
                    reverse_tui.push((name.clone(), key_str.clone(), tui_command_description(name)));
                }
            }
        }

        Self {
            mode: InputMode::Normal,
            global_bindings,
            mutate_bindings,
            interact_bindings,
            browse_bindings,
            navigation_bindings,
            tui_bindings,
            reverse_global,
            reverse_mutate,
            reverse_interact,
            reverse_browse,
            reverse_navigation,
            reverse_tui,
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
            InputMode::QueryEditor => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => return Some((Command::ExitMode, false)),
                (KeyCode::Enter, KeyModifiers::CONTROL) => return Some((Command::QueryEditorExecute, false)),
                (KeyCode::Enter, _) => return Some((Command::QueryEditorNewLine, false)),
                (KeyCode::Tab, _) => return Some((Command::QueryEditorIndent, false)),
                (KeyCode::BackTab, _) => return Some((Command::QueryEditorDeIndent, false)),
                (KeyCode::Char('r'), KeyModifiers::CONTROL) => return Some((Command::OpenQueryHistory, false)),
                (KeyCode::Char('s'), KeyModifiers::CONTROL) => return Some((Command::OpenSaveQueryDialog, false)),
                (KeyCode::Char('o'), KeyModifiers::CONTROL) => return Some((Command::OpenSavedQueries, false)),
                (KeyCode::Down, KeyModifiers::CONTROL) => return Some((Command::EnterQueryBrowse, false)),
                (KeyCode::Char(' '), KeyModifiers::CONTROL) => return Some((Command::TriggerCompletion, false)),
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
            },
            InputMode::QueryBrowse => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => return Some((Command::ExitMode, false)),
                (KeyCode::Char('i'), _) | (KeyCode::Enter, _) => {
                    return Some((Command::EnterMode(InputMode::QueryEditor), false))
                }
                (KeyCode::Up, KeyModifiers::CONTROL) => {
                    return Some((Command::EnterMode(InputMode::QueryEditor), false))
                }
                (KeyCode::Char('j'), _) => return Some((Command::QueryBrowseNext, false)),
                (KeyCode::Char('k'), _) => return Some((Command::QueryBrowsePrev, false)),
                (KeyCode::Char('h'), _) | (KeyCode::Left, _) => return Some((Command::QueryBrowseScrollLeft, false)),
                (KeyCode::Char('l'), _) | (KeyCode::Right, _) => return Some((Command::QueryBrowseScrollRight, false)),
                (KeyCode::PageDown, _) => return Some((Command::QueryEditorScrollUp, false)),
                (KeyCode::PageUp, _) => return Some((Command::QueryEditorScrollDown, false)),
                (KeyCode::Char('y'), _) => return Some((Command::QueryCopyRow, false)),
                (KeyCode::Char('Y'), _) => return Some((Command::QueryCopyAll, false)),
                (KeyCode::Char('E'), _) => return Some((Command::OpenExportDialog, false)),
                _ => return None,
            },
            InputMode::QueryHistory => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => return Some((Command::CloseQueryHistory, false)),
                (KeyCode::Enter, _) => return Some((Command::QueryHistorySelect, false)),
                (KeyCode::Char('j'), _) | (KeyCode::Down, _) => return Some((Command::QueryHistoryNext, false)),
                (KeyCode::Char('k'), _) | (KeyCode::Up, _) => return Some((Command::QueryHistoryPrev, false)),
                (KeyCode::Char('d'), _) => return Some((Command::QueryHistoryDelete, false)),
                _ => return None,
            },
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
            InputMode::SavedQueries => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => return Some((Command::SavedQueriesClose, false)),
                (KeyCode::Enter, _) => return Some((Command::SavedQueriesSelect, false)),
                (KeyCode::Char('j'), _) | (KeyCode::Down, _) => return Some((Command::SavedQueriesNext, false)),
                (KeyCode::Char('k'), _) | (KeyCode::Up, _) => return Some((Command::SavedQueriesPrev, false)),
                (KeyCode::Char('d'), _) => return Some((Command::SavedQueriesDelete, false)),
                (KeyCode::Char('e'), _) => return Some((Command::SavedQueriesStartRename, false)),
                (KeyCode::Char('/'), _) => return Some((Command::SavedQueriesStartFilter, false)),
                (KeyCode::Char(c), _) => return Some((Command::SavedQueriesInput(c), false)),
                (KeyCode::Backspace, _) => return Some((Command::SavedQueriesBackspace, false)),
                _ => return None,
            },
            InputMode::Completion => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => return Some((Command::CompleteDismiss, false)),
                (KeyCode::Enter, _) | (KeyCode::Tab, _) => return Some((Command::CompleteAccept, false)),
                (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                    return Some((Command::CompletePrev, false))
                }
                (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                    return Some((Command::CompleteNext, false))
                }
                (KeyCode::Char(c), _) => return Some((Command::CompleteInput(c), false)),
                (KeyCode::Backspace, _) => return Some((Command::CompleteBackspace, false)),
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
            | InputMode::Completion => {
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
}

#[cfg(test)]
mod tests;
