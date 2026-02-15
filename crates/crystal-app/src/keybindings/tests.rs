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
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('l'), KeyModifiers::ALT)), Some(Command::ToggleAppLogsTab));
    assert_eq!(
        d.dispatch(press_mod(KeyCode::Char('o'), KeyModifiers::CONTROL)),
        Some(Command::EnterMode(InputMode::ContextSelector))
    );
    assert_eq!(d.dispatch(press(KeyCode::Tab)), Some(Command::FocusNextPane));
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('v'), KeyModifiers::ALT)), Some(Command::SplitVertical));
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('h'), KeyModifiers::ALT)), Some(Command::SplitHorizontal));
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('w'), KeyModifiers::ALT)), Some(Command::ClosePane));
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('c'), KeyModifiers::ALT)), Some(Command::CloseTab));
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
fn insert_mode_forwards_all_keys_as_send_input() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);

    // global bindings are NOT active in Insert mode — 'q' goes to terminal
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), Some(Command::Pane(PaneCommand::SendInput("q".into()))));

    let result = d.dispatch(press(KeyCode::Char('a')));
    assert_eq!(result, Some(Command::Pane(PaneCommand::SendInput("a".into()))));
}

#[test]
fn insert_mode_esc_exits_to_normal() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some(Command::ExitMode));
}

#[test]
fn insert_mode_ctrl_c_sends_interrupt() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);
    let result = d.dispatch(press_mod(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert_eq!(result, Some(Command::Pane(PaneCommand::SendInput("\x03".into()))));
}

#[test]
fn insert_mode_ctrl_d_sends_eof() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);
    let result = d.dispatch(press_mod(KeyCode::Char('d'), KeyModifiers::CONTROL));
    assert_eq!(result, Some(Command::Pane(PaneCommand::SendInput("\x04".into()))));
}

#[test]
fn insert_mode_arrow_keys_send_escape_sequences() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);

    assert_eq!(d.dispatch(press(KeyCode::Up)), Some(Command::Pane(PaneCommand::SendInput("\x1b[A".into()))));
    assert_eq!(d.dispatch(press(KeyCode::Down)), Some(Command::Pane(PaneCommand::SendInput("\x1b[B".into()))));
    assert_eq!(d.dispatch(press(KeyCode::Right)), Some(Command::Pane(PaneCommand::SendInput("\x1b[C".into()))));
    assert_eq!(d.dispatch(press(KeyCode::Left)), Some(Command::Pane(PaneCommand::SendInput("\x1b[D".into()))));
}

#[test]
fn insert_mode_enter_sends_carriage_return() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);
    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some(Command::Pane(PaneCommand::SendInput("\r".into()))));
}

#[test]
fn insert_mode_backspace_sends_del() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);
    assert_eq!(d.dispatch(press(KeyCode::Backspace)), Some(Command::Pane(PaneCommand::SendInput("\x7f".into()))));
}

#[test]
fn insert_mode_special_keys() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);

    assert_eq!(d.dispatch(press(KeyCode::Home)), Some(Command::Pane(PaneCommand::SendInput("\x1b[H".into()))));
    assert_eq!(d.dispatch(press(KeyCode::End)), Some(Command::Pane(PaneCommand::SendInput("\x1b[F".into()))));
    assert_eq!(d.dispatch(press(KeyCode::PageUp)), Some(Command::Pane(PaneCommand::SendInput("\x1b[5~".into()))));
    assert_eq!(d.dispatch(press(KeyCode::PageDown)), Some(Command::Pane(PaneCommand::SendInput("\x1b[6~".into()))));
    assert_eq!(d.dispatch(press(KeyCode::Delete)), Some(Command::Pane(PaneCommand::SendInput("\x1b[3~".into()))));
}

#[test]
fn normal_mode_arrow_keys_not_terminal_input() {
    let d = default_dispatcher();
    // In Normal mode, arrow keys should NOT produce SendInput
    let result = d.dispatch(press(KeyCode::Up));
    assert_ne!(result, Some(Command::Pane(PaneCommand::SendInput("\x1b[A".into()))));
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
fn shift_tab_dispatches_focus_prev() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press_mod(KeyCode::Tab, KeyModifiers::SHIFT)), Some(Command::FocusPrevPane));
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
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('k'), KeyModifiers::ALT)), Some(Command::ResizeGrow));
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('j'), KeyModifiers::ALT)), Some(Command::ResizeShrink));
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
fn context_mode_dispatches_nav_and_input() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::ContextSelector);

    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some(Command::ContextConfirm));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some(Command::ExitMode));
    assert_eq!(d.dispatch(press(KeyCode::Up)), Some(Command::Pane(PaneCommand::SelectPrev)));
    assert_eq!(d.dispatch(press(KeyCode::Down)), Some(Command::Pane(PaneCommand::SelectNext)));
    assert_eq!(d.dispatch(press(KeyCode::Char('a'))), Some(Command::ContextInput('a')));
    assert_eq!(d.dispatch(press(KeyCode::Backspace)), Some(Command::ContextBackspace));
}

#[test]
fn namespace_mode_global_bindings_still_active() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::NamespaceSelector);
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), Some(Command::Quit));
}

