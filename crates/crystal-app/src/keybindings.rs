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
    ContextSelector,
    ResourceSwitcher,
    ConfirmDialog,
    FilterInput,
    PortForwardInput,
}

#[allow(dead_code)]
pub struct KeybindingDispatcher {
    mode: InputMode,
    global_bindings: HashMap<KeyEvent, Command>,
    mutate_bindings: HashMap<KeyEvent, Command>,
    browse_bindings: HashMap<KeyEvent, Command>,
    navigation_bindings: HashMap<KeyEvent, Command>,
    tui_bindings: HashMap<KeyEvent, Command>,
    reverse_global: Vec<(String, String, String)>,
    reverse_mutate: Vec<(String, String, String)>,
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
            browse_bindings,
            navigation_bindings,
            tui_bindings,
            reverse_global,
            reverse_mutate,
            reverse_browse,
            reverse_navigation,
            reverse_tui,
        }
    }

    pub fn dispatch(&self, key: KeyEvent) -> Option<Command> {
        let key = normalize_key_event(key);

        match self.mode {
            InputMode::Insert => {
                if key.code == KeyCode::Esc {
                    return Some(Command::ExitMode);
                }
                let s = key_to_input_string(key);
                if s.is_empty() {
                    return None;
                }
                return Some(Command::Pane(PaneCommand::SendInput(s)));
            }
            InputMode::ResourceSwitcher => match key.code {
                KeyCode::Enter => return Some(Command::ResourceSwitcherConfirm),
                KeyCode::Esc => return Some(Command::DenyAction),
                KeyCode::Up => return Some(Command::Pane(PaneCommand::SelectPrev)),
                KeyCode::Down => return Some(Command::Pane(PaneCommand::SelectNext)),
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
                KeyCode::Esc => return Some(Command::FilterCancel),
                KeyCode::Enter => return Some(Command::ExitMode),
                KeyCode::Char(c) => return Some(Command::FilterInput(c)),
                KeyCode::Backspace => return Some(Command::FilterBackspace),
                _ => return None,
            },
            InputMode::PortForwardInput => match key.code {
                KeyCode::Esc => return Some(Command::PortForwardCancel),
                KeyCode::Enter => return Some(Command::PortForwardConfirm),
                KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                    return Some(Command::PortForwardToggleField);
                }
                KeyCode::Char(c) if c.is_ascii_digit() => return Some(Command::PortForwardInput(c)),
                KeyCode::Backspace => return Some(Command::PortForwardBackspace),
                _ => return None,
            },
            _ => {}
        }

        if let Some(cmd) = self.global_bindings.get(&key) {
            return Some(cmd.clone());
        }

        match self.mode {
            InputMode::Insert => unreachable!("handled above"),
            InputMode::Normal => self
                .mutate_bindings
                .get(&key)
                .or_else(|| self.browse_bindings.get(&key))
                .or_else(|| self.navigation_bindings.get(&key))
                .or_else(|| self.tui_bindings.get(&key))
                .cloned(),
            InputMode::NamespaceSelector => match key.code {
                KeyCode::Enter => Some(Command::NamespaceConfirm),
                KeyCode::Esc => Some(Command::ExitMode),
                KeyCode::Up => Some(Command::Pane(PaneCommand::SelectPrev)),
                KeyCode::Down => Some(Command::Pane(PaneCommand::SelectNext)),
                KeyCode::Char(c) => Some(Command::NamespaceInput(c)),
                KeyCode::Backspace => Some(Command::NamespaceBackspace),
                _ => None,
            },
            InputMode::ContextSelector => match key.code {
                KeyCode::Enter => Some(Command::ContextConfirm),
                KeyCode::Esc => Some(Command::ExitMode),
                KeyCode::Up => Some(Command::Pane(PaneCommand::SelectPrev)),
                KeyCode::Down => Some(Command::Pane(PaneCommand::SelectNext)),
                KeyCode::Char(c) => Some(Command::ContextInput(c)),
                KeyCode::Backspace => Some(Command::ContextBackspace),
                _ => None,
            },
            InputMode::Search | InputMode::Command => None,
            InputMode::Pane | InputMode::Tab => None,
            InputMode::ResourceSwitcher
            | InputMode::ConfirmDialog
            | InputMode::FilterInput
            | InputMode::PortForwardInput => {
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
        let all_reverse: Vec<_> = self
            .reverse_global
            .iter()
            .chain(&self.reverse_tui)
            .chain(&self.reverse_browse)
            .chain(&self.reverse_navigation)
            .chain(&self.reverse_mutate)
            .collect();

        let priority = [
            "split_vertical",
            "split_horizontal",
            "focus_next",
            "close_pane",
            "namespace_selector",
            "context_selector",
            "app_logs",
            "help",
            "quit",
        ];
        let mut hints = Vec::new();
        for name in &priority {
            if let Some((_, key_str, desc)) = all_reverse.iter().find(|(n, _, _)| n == name) {
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

    pub fn navigation_shortcuts(&self) -> Vec<(String, String)> {
        let mut sorted = self.reverse_navigation.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted.into_iter().map(|(_, key_str, desc)| (format_key_display(&key_str), desc)).collect()
    }

    pub fn browse_shortcuts(&self) -> Vec<(String, String)> {
        let mut sorted = self.reverse_browse.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted.into_iter().map(|(_, key_str, desc)| (format_key_display(&key_str), desc)).collect()
    }

    pub fn tui_shortcuts(&self) -> Vec<(String, String)> {
        let mut sorted = self.reverse_tui.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted.into_iter().map(|(_, key_str, desc)| (format_key_display(&key_str), desc)).collect()
    }

    pub fn mutate_shortcuts(&self) -> Vec<(String, String)> {
        let mut sorted = self.reverse_mutate.clone();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        sorted.into_iter().map(|(_, key_str, desc)| (format_key_display(&key_str), desc)).collect()
    }
}

fn normalize_key_event(key: KeyEvent) -> KeyEvent {
    if key.code == KeyCode::Tab && key.modifiers.contains(KeyModifiers::SHIFT) {
        let mut modifiers = key.modifiers;
        modifiers -= KeyModifiers::SHIFT;
        return KeyEvent::new(KeyCode::BackTab, modifiers);
    }
    if key.code == KeyCode::BackTab && key.modifiers.contains(KeyModifiers::SHIFT) {
        let mut modifiers = key.modifiers;
        modifiers -= KeyModifiers::SHIFT;
        return KeyEvent::new(KeyCode::BackTab, modifiers);
    }
    // Normalize Shift+char: crossterm may report Shift+'g' or just 'G' with SHIFT.
    // Canonicalize to uppercase char + SHIFT modifier.
    if let KeyCode::Char(c) = key.code {
        if c.is_ascii_lowercase() && key.modifiers.contains(KeyModifiers::SHIFT) {
            return KeyEvent::new(KeyCode::Char(c.to_ascii_uppercase()), key.modifiers);
        }
        if c.is_ascii_uppercase() && !key.modifiers.contains(KeyModifiers::SHIFT) {
            return KeyEvent::new(key.code, key.modifiers | KeyModifiers::SHIFT);
        }
    }
    key
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
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char(c) = key.code {
            let byte = (c as u8).wrapping_sub(b'a').wrapping_add(1);
            return String::from(byte as char);
        }
    }

    match key.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "\r".into(),
        KeyCode::Tab => "\t".into(),
        KeyCode::Backspace => "\x7f".into(),
        KeyCode::Esc => "\x1b".into(),
        KeyCode::Up => "\x1b[A".into(),
        KeyCode::Down => "\x1b[B".into(),
        KeyCode::Right => "\x1b[C".into(),
        KeyCode::Left => "\x1b[D".into(),
        KeyCode::Home => "\x1b[H".into(),
        KeyCode::End => "\x1b[F".into(),
        KeyCode::PageUp => "\x1b[5~".into(),
        KeyCode::PageDown => "\x1b[6~".into(),
        KeyCode::Delete => "\x1b[3~".into(),
        KeyCode::F(n) => match n {
            1 => "\x1bOP".into(),
            2 => "\x1bOQ".into(),
            3 => "\x1bOR".into(),
            4 => "\x1bOS".into(),
            5 => "\x1b[15~".into(),
            6 => "\x1b[17~".into(),
            7 => "\x1b[18~".into(),
            8 => "\x1b[19~".into(),
            9 => "\x1b[20~".into(),
            10 => "\x1b[21~".into(),
            11 => "\x1b[23~".into(),
            12 => "\x1b[24~".into(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

pub fn parse_key_string(s: &str) -> Option<KeyEvent> {
    let trimmed = s.trim();
    let parts: Vec<&str> = trimmed.split('+').collect();

    let mut modifiers = KeyModifiers::NONE;

    let key_part_raw = if parts.len() == 1 {
        parts[0]
    } else {
        for &modifier in &parts[..parts.len() - 1] {
            match modifier.to_ascii_lowercase().as_str() {
                "alt" => modifiers |= KeyModifiers::ALT,
                "ctrl" => modifiers |= KeyModifiers::CONTROL,
                "shift" => modifiers |= KeyModifiers::SHIFT,
                _ => return None,
            }
        }
        parts[parts.len() - 1]
    };

    let key_lower = key_part_raw.to_ascii_lowercase();
    let code = match key_lower.as_str() {
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
        _ if key_part_raw.len() == 1 => {
            let ch = key_part_raw.chars().next().unwrap();
            if ch.is_ascii_uppercase() {
                modifiers |= KeyModifiers::SHIFT;
                KeyCode::Char(ch)
            } else if modifiers.contains(KeyModifiers::SHIFT) && ch.is_ascii_lowercase() {
                KeyCode::Char(ch.to_ascii_uppercase())
            } else {
                KeyCode::Char(ch)
            }
        }
        s if s.starts_with('f') => {
            let n: u8 = s[1..].parse().ok()?;
            KeyCode::F(n)
        }
        _ => return None,
    };

    Some(KeyEvent::new(code, modifiers))
}

fn global_command_from_name(name: &str) -> Option<Command> {
    match name {
        "quit" => Some(Command::Quit),
        "help" => Some(Command::ShowHelp),
        "app_logs" => Some(Command::ToggleAppLogsTab),
        "enter_insert" => Some(Command::EnterMode(InputMode::Insert)),
        "namespace_selector" => Some(Command::EnterMode(InputMode::NamespaceSelector)),
        "context_selector" => Some(Command::EnterMode(InputMode::ContextSelector)),
        _ => None,
    }
}

fn global_command_description(name: &str) -> String {
    match name {
        "quit" => "Quit",
        "help" => "Help",
        "app_logs" => "App logs",
        "enter_insert" => "Insert mode",
        "namespace_selector" => "Namespace",
        "context_selector" => "Context",
        _ => "Unknown",
    }
    .into()
}

fn mutate_command_from_name(name: &str) -> Option<Command> {
    match name {
        "delete" => Some(Command::DeleteResource),
        "scale" => Some(Command::ScaleResource),
        "restart_rollout" => Some(Command::RestartRollout),
        "exec" => Some(Command::ExecInto),
        "port_forward" => Some(Command::PortForward),
        _ => None,
    }
}

fn mutate_command_description(name: &str) -> String {
    match name {
        "delete" => "Delete",
        "scale" => "Scale",
        "restart_rollout" => "Restart",
        "exec" => "Exec",
        "port_forward" => "Port Forward",
        _ => "Unknown",
    }
    .into()
}

fn browse_command_from_name(name: &str) -> Option<Command> {
    match name {
        "view_yaml" => Some(Command::ViewYaml),
        "view_describe" => Some(Command::ViewDescribe),
        "view_logs" => Some(Command::ViewLogs),
        "filter" => Some(Command::EnterMode(InputMode::FilterInput)),
        "resource_switcher" => Some(Command::EnterResourceSwitcher),
        "sort_column" => Some(Command::SortByColumn),
        "toggle_all_namespaces" => Some(Command::ToggleAllNamespaces),
        "toggle_follow" => Some(Command::Pane(PaneCommand::ToggleFollow)),
        _ => None,
    }
}

fn browse_command_description(name: &str) -> String {
    match name {
        "view_yaml" => "View YAML",
        "view_describe" => "Describe",
        "view_logs" => "Logs",
        "filter" => "Filter",
        "resource_switcher" => "Resources",
        "sort_column" => "Sort",
        "toggle_all_namespaces" => "All NS",
        "toggle_follow" => "Follow",
        _ => "Unknown",
    }
    .into()
}

fn navigation_command_from_name(name: &str) -> Option<Command> {
    match name {
        "scroll_up" | "select_prev" => Some(Command::Pane(PaneCommand::SelectPrev)),
        "scroll_down" | "select_next" => Some(Command::Pane(PaneCommand::SelectNext)),
        "select" => Some(Command::Pane(PaneCommand::Select)),
        "back" => Some(Command::Pane(PaneCommand::Back)),
        "go_to_top" => Some(Command::Pane(PaneCommand::GoToTop)),
        "go_to_bottom" => Some(Command::Pane(PaneCommand::GoToBottom)),
        "page_up" => Some(Command::Pane(PaneCommand::PageUp)),
        "page_down" => Some(Command::Pane(PaneCommand::PageDown)),
        _ => None,
    }
}

fn navigation_command_description(name: &str) -> String {
    match name {
        "scroll_up" | "select_prev" => "Up",
        "scroll_down" | "select_next" => "Down",
        "select" => "Select",
        "back" => "Back",
        "go_to_top" => "Go to top",
        "go_to_bottom" => "Go to bottom",
        "page_up" => "Page up",
        "page_down" => "Page down",
        _ => "Unknown",
    }
    .into()
}

fn tui_command_from_name(name: &str) -> Option<Command> {
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

fn tui_command_description(name: &str) -> String {
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

#[cfg(test)]
mod tests;
