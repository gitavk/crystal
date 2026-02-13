use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crystal_config::KeybindingsConfig;
use crystal_tui::pane::{Direction, PaneCommand};

use crate::command::Command;

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
    ResourceSwitcher,
    ConfirmDialog,
    FilterInput,
}

#[allow(dead_code)]
pub struct KeybindingDispatcher {
    mode: InputMode,
    global_bindings: HashMap<KeyEvent, Command>,
    pane_bindings: HashMap<KeyEvent, Command>,
    resource_bindings: HashMap<KeyEvent, Command>,
    reverse_global: Vec<(String, String, String)>,
    reverse_pane: Vec<(String, String, String)>,
    reverse_resource: Vec<(String, String, String)>,
}

impl KeybindingDispatcher {
    pub fn from_config(config: &KeybindingsConfig) -> Self {
        let mut global_bindings = HashMap::new();
        let mut reverse_global = Vec::new();

        for (name, key_str) in &config.global {
            if let Some(cmd) = command_from_name(name) {
                if let Some(key) = parse_key_string(key_str) {
                    global_bindings.insert(key, cmd);
                    reverse_global.push((name.clone(), key_str.clone(), command_description(name)));
                }
            }
        }

        let mut pane_bindings = HashMap::new();
        let mut reverse_pane = Vec::new();

        for (name, key_str) in &config.pane {
            if let Some(cmd) = pane_command_from_name(name) {
                if let Some(key) = parse_key_string(key_str) {
                    pane_bindings.insert(key, Command::Pane(cmd));
                    reverse_pane.push((name.clone(), key_str.clone(), pane_command_description(name)));
                }
            }
        }

        let mut resource_bindings = HashMap::new();
        let mut reverse_resource = Vec::new();

        for (name, key_str) in &config.resource {
            if let Some(cmd) = resource_command_from_name(name) {
                if let Some(key) = parse_key_string(key_str) {
                    resource_bindings.insert(key, cmd);
                    reverse_resource.push((name.clone(), key_str.clone(), resource_command_description(name)));
                }
            }
        }

        Self {
            mode: InputMode::Normal,
            global_bindings,
            pane_bindings,
            resource_bindings,
            reverse_global,
            reverse_pane,
            reverse_resource,
        }
    }

    pub fn dispatch(&self, key: KeyEvent) -> Option<Command> {
        match self.mode {
            InputMode::ResourceSwitcher => match key.code {
                KeyCode::Enter => return Some(Command::ResourceSwitcherConfirm),
                KeyCode::Esc => return Some(Command::ExitMode),
                KeyCode::Char(c) => return Some(Command::ResourceSwitcherInput(c)),
                KeyCode::Backspace => return Some(Command::ResourceSwitcherBackspace),
                _ => return None,
            },
            InputMode::ConfirmDialog => match key.code {
                KeyCode::Char('y') => return Some(Command::ConfirmAction),
                KeyCode::Char('n') | KeyCode::Esc => return Some(Command::DenyAction),
                _ => return None,
            },
            InputMode::FilterInput => match key.code {
                KeyCode::Esc => return Some(Command::ExitMode),
                KeyCode::Enter => return Some(Command::ExitMode),
                KeyCode::Char(c) => return Some(Command::Pane(PaneCommand::SearchInput(c))),
                KeyCode::Backspace => return Some(Command::Pane(PaneCommand::ClearFilter)),
                _ => return None,
            },
            _ => {}
        }

        if let Some(cmd) = self.global_bindings.get(&key) {
            return Some(cmd.clone());
        }

        match self.mode {
            InputMode::Insert => Some(Command::Pane(PaneCommand::SendInput(key_to_input_string(key)))),
            InputMode::Normal => {
                self.resource_bindings.get(&key).cloned().or_else(|| self.pane_bindings.get(&key).cloned())
            }
            InputMode::NamespaceSelector => match key.code {
                KeyCode::Enter => Some(Command::NamespaceConfirm),
                KeyCode::Esc => Some(Command::ExitMode),
                KeyCode::Up => Some(Command::Pane(PaneCommand::SelectPrev)),
                KeyCode::Down => Some(Command::Pane(PaneCommand::SelectNext)),
                KeyCode::Char(c) => Some(Command::NamespaceInput(c)),
                KeyCode::Backspace => Some(Command::NamespaceBackspace),
                _ => None,
            },
            InputMode::Search | InputMode::Command => None,
            InputMode::Pane | InputMode::Tab => None,
            InputMode::ResourceSwitcher | InputMode::ConfirmDialog | InputMode::FilterInput => {
                unreachable!("handled above")
            }
        }
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
    }