#[test]
fn resource_bindings_map_in_normal_mode() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press(KeyCode::Char('y'))), Some(Command::ViewYaml));
    assert_eq!(d.dispatch(press(KeyCode::Char('d'))), Some(Command::ViewDescribe));
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('d'), KeyModifiers::CONTROL)), Some(Command::DeleteResource));
    assert_eq!(d.dispatch(press(KeyCode::Char('l'))), Some(Command::ViewLogs));
    assert_eq!(d.dispatch(press(KeyCode::Char('e'))), Some(Command::ExecInto));
    assert_eq!(d.dispatch(press(KeyCode::Char('p'))), Some(Command::PortForward));
    assert_eq!(d.dispatch(press(KeyCode::Char('a'))), Some(Command::ToggleAllNamespaces));
    assert_eq!(d.dispatch(press(KeyCode::Char('s'))), Some(Command::SortByColumn));
    assert_eq!(d.dispatch(press(KeyCode::Char('/'))), Some(Command::EnterMode(InputMode::FilterInput)));
    assert_eq!(d.dispatch(press(KeyCode::Char(':'))), Some(Command::EnterResourceSwitcher));
}

#[test]
fn resource_bindings_shift_keys() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('S'), KeyModifiers::SHIFT)), Some(Command::ScaleResource));
    assert_eq!(d.dispatch(press_mod(KeyCode::Char('R'), KeyModifiers::SHIFT)), Some(Command::RestartRollout));
}

#[test]
fn resource_switcher_mode_accepts_input_backspace_confirm_esc() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::ResourceSwitcher);

    assert_eq!(d.dispatch(press(KeyCode::Char('p'))), Some(Command::ResourceSwitcherInput('p')));
    assert_eq!(d.dispatch(press(KeyCode::Backspace)), Some(Command::ResourceSwitcherBackspace));
    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some(Command::ResourceSwitcherConfirm));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some(Command::DenyAction));
}

#[test]
fn resource_switcher_mode_ignores_global_bindings() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::ResourceSwitcher);
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), Some(Command::ResourceSwitcherInput('q')));
}

#[test]
fn resource_switcher_mode_ignores_unknown_keys() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::ResourceSwitcher);
    assert_eq!(d.dispatch(press(KeyCode::F(5))), None);
}

#[test]
fn confirm_dialog_mode_accepts_y_n_esc() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::ConfirmDialog);

    assert_eq!(d.dispatch(press(KeyCode::Char('y'))), Some(Command::ConfirmAction));
    assert_eq!(d.dispatch(press(KeyCode::Char('n'))), Some(Command::DenyAction));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some(Command::DenyAction));
}

#[test]
fn confirm_dialog_mode_ignores_other_keys() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::ConfirmDialog);
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), None);
    assert_eq!(d.dispatch(press(KeyCode::Char('a'))), None);
    assert_eq!(d.dispatch(press(KeyCode::Tab)), None);
}

#[test]
fn filter_input_mode_forwards_chars_and_responds_to_esc_enter() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::FilterInput);

    assert_eq!(d.dispatch(press(KeyCode::Char('a'))), Some(Command::FilterInput('a')));
    assert_eq!(d.dispatch(press(KeyCode::Backspace)), Some(Command::FilterBackspace));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some(Command::FilterCancel));
    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some(Command::ExitMode));
}

#[test]
fn filter_input_mode_ignores_global_bindings() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::FilterInput);
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), Some(Command::FilterInput('q')));
}

#[test]
fn port_forward_input_mode_handles_edit_confirm_cancel() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::PortForwardInput);

    assert_eq!(d.dispatch(press(KeyCode::Char('3'))), Some(Command::PortForwardInput('3')));
    assert_eq!(d.dispatch(press(KeyCode::Backspace)), Some(Command::PortForwardBackspace));
    assert_eq!(d.dispatch(press(KeyCode::Tab)), Some(Command::PortForwardToggleField));
    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some(Command::PortForwardConfirm));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some(Command::PortForwardCancel));
}

#[test]
fn port_forward_input_mode_ignores_non_digits() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::PortForwardInput);
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), None);
}

#[test]
fn resource_command_config_names_map_correctly() {
    let mut config = KeybindingsConfig::default();
    config.resource.insert("view_yaml".into(), "f1".into());
    config.resource.insert("view_describe".into(), "f2".into());
    config.resource.insert("delete".into(), "f3".into());
    config.resource.insert("scale".into(), "f4".into());
    config.resource.insert("restart".into(), "f5".into());
    config.resource.insert("view_logs".into(), "f6".into());
    config.resource.insert("exec".into(), "f7".into());
    config.resource.insert("port_forward".into(), "f8".into());
    config.resource.insert("toggle_all_namespaces".into(), "f9".into());
    config.resource.insert("sort".into(), "f10".into());
    config.resource.insert("filter".into(), "f11".into());
    config.resource.insert("resource_switcher".into(), "f12".into());

    let d = KeybindingDispatcher::from_config(&config);
    assert_eq!(d.dispatch(press(KeyCode::F(1))), Some(Command::ViewYaml));
    assert_eq!(d.dispatch(press(KeyCode::F(2))), Some(Command::ViewDescribe));
    assert_eq!(d.dispatch(press(KeyCode::F(3))), Some(Command::DeleteResource));
    assert_eq!(d.dispatch(press(KeyCode::F(4))), Some(Command::ScaleResource));
    assert_eq!(d.dispatch(press(KeyCode::F(5))), Some(Command::RestartRollout));
    assert_eq!(d.dispatch(press(KeyCode::F(6))), Some(Command::ViewLogs));
    assert_eq!(d.dispatch(press(KeyCode::F(7))), Some(Command::ExecInto));
    assert_eq!(d.dispatch(press(KeyCode::F(8))), Some(Command::PortForward));
    assert_eq!(d.dispatch(press(KeyCode::F(9))), Some(Command::ToggleAllNamespaces));
    assert_eq!(d.dispatch(press(KeyCode::F(10))), Some(Command::SortByColumn));
    assert_eq!(d.dispatch(press(KeyCode::F(11))), Some(Command::EnterMode(InputMode::FilterInput)));
    assert_eq!(d.dispatch(press(KeyCode::F(12))), Some(Command::EnterResourceSwitcher));
}
