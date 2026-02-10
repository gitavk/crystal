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
mod tests {
    use super::*;
    use crossterm::event::KeyEventKind;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn press_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent { code, modifiers, kind: KeyEventKind::Press, state: crossterm::event::KeyEventState::NONE }
    }

    #[test]
    fn quit_maps_globally() {
        let cmd = map_key_to_command(press(KeyCode::Char('q')), InputMode::Normal);
        assert_eq!(cmd, Some(Command::Quit));
    }

    #[test]
    fn j_maps_to_pane_select_next() {
        let cmd = map_key_to_command(press(KeyCode::Char('j')), InputMode::Normal);
        assert_eq!(cmd, Some(Command::Pane(PaneCommand::SelectNext)));
    }

    #[test]
    fn k_maps_to_pane_select_prev() {
        let cmd = map_key_to_command(press(KeyCode::Char('k')), InputMode::Normal);
        assert_eq!(cmd, Some(Command::Pane(PaneCommand::SelectPrev)));
    }

    #[test]
    fn tab_maps_to_focus_next() {
        let cmd = map_key_to_command(press(KeyCode::Tab), InputMode::Normal);
        assert_eq!(cmd, Some(Command::FocusNextPane));
    }

    #[test]
    fn backtab_maps_to_focus_prev() {
        let cmd = map_key_to_command(press(KeyCode::BackTab), InputMode::Normal);
        assert_eq!(cmd, Some(Command::FocusPrevPane));
    }

    #[test]
    fn help_maps_globally() {
        let cmd = map_key_to_command(press(KeyCode::Char('?')), InputMode::Normal);
        assert_eq!(cmd, Some(Command::ShowHelp));
    }

    #[test]
    fn alt_v_maps_to_split_vertical() {
        let cmd = map_key_to_command(press_mod(KeyCode::Char('v'), KeyModifiers::ALT), InputMode::Normal);
        assert_eq!(cmd, Some(Command::SplitVertical));
    }

    #[test]
    fn namespace_mode_returns_none() {
        let cmd = map_key_to_command(press(KeyCode::Char('j')), InputMode::NamespaceSelector);
        assert_eq!(cmd, None);
    }
}
