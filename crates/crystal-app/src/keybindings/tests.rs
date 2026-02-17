use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn default_dispatcher() -> KeybindingDispatcher {
    let config = crystal_config::Config::default();
    KeybindingDispatcher::from_config(&config.keybindings)
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn press_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn alt(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::ALT)
}

fn ctrl_alt(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL | KeyModifiers::ALT)
}

#[test]
fn dispatch_global_keys() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(ctrl(KeyCode::Char('q'))), Some((Command::Quit, false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('?'))), Some((Command::ShowHelp, false)));
    assert_eq!(d.dispatch(ctrl(KeyCode::Char('l'))), Some((Command::ToggleAppLogsTab, false)));
    assert_eq!(d.dispatch(ctrl(KeyCode::Char('o'))), Some((Command::EnterMode(InputMode::ContextSelector), false)));
    assert_eq!(d.dispatch(ctrl(KeyCode::Char('n'))), Some((Command::EnterMode(InputMode::NamespaceSelector), false)));
    assert_eq!(d.dispatch(ctrl(KeyCode::Char('e'))), Some((Command::EnterMode(InputMode::Insert), false)));
}

#[test]
fn dispatch_navigation_keys() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press(KeyCode::Char('j'))), Some((Command::Pane(PaneCommand::SelectNext), false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('k'))), Some((Command::Pane(PaneCommand::SelectPrev), false)));
    assert_eq!(d.dispatch(press(KeyCode::Down)), Some((Command::Pane(PaneCommand::SelectNext), false)));
    assert_eq!(d.dispatch(press(KeyCode::Up)), Some((Command::Pane(PaneCommand::SelectPrev), false)));
    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some((Command::Pane(PaneCommand::Select), false)));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some((Command::Pane(PaneCommand::Back), false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('g'))), Some((Command::Pane(PaneCommand::GoToTop), false)));
    assert_eq!(
        d.dispatch(press_mod(KeyCode::Char('G'), KeyModifiers::SHIFT)),
        Some((Command::Pane(PaneCommand::GoToBottom), false))
    );
    assert_eq!(d.dispatch(press(KeyCode::PageUp)), Some((Command::Pane(PaneCommand::PageUp), false)));
    assert_eq!(d.dispatch(press(KeyCode::PageDown)), Some((Command::Pane(PaneCommand::PageDown), false)));
}

#[test]
fn dispatch_browse_keys() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press(KeyCode::Char('y'))), Some((Command::ViewYaml, false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('d'))), Some((Command::ViewDescribe, false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('l'))), Some((Command::ViewLogs, false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('/'))), Some((Command::EnterMode(InputMode::FilterInput), false)));
    assert_eq!(d.dispatch(press(KeyCode::Char(':'))), Some((Command::EnterResourceSwitcher, false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('s'))), Some((Command::SortByColumn, false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('a'))), Some((Command::ToggleAllNamespaces, false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('f'))), Some((Command::Pane(PaneCommand::ToggleFollow), false)));
}

#[test]
fn dispatch_tui_keys() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(alt(KeyCode::Char('v'))), Some((Command::SplitVertical, false)));
    assert_eq!(d.dispatch(alt(KeyCode::Char('h'))), Some((Command::SplitHorizontal, false)));
    assert_eq!(d.dispatch(alt(KeyCode::Char('w'))), Some((Command::ClosePane, false)));
    assert_eq!(d.dispatch(alt(KeyCode::Char('f'))), Some((Command::ToggleFullscreen, false)));
    assert_eq!(d.dispatch(alt(KeyCode::Char('t'))), Some((Command::NewTab, false)));
    assert_eq!(d.dispatch(alt(KeyCode::Char('c'))), Some((Command::CloseTab, false)));
    assert_eq!(d.dispatch(press(KeyCode::Tab)), Some((Command::FocusNextPane, false)));
    assert_eq!(d.dispatch(press_mod(KeyCode::Tab, KeyModifiers::SHIFT)), Some((Command::FocusPrevPane, false)));
}

