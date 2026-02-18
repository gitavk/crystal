use super::*;
use crate::keybindings::KeybindingDispatcher;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn default_dispatcher() -> KeybindingDispatcher {
    let config = crystal_config::Config::load();
    KeybindingDispatcher::from_config(&config.keybindings)
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn press_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn quit_maps_globally() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('q'), KeyModifiers::CONTROL)), Some((Command::Quit, false)));
}

#[test]
fn j_maps_to_pane_select_next() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press(KeyCode::Char('j'))), Some((Command::Pane(PaneCommand::SelectNext), false)));
}

#[test]
fn k_maps_to_pane_select_prev() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press(KeyCode::Char('k'))), Some((Command::Pane(PaneCommand::SelectPrev), false)));
}

#[test]
fn tab_maps_to_focus_next() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press(KeyCode::Tab)), Some((Command::FocusNextPane, false)));
}

#[test]
fn backtab_maps_to_focus_prev() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press(KeyCode::BackTab)), Some((Command::FocusPrevPane, false)));
}

#[test]
fn help_maps_globally() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press(KeyCode::F(1))), Some((Command::ShowHelp, false)));
}

#[test]
fn alt_v_maps_to_split_vertical() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('v'), KeyModifiers::ALT)), Some((Command::SplitVertical, false)));
}

#[test]
fn namespace_mode_forwards_chars_as_input() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::NamespaceSelector);
    assert_eq!(d.dispatch(press(KeyCode::Char('j'))), Some((Command::NamespaceInput('j'), false)));
}
