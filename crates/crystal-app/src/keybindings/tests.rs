use super::*;
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
fn dispatch_configured_keys_to_commands() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), Some(Command::Quit));
    assert_eq!(d.dispatch(press(KeyCode::Char('?'))), Some(Command::ShowHelp));
    assert_eq!(d.dispatch(press(KeyCode::Tab)), Some(Command::FocusNextPane));
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('v'), KeyModifiers::ALT)), Some(Command::SplitVertical));
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('h'), KeyModifiers::ALT)), Some(Command::SplitHorizontal));
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('w'), KeyModifiers::ALT)), Some(Command::ClosePane));
}

#[test]
fn global_takes_precedence_over_pane() {
    let mut config = KeybindingsConfig::default();
    config.global.insert("quit".into(), "j".into());
    config.pane.insert("select_next".into(), "j".into());

    let d = KeybindingDispatcher::from_config(&config);
    assert_eq!(d.dispatch(press(KeyCode::Char('j'))), Some(Command::Quit));
}

#[test]
fn config_merge_overrides() {
    let mut config = crystal_config::Config::load();
    config.keybindings.global.insert("quit".into(), "x".into());
    let d = KeybindingDispatcher::from_config(&config.keybindings);

    assert_eq!(d.dispatch(press(KeyCode::Char('x'))), Some(Command::Quit));
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), None);
}

#[test]
fn invalid_key_string_skipped() {
    let mut config = KeybindingsConfig::default();
    config.global.insert("quit".into(), "notakey+combo+bad".into());
    config.global.insert("help".into(), "?".into());

    let d = KeybindingDispatcher::from_config(&config);
    assert_eq!(d.dispatch(press(KeyCode::Char('?'))), Some(Command::ShowHelp));
}

#[test]
fn missing_config_uses_defaults() {
    let d = default_dispatcher();
    assert!(d.dispatch(press(KeyCode::Char('q'))).is_some());
    assert!(d.dispatch(press(KeyCode::Enter)).is_some());
}

#[test]
fn mode_switch_changes_active_bindings() {
    let mut d = default_dispatcher();

    assert_eq!(d.dispatch(press(KeyCode::Char('j'))), Some(Command::Pane(PaneCommand::SelectNext)));

    d.set_mode(InputMode::NamespaceSelector);
    // pane bindings not active — char forwarded as namespace input
    assert_eq!(d.dispatch(press(KeyCode::Char('j'))), Some(Command::NamespaceInput('j')));
    // global bindings still active
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), Some(Command::Quit));
}

#[test]
fn insert_mode_forwards_non_global_as_send_input() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);

    // global binding still works
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), Some(Command::Quit));

    // non-global key forwarded as SendInput
    let result = d.dispatch(press(KeyCode::Char('a')));
    assert_eq!(result, Some(Command::Pane(PaneCommand::SendInput("a".into()))));
}

#[test]
fn parse_simple_char() {
    let key = parse_key_string("q").unwrap();
    assert_eq!(key.code, KeyCode::Char('q'));
    assert_eq!(key.modifiers, KeyModifiers::NONE);
}

#[test]
fn parse_alt_modifier() {
    let key = parse_key_string("alt+v").unwrap();
    assert_eq!(key.code, KeyCode::Char('v'));
    assert!(key.modifiers.contains(KeyModifiers::ALT));
}

#[test]
fn parse_ctrl_modifier() {
    let key = parse_key_string("ctrl+c").unwrap();
    assert_eq!(key.code, KeyCode::Char('c'));
    assert!(key.modifiers.contains(KeyModifiers::CONTROL));
}

#[test]
fn parse_shift_tab_becomes_backtab() {
    let key = parse_key_string("shift+tab").unwrap();
    assert_eq!(key.code, KeyCode::BackTab);
    assert!(!key.modifiers.contains(KeyModifiers::SHIFT));
}