#[test]
fn dispatch_mutate_keys_require_confirmation() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(ctrl_alt(KeyCode::Char('d'))), Some((Command::DeleteResource, true)));
    assert_eq!(d.dispatch(ctrl_alt(KeyCode::Char('s'))), Some((Command::ScaleResource, true)));
    assert_eq!(d.dispatch(ctrl_alt(KeyCode::Char('r'))), Some((Command::RestartRollout, true)));
}

#[test]
fn dispatch_interact_keys_no_confirmation() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(press(KeyCode::Char('e'))), Some((Command::ExecInto, false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('p'))), Some((Command::PortForward, false)));
}

#[test]
fn global_takes_precedence_over_navigation() {
    let mut config = KeybindingsConfig::default();
    config.global.insert("quit".into(), "j".into());
    config.navigation.insert("scroll_down".into(), "j".into());

    let d = KeybindingDispatcher::from_config(&config);
    assert_eq!(d.dispatch(press(KeyCode::Char('j'))), Some((Command::Quit, false)));
}

#[test]
fn global_shadows_lower_priority_group() {
    let mut config = KeybindingsConfig::default();
    config.global.insert("quit".into(), "x".into());
    config.mutate.insert("delete".into(), "x".into());
    config.browse.insert("view_yaml".into(), "x".into());
    config.navigation.insert("scroll_up".into(), "x".into());
    config.tui.insert("new_tab".into(), "x".into());

    let d = KeybindingDispatcher::from_config(&config);
    assert_eq!(d.dispatch(press(KeyCode::Char('x'))), Some((Command::Quit, false)));
}

#[test]
fn config_merge_overrides() {
    let mut config = crystal_config::Config::load();
    config.keybindings.global.insert("quit".into(), "ctrl+x".into());
    let d = KeybindingDispatcher::from_config(&config.keybindings);

    assert_eq!(d.dispatch(ctrl(KeyCode::Char('x'))), Some((Command::Quit, false)));
    assert_eq!(d.dispatch(ctrl(KeyCode::Char('q'))), None);
}

#[test]
fn invalid_key_string_skipped() {
    let mut config = KeybindingsConfig::default();
    config.global.insert("quit".into(), "notakey+combo+bad".into());
    config.global.insert("help".into(), "?".into());

    let d = KeybindingDispatcher::from_config(&config);
    assert_eq!(d.dispatch(press(KeyCode::Char('?'))), Some((Command::ShowHelp, false)));
}

#[test]
fn missing_config_uses_defaults() {
    let d = default_dispatcher();
    assert!(d.dispatch(ctrl(KeyCode::Char('q'))).is_some());
    assert!(d.dispatch(press(KeyCode::Enter)).is_some());
}

#[test]
fn mode_switch_changes_active_bindings() {
    let mut d = default_dispatcher();

    assert_eq!(d.dispatch(press(KeyCode::Char('j'))), Some((Command::Pane(PaneCommand::SelectNext), false)));

    d.set_mode(InputMode::NamespaceSelector);
    assert_eq!(d.dispatch(press(KeyCode::Char('j'))), Some((Command::NamespaceInput('j'), false)));
    assert_eq!(d.dispatch(ctrl(KeyCode::Char('q'))), Some((Command::Quit, false)));
}

#[test]
fn insert_mode_forwards_all_keys_as_send_input() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);

    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), Some((Command::Pane(PaneCommand::SendInput("q".into())), false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('a'))), Some((Command::Pane(PaneCommand::SendInput("a".into())), false)));
}

#[test]
fn insert_mode_esc_exits_to_normal() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some((Command::ExitMode, false)));
}

#[test]
fn insert_mode_ctrl_c_sends_interrupt() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);
    let result = d.dispatch(ctrl(KeyCode::Char('c')));
    assert_eq!(result, Some((Command::Pane(PaneCommand::SendInput("\x03".into())), false)));
}

#[test]
fn insert_mode_ctrl_d_sends_eof() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);
    let result = d.dispatch(ctrl(KeyCode::Char('d')));
    assert_eq!(result, Some((Command::Pane(PaneCommand::SendInput("\x04".into())), false)));
}

