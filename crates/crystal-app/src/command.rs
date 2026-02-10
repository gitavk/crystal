use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crystal_tui::pane::{Direction, PaneCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    NamespaceSelector,
}

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
    EnterMode(InputMode),
    ExitMode,
    Pane(PaneCommand),
}

pub fn map_key_to_command(key: KeyEvent, mode: InputMode) -> Option<Command> {
    match mode {
        InputMode::Normal => map_normal_key(key),
        InputMode::NamespaceSelector => None,
    }
}

fn map_normal_key(key: KeyEvent) -> Option<Command> {
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    match key.code {
        KeyCode::Char('q') => Some(Command::Quit),
        KeyCode::Char('?') => Some(Command::ShowHelp),
        KeyCode::Tab => Some(Command::FocusNextPane),
        KeyCode::BackTab => Some(Command::FocusPrevPane),
        KeyCode::Char('v') if alt => Some(Command::SplitVertical),
        KeyCode::Char('h') if alt => Some(Command::SplitHorizontal),
        KeyCode::Char('w') if alt => Some(Command::ClosePane),
        KeyCode::Char(':') => Some(Command::EnterMode(InputMode::NamespaceSelector)),
        KeyCode::Char('j') | KeyCode::Down => Some(Command::Pane(PaneCommand::SelectNext)),
        KeyCode::Char('k') | KeyCode::Up => Some(Command::Pane(PaneCommand::SelectPrev)),
        KeyCode::Enter => Some(Command::Pane(PaneCommand::Select)),
        KeyCode::Esc => Some(Command::Pane(PaneCommand::Back)),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