#[test]
fn parse_special_keys() {
    assert_eq!(parse_key_string("enter").unwrap().code, KeyCode::Enter);
    assert_eq!(parse_key_string("esc").unwrap().code, KeyCode::Esc);
    assert_eq!(parse_key_string("tab").unwrap().code, KeyCode::Tab);
    assert_eq!(parse_key_string("up").unwrap().code, KeyCode::Up);
    assert_eq!(parse_key_string("down").unwrap().code, KeyCode::Down);
    assert_eq!(parse_key_string("space").unwrap().code, KeyCode::Char(' '));
}

#[test]
fn parse_alt_arrow() {
    let key = parse_key_string("alt+up").unwrap();
    assert_eq!(key.code, KeyCode::Up);
    assert!(key.modifiers.contains(KeyModifiers::ALT));
}

#[test]
fn parse_alt_bracket() {
    let key = parse_key_string("alt+[").unwrap();
    assert_eq!(key.code, KeyCode::Char('['));
    assert!(key.modifiers.contains(KeyModifiers::ALT));
}

#[test]
fn parse_function_key() {
    let key = parse_key_string("f1").unwrap();
    assert_eq!(key.code, KeyCode::F(1));
}

#[test]
fn parse_invalid_returns_none() {
    assert!(parse_key_string("notakey+combo+bad").is_none());
}

#[test]
fn global_hints_returns_expected_keys() {
    let d = default_dispatcher();
    let hints = d.global_hints();
    assert!(!hints.is_empty());
    let keys: Vec<&str> = hints.iter().map(|(k, _)| k.as_str()).collect();
    assert!(keys.contains(&"alt+v"));
    assert!(keys.contains(&"q"));
}

#[test]
fn global_shortcuts_formatted() {
    let d = default_dispatcher();
    let shortcuts = d.global_shortcuts();
    assert!(!shortcuts.is_empty());
    let has_capitalized = shortcuts.iter().any(|(k, _)| k.starts_with(|c: char| c.is_uppercase()));
    assert!(has_capitalized);
}

#[test]
fn goto_tab_dispatch() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press(KeyCode::Char('1'))), Some(Command::GoToTab(1)));
    assert_eq!(d.dispatch(press(KeyCode::Char('5'))), Some(Command::GoToTab(5)));
    assert_eq!(d.dispatch(press(KeyCode::Char('9'))), Some(Command::GoToTab(9)));
}

#[test]
fn focus_direction_dispatch() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press_mod(KeyCode::Up, KeyModifiers::ALT)), Some(Command::FocusDirection(Direction::Up)));
    assert_eq!(d.dispatch(press_mod(KeyCode::Down, KeyModifiers::ALT)), Some(Command::FocusDirection(Direction::Down)));
    assert_eq!(d.dispatch(press_mod(KeyCode::Left, KeyModifiers::ALT)), Some(Command::FocusDirection(Direction::Left)));
    assert_eq!(
        d.dispatch(press_mod(KeyCode::Right, KeyModifiers::ALT)),
        Some(Command::FocusDirection(Direction::Right))
    );
}

#[test]
fn resize_dispatch() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press_mod(KeyCode::Char(']'), KeyModifiers::ALT)), Some(Command::ResizeGrow));
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('['), KeyModifiers::ALT)), Some(Command::ResizeShrink));
}

#[test]
fn fullscreen_dispatch() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('f'), KeyModifiers::ALT)), Some(Command::ToggleFullscreen));
}

#[test]
fn namespace_mode_dispatches_nav_and_input() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::NamespaceSelector);

    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some(Command::NamespaceConfirm));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some(Command::ExitMode));
    assert_eq!(d.dispatch(press(KeyCode::Up)), Some(Command::Pane(PaneCommand::SelectPrev)));
    assert_eq!(d.dispatch(press(KeyCode::Down)), Some(Command::Pane(PaneCommand::SelectNext)));
    assert_eq!(d.dispatch(press(KeyCode::Char('a'))), Some(Command::NamespaceInput('a')));
    assert_eq!(d.dispatch(press(KeyCode::Backspace)), Some(Command::NamespaceBackspace));
}

#[test]
fn namespace_mode_global_bindings_still_active() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::NamespaceSelector);
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), Some(Command::Quit));
}