#[test]
fn insert_mode_arrow_keys_send_escape_sequences() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);

    assert_eq!(d.dispatch(press(KeyCode::Up)), Some((Command::Pane(PaneCommand::SendInput("\x1b[A".into())), false)));
    assert_eq!(d.dispatch(press(KeyCode::Down)), Some((Command::Pane(PaneCommand::SendInput("\x1b[B".into())), false)));
    assert_eq!(
        d.dispatch(press(KeyCode::Right)),
        Some((Command::Pane(PaneCommand::SendInput("\x1b[C".into())), false))
    );
    assert_eq!(d.dispatch(press(KeyCode::Left)), Some((Command::Pane(PaneCommand::SendInput("\x1b[D".into())), false)));
}

#[test]
fn insert_mode_enter_sends_carriage_return() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);
    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some((Command::Pane(PaneCommand::SendInput("\r".into())), false)));
}

#[test]
fn insert_mode_backspace_sends_del() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);
    assert_eq!(
        d.dispatch(press(KeyCode::Backspace)),
        Some((Command::Pane(PaneCommand::SendInput("\x7f".into())), false))
    );
}

#[test]
fn insert_mode_special_keys() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::Insert);

    assert_eq!(d.dispatch(press(KeyCode::Home)), Some((Command::Pane(PaneCommand::SendInput("\x1b[H".into())), false)));
    assert_eq!(d.dispatch(press(KeyCode::End)), Some((Command::Pane(PaneCommand::SendInput("\x1b[F".into())), false)));
    assert_eq!(
        d.dispatch(press(KeyCode::PageUp)),
        Some((Command::Pane(PaneCommand::SendInput("\x1b[5~".into())), false))
    );
    assert_eq!(
        d.dispatch(press(KeyCode::PageDown)),
        Some((Command::Pane(PaneCommand::SendInput("\x1b[6~".into())), false))
    );
    assert_eq!(
        d.dispatch(press(KeyCode::Delete)),
        Some((Command::Pane(PaneCommand::SendInput("\x1b[3~".into())), false))
    );
}

#[test]
fn normal_mode_arrow_keys_not_terminal_input() {
    let d = default_dispatcher();
    let result = d.dispatch(press(KeyCode::Up));
    assert_ne!(result, Some((Command::Pane(PaneCommand::SendInput("\x1b[A".into())), false)));
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
fn parse_ctrl_alt_modifier() {
    let key = parse_key_string("ctrl+alt+d").unwrap();
    assert_eq!(key.code, KeyCode::Char('d'));
    assert!(key.modifiers.contains(KeyModifiers::CONTROL));
    assert!(key.modifiers.contains(KeyModifiers::ALT));
}

#[test]
fn parse_uppercase_char_adds_shift() {
    let key = parse_key_string("G").unwrap();
    assert_eq!(key.code, KeyCode::Char('G'));
    assert!(key.modifiers.contains(KeyModifiers::SHIFT));
}

#[test]
fn parse_shift_g_produces_uppercase_with_shift() {
    let key = parse_key_string("shift+g").unwrap();
    assert_eq!(key.code, KeyCode::Char('G'));
    assert!(key.modifiers.contains(KeyModifiers::SHIFT));
}

#[test]
fn uppercase_and_shift_g_canonicalize_same() {
    let upper = parse_key_string("G").unwrap();
    let shift = parse_key_string("shift+g").unwrap();
    assert_eq!(upper, shift);
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
    assert_eq!(d.dispatch(press_mod(KeyCode::Tab, KeyModifiers::SHIFT)), Some((Command::FocusPrevPane, false)));
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
    assert!(keys.contains(&"ctrl+q") || keys.contains(&"?"));
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
    assert_eq!(d.dispatch(press(KeyCode::Char('1'))), Some((Command::GoToTab(1), false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('5'))), Some((Command::GoToTab(5), false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('9'))), Some((Command::GoToTab(9), false)));
}

#[test]
fn focus_direction_dispatch() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(alt(KeyCode::Up)), Some((Command::FocusDirection(Direction::Up), false)));
    assert_eq!(d.dispatch(alt(KeyCode::Down)), Some((Command::FocusDirection(Direction::Down), false)));
    assert_eq!(d.dispatch(alt(KeyCode::Left)), Some((Command::FocusDirection(Direction::Left), false)));
    assert_eq!(d.dispatch(alt(KeyCode::Right)), Some((Command::FocusDirection(Direction::Right), false)));
}