    pub fn mode(&self) -> InputMode {
        self.mode
    }

    pub fn global_hints(&self) -> Vec<(String, String)> {
        let priority =
            ["split_vertical", "split_horizontal", "focus_next", "close_pane", "namespace_selector", "help", "quit"];
        let mut hints = Vec::new();
        for name in &priority {
            if let Some((_, key_str, desc)) = self.reverse_global.iter().find(|(n, _, _)| n == name) {
                hints.push((key_str.clone(), desc.clone()));
            }
        }
        hints
    }

    pub fn global_shortcuts(&self) -> Vec<(String, String)> {
        let mut sorted = self.reverse_global.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted.into_iter().map(|(_, key_str, desc)| (format_key_display(&key_str), desc)).collect()
    }

    pub fn pane_shortcuts(&self) -> Vec<(String, String)> {
        let mut sorted = self.reverse_pane.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted.into_iter().map(|(_, key_str, desc)| (format_key_display(&key_str), desc)).collect()
    }

    #[allow(dead_code)]
    pub fn resource_shortcuts(&self) -> Vec<(String, String)> {
        let mut sorted = self.reverse_resource.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted.into_iter().map(|(_, key_str, desc)| (format_key_display(&key_str), desc)).collect()
    }
}

fn format_key_display(key_str: &str) -> String {
    key_str
        .split('+')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    format!("{upper}{}", chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

fn key_to_input_string(key: KeyEvent) -> String {
    match key.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "\n".to_string(),
        KeyCode::Tab => "\t".to_string(),
        KeyCode::Backspace => "\x08".to_string(),
        _ => String::new(),
    }
}

pub fn parse_key_string(s: &str) -> Option<KeyEvent> {
    let s = s.trim().to_lowercase();
    let parts: Vec<&str> = s.split('+').collect();

    let mut modifiers = KeyModifiers::NONE;

    let key_part = if parts.len() == 1 {
        parts[0]
    } else {
        for &modifier in &parts[..parts.len() - 1] {
            match modifier {
                "alt" => modifiers |= KeyModifiers::ALT,
                "ctrl" => modifiers |= KeyModifiers::CONTROL,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                _ => return None,
            }
        }
        parts[parts.len() - 1]
    };

    let code = match key_part {
        "tab" if modifiers.contains(KeyModifiers::SHIFT) => {
            modifiers -= KeyModifiers::SHIFT;
            KeyCode::BackTab
        }
        "tab" => KeyCode::Tab,
        "enter" => KeyCode::Enter,
        "esc" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "delete" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" => KeyCode::PageUp,
        "pagedown" => KeyCode::PageDown,
        "space" => KeyCode::Char(' '),
        s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        s if s.starts_with('f') => {
            let n: u8 = s[1..].parse().ok()?;
            KeyCode::F(n)
        }
        _ => return None,
    };

    Some(KeyEvent::new(code, modifiers))
}

fn command_from_name(name: &str) -> Option<Command> {
    match name {
        "quit" => Some(Command::Quit),
        "help" => Some(Command::ShowHelp),
        "focus_next" => Some(Command::FocusNextPane),
        "focus_prev" => Some(Command::FocusPrevPane),
        "focus_up" => Some(Command::FocusDirection(Direction::Up)),
        "focus_down" => Some(Command::FocusDirection(Direction::Down)),
        "focus_left" => Some(Command::FocusDirection(Direction::Left)),
        "focus_right" => Some(Command::FocusDirection(Direction::Right)),
        "split_vertical" => Some(Command::SplitVertical),
        "split_horizontal" => Some(Command::SplitHorizontal),
        "close_pane" => Some(Command::ClosePane),
        "new_tab" => Some(Command::NewTab),
        "close_tab" => Some(Command::CloseTab),
        "next_tab" => Some(Command::NextTab),
        "prev_tab" => Some(Command::PrevTab),
        "toggle_fullscreen" => Some(Command::ToggleFullscreen),
        "resize_grow" => Some(Command::ResizeGrow),
        "resize_shrink" => Some(Command::ResizeShrink),
        "namespace_selector" => Some(Command::EnterMode(InputMode::NamespaceSelector)),
        s if s.starts_with("goto_tab_") => s["goto_tab_".len()..].parse::<usize>().ok().map(Command::GoToTab),
        _ => None,
    }
}

fn pane_command_from_name(name: &str) -> Option<PaneCommand> {
    match name {
        "scroll_up" | "select_prev" | "navigate_up" => Some(PaneCommand::SelectPrev),
        "scroll_down" | "select_next" | "navigate_down" => Some(PaneCommand::SelectNext),
        "select" => Some(PaneCommand::Select),
        "back" => Some(PaneCommand::Back),
        "toggle_follow" => Some(PaneCommand::ToggleFollow),
        _ => None,
    }
}

fn resource_command_from_name(name: &str) -> Option<Command> {
    match name {
        "view_yaml" => Some(Command::ViewYaml),
        "view_describe" => Some(Command::ViewDescribe),
        "delete" => Some(Command::DeleteResource),
        "scale" => Some(Command::ScaleResource),
        "restart" => Some(Command::RestartRollout),
        "view_logs" => Some(Command::ViewLogs),
        "exec" => Some(Command::ExecInto),
        "toggle_all_namespaces" => Some(Command::ToggleAllNamespaces),
        "sort" => Some(Command::SortByColumn),
        "filter" => Some(Command::EnterMode(InputMode::FilterInput)),
        "resource_switcher" => Some(Command::EnterResourceSwitcher),
        _ => None,
    }
}

fn resource_command_description(name: &str) -> String {
    match name {
        "view_yaml" => "View YAML",
        "view_describe" => "Describe",
        "delete" => "Delete",
        "scale" => "Scale",
        "restart" => "Restart",
        "view_logs" => "Logs",
        "exec" => "Exec",
        "toggle_all_namespaces" => "All NS",
        "sort" => "Sort",
        "filter" => "Filter",
        "resource_switcher" => "Resources",
        _ => "Unknown",
    }
    .into()
}

fn command_description(name: &str) -> String {
    match name {
        "quit" => "Quit",
        "help" => "Help",
        "focus_next" => "Focus next",
        "focus_prev" => "Focus prev",
        "focus_up" => "Focus up",
        "focus_down" => "Focus down",
        "focus_left" => "Focus left",
        "focus_right" => "Focus right",
        "split_vertical" => "Split V",
        "split_horizontal" => "Split H",
        "close_pane" => "Close",
        "new_tab" => "New tab",
        "close_tab" => "Close tab",
        "next_tab" => "Next tab",
        "prev_tab" => "Prev tab",
        "toggle_fullscreen" => "Fullscreen",
        "resize_grow" => "Grow",
        "resize_shrink" => "Shrink",
        "namespace_selector" => "Namespace",
        s if s.starts_with("goto_tab_") => "Go to tab",
        _ => "Unknown",
    }
    .into()
}

fn pane_command_description(name: &str) -> String {
    match name {
        "scroll_up" | "select_prev" | "navigate_up" => "Up",
        "scroll_down" | "select_next" | "navigate_down" => "Down",
        "select" => "Select",
        "back" => "Back",
        "toggle_follow" => "Follow",
        _ => "Unknown",
    }
    .into()
}

#[cfg(test)]
mod tests;
