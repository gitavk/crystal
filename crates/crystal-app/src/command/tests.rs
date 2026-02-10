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