#[test]
fn resize_dispatch() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(alt(KeyCode::Char('k'))), Some((Command::ResizeGrow, false)));
    assert_eq!(d.dispatch(alt(KeyCode::Char('j'))), Some((Command::ResizeShrink, false)));
}

#[test]
fn fullscreen_dispatch() {
    let d = default_dispatcher();
    assert_eq!(d.dispatch(alt(KeyCode::Char('f'))), Some((Command::ToggleFullscreen, false)));
}

#[test]
fn namespace_mode_dispatches_nav_and_input() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::NamespaceSelector);

    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some((Command::NamespaceConfirm, false)));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some((Command::ExitMode, false)));
    assert_eq!(d.dispatch(press(KeyCode::Up)), Some((Command::Pane(PaneCommand::SelectPrev), false)));
    assert_eq!(d.dispatch(press(KeyCode::Down)), Some((Command::Pane(PaneCommand::SelectNext), false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('a'))), Some((Command::NamespaceInput('a'), false)));
    assert_eq!(d.dispatch(press(KeyCode::Backspace)), Some((Command::NamespaceBackspace, false)));
}

#[test]
fn context_mode_dispatches_nav_and_input() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::ContextSelector);

    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some((Command::ContextConfirm, false)));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some((Command::ExitMode, false)));
    assert_eq!(d.dispatch(press(KeyCode::Up)), Some((Command::Pane(PaneCommand::SelectPrev), false)));
    assert_eq!(d.dispatch(press(KeyCode::Down)), Some((Command::Pane(PaneCommand::SelectNext), false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('a'))), Some((Command::ContextInput('a'), false)));
    assert_eq!(d.dispatch(press(KeyCode::Backspace)), Some((Command::ContextBackspace, false)));
}

#[test]
fn namespace_mode_global_bindings_still_active() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::NamespaceSelector);
    assert_eq!(d.dispatch(ctrl(KeyCode::Char('q'))), Some((Command::Quit, false)));
}

#[test]
fn resource_switcher_mode_accepts_input_backspace_confirm_esc() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::ResourceSwitcher);

    assert_eq!(d.dispatch(press(KeyCode::Char('p'))), Some((Command::ResourceSwitcherInput('p'), false)));
    assert_eq!(d.dispatch(press(KeyCode::Backspace)), Some((Command::ResourceSwitcherBackspace, false)));
    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some((Command::ResourceSwitcherConfirm, false)));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some((Command::DenyAction, false)));
}

#[test]
fn resource_switcher_mode_ignores_global_bindings() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::ResourceSwitcher);
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), Some((Command::ResourceSwitcherInput('q'), false)));
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

    assert_eq!(d.dispatch(press(KeyCode::Char('y'))), Some((Command::ConfirmAction, false)));
    assert_eq!(d.dispatch(press(KeyCode::Char('n'))), Some((Command::DenyAction, false)));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some((Command::DenyAction, false)));
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

    assert_eq!(d.dispatch(press(KeyCode::Char('a'))), Some((Command::FilterInput('a'), false)));
    assert_eq!(d.dispatch(press(KeyCode::Backspace)), Some((Command::FilterBackspace, false)));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some((Command::FilterCancel, false)));
    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some((Command::ExitMode, false)));
}

#[test]
fn filter_input_mode_ignores_global_bindings() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::FilterInput);
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), Some((Command::FilterInput('q'), false)));
}

#[test]
fn port_forward_input_mode_handles_edit_confirm_cancel() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::PortForwardInput);

    assert_eq!(d.dispatch(press(KeyCode::Char('3'))), Some((Command::PortForwardInput('3'), false)));
    assert_eq!(d.dispatch(press(KeyCode::Backspace)), Some((Command::PortForwardBackspace, false)));
    assert_eq!(d.dispatch(press(KeyCode::Tab)), Some((Command::PortForwardToggleField, false)));
    assert_eq!(d.dispatch(press(KeyCode::Enter)), Some((Command::PortForwardConfirm, false)));
    assert_eq!(d.dispatch(press(KeyCode::Esc)), Some((Command::PortForwardCancel, false)));
}

#[test]
fn port_forward_input_mode_ignores_non_digits() {
    let mut d = default_dispatcher();
    d.set_mode(InputMode::PortForwardInput);
    assert_eq!(d.dispatch(press(KeyCode::Char('q'))), None);
}

#[test]
fn mutate_command_config_names_map_correctly() {
    let mut config = KeybindingsConfig::default();
    config.mutate.insert("delete".into(), "f3".into());
    config.mutate.insert("scale".into(), "f4".into());
    config.mutate.insert("restart_rollout".into(), "f5".into());

    let d = KeybindingDispatcher::from_config(&config);
    assert_eq!(d.dispatch(press(KeyCode::F(3))), Some((Command::DeleteResource, true)));
    assert_eq!(d.dispatch(press(KeyCode::F(4))), Some((Command::ScaleResource, true)));
    assert_eq!(d.dispatch(press(KeyCode::F(5))), Some((Command::RestartRollout, true)));
}

#[test]
fn interact_command_config_names_map_correctly() {
    let mut config = KeybindingsConfig::default();
    config.interact.insert("exec".into(), "f7".into());
    config.interact.insert("port_forward".into(), "f8".into());

    let d = KeybindingDispatcher::from_config(&config);
    assert_eq!(d.dispatch(press(KeyCode::F(7))), Some((Command::ExecInto, false)));
    assert_eq!(d.dispatch(press(KeyCode::F(8))), Some((Command::PortForward, false)));
}

#[test]
fn browse_command_config_names_map_correctly() {
    let mut config = KeybindingsConfig::default();
    config.browse.insert("view_yaml".into(), "f1".into());
    config.browse.insert("view_describe".into(), "f2".into());
    config.browse.insert("view_logs".into(), "f6".into());
    config.browse.insert("toggle_all_namespaces".into(), "f9".into());
    config.browse.insert("sort_column".into(), "f10".into());
    config.browse.insert("filter".into(), "f11".into());
    config.browse.insert("resource_switcher".into(), "f12".into());

    let d = KeybindingDispatcher::from_config(&config);
    assert_eq!(d.dispatch(press(KeyCode::F(1))), Some((Command::ViewYaml, false)));
    assert_eq!(d.dispatch(press(KeyCode::F(2))), Some((Command::ViewDescribe, false)));
    assert_eq!(d.dispatch(press(KeyCode::F(6))), Some((Command::ViewLogs, false)));
    assert_eq!(d.dispatch(press(KeyCode::F(9))), Some((Command::ToggleAllNamespaces, false)));
    assert_eq!(d.dispatch(press(KeyCode::F(10))), Some((Command::SortByColumn, false)));
    assert_eq!(d.dispatch(press(KeyCode::F(11))), Some((Command::EnterMode(InputMode::FilterInput), false)));
    assert_eq!(d.dispatch(press(KeyCode::F(12))), Some((Command::EnterResourceSwitcher, false)));
}

#[test]
fn all_bindings_returns_entries_for_all_groups() {
    let d = default_dispatcher();
    let bindings = d.all_bindings();
    assert!(!bindings.is_empty());

    let groups: Vec<&str> = bindings.iter().map(|(g, _, _)| g.as_str()).collect();
    assert!(groups.contains(&"Global"));
    assert!(groups.contains(&"Mutate"));
    assert!(groups.contains(&"Interact"));
    assert!(groups.contains(&"Browse"));
    assert!(groups.contains(&"Navigation"));
    assert!(groups.contains(&"TUI"));
}

#[test]
fn from_config_builds_all_five_maps() {
    let config = crystal_config::Config::load();
    let d = KeybindingDispatcher::from_config(&config.keybindings);

    assert!(!d.global_shortcuts().is_empty());
    assert!(!d.mutate_shortcuts().is_empty());
    assert!(!d.browse_shortcuts().is_empty());
    assert!(!d.navigation_shortcuts().is_empty());
    assert!(!d.tui_shortcuts().is_empty());
}
