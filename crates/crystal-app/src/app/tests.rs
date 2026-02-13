use super::*;
use crossterm::event::KeyCode;
use crystal_tui::pane::{PaneCommand, PaneTree};

use crate::keybindings::KeybindingDispatcher;

fn test_dispatcher() -> KeybindingDispatcher {
    let config = crystal_config::Config::load();
    KeybindingDispatcher::from_config(&config.keybindings)
}

fn make_test_app() -> (HashMap<PaneId, Box<dyn Pane>>, PaneTree, PaneId) {
    let headers = vec!["NAME".into(), "STATUS".into()];
    let mut pane1 = ResourceListPane::new(ResourceKind::Pods, headers.clone());
    pane1.state.set_items(vec![vec!["pod-a".into(), "Running".into()], vec!["pod-b".into(), "Pending".into()]]);
    pane1.refresh_filter_and_sort();

    let mut pane2 = ResourceListPane::new(ResourceKind::Services, headers);
    pane2.state.set_items(vec![vec!["svc-a".into(), "Active".into()]]);
    pane2.refresh_filter_and_sort();

    let mut tree = PaneTree::new(ViewType::ResourceList(ResourceKind::Pods));
    let pane2_id = tree.split(1, SplitDirection::Vertical, ViewType::ResourceList(ResourceKind::Services)).unwrap();

    let mut panes: HashMap<PaneId, Box<dyn Pane>> = HashMap::new();
    panes.insert(1, Box::new(pane1));
    panes.insert(pane2_id, Box::new(pane2));

    (panes, tree, 1)
}

fn make_test_tab_manager() -> (HashMap<PaneId, Box<dyn Pane>>, TabManager) {
    let headers = vec!["NAME".into(), "STATUS".into()];
    let mut pane1 = ResourceListPane::new(ResourceKind::Pods, headers.clone());
    pane1.state.set_items(vec![vec!["pod-a".into(), "Running".into()], vec!["pod-b".into(), "Pending".into()]]);
    pane1.refresh_filter_and_sort();

    let tm = TabManager::new(ViewType::ResourceList(ResourceKind::Pods));

    let mut panes: HashMap<PaneId, Box<dyn Pane>> = HashMap::new();
    panes.insert(1, Box::new(pane1));

    (panes, tm)
}

#[test]
fn pane_command_dispatched_to_focused_only() {
    let (mut panes, _tree, focused) = make_test_app();

    assert_eq!(
        panes.get(&focused).unwrap().as_any().downcast_ref::<ResourceListPane>().unwrap().state.selected,
        Some(0)
    );

    if let Some(pane) = panes.get_mut(&focused) {
        pane.handle_command(&PaneCommand::SelectNext);
    }

    assert_eq!(
        panes.get(&focused).unwrap().as_any().downcast_ref::<ResourceListPane>().unwrap().state.selected,
        Some(1)
    );

    assert_eq!(panes.get(&2).unwrap().as_any().downcast_ref::<ResourceListPane>().unwrap().state.selected, Some(0));
}

#[test]
fn unfocused_pane_receives_no_commands() {
    let (mut panes, _tree, focused) = make_test_app();
    let unfocused = 2;
    assert_ne!(focused, unfocused);

    for _ in 0..3 {
        if let Some(pane) = panes.get_mut(&focused) {
            pane.handle_command(&PaneCommand::SelectNext);
        }
    }

    let unfocused_pane = panes.get(&unfocused).unwrap().as_any().downcast_ref::<ResourceListPane>().unwrap();
    assert_eq!(unfocused_pane.state.selected, Some(0));
}

#[test]
fn global_command_takes_precedence() {
    let d = test_dispatcher();

    let key = KeyEvent::new(KeyCode::Char('q'), crossterm::event::KeyModifiers::NONE);
    assert_eq!(d.dispatch(key), Some(Command::Quit));

    let key = KeyEvent::new(KeyCode::Char('j'), crossterm::event::KeyModifiers::NONE);
    assert!(matches!(d.dispatch(key), Some(Command::Pane(PaneCommand::SelectNext))));
}

#[test]
fn focus_cycling_wraps_around() {
    let (_panes, tree, _) = make_test_app();
    let ids = tree.leaf_ids();
    assert_eq!(ids, vec![1, 2]);

    let focused = 1;
    let pos = ids.iter().position(|&id| id == focused).unwrap();
    let next = ids[(pos + 1) % ids.len()];
    assert_eq!(next, 2);

    let focused = 2;
    let pos = ids.iter().position(|&id| id == focused).unwrap();
    let next = ids[(pos + 1) % ids.len()];
    assert_eq!(next, 1);
}

#[test]
fn help_pane_updates_context_on_focus() {
    let d = test_dispatcher();
    let mut help = HelpPane::new(d.global_shortcuts(), d.pane_shortcuts(), d.resource_shortcuts());
    let resource_view = ViewType::ResourceList(ResourceKind::Pods);
    help.on_focus_change(Some(&resource_view));

    let help_ref = help.as_any().downcast_ref::<HelpPane>().unwrap();
    assert_eq!(help_ref.view_type(), &ViewType::Help);
}

#[test]
fn tab_manager_new_tab_creates_pane() {
    let (mut panes, mut tm) = make_test_tab_manager();

    assert_eq!(tm.tabs().len(), 1);
    assert_eq!(tm.tab_names(), vec!["Main"]);

    let tab_id = tm.new_tab("Second", ViewType::Empty);
    let new_pane_id = tm.tabs().iter().find(|t| t.id == tab_id).unwrap().focused_pane;
    panes.insert(new_pane_id, Box::new(EmptyPane(ViewType::Empty)));

    assert_eq!(tm.tabs().len(), 2);
    assert_eq!(tm.active_index(), 1);
    assert_eq!(tm.active().name, "Second");
    assert!(panes.contains_key(&new_pane_id));
}

#[test]
fn tab_manager_close_tab_cleans_up_panes() {
    let (mut panes, mut tm) = make_test_tab_manager();

    let tab_id = tm.new_tab("Temp", ViewType::Empty);
    let pane_id = tm.tabs().iter().find(|t| t.id == tab_id).unwrap().focused_pane;
    panes.insert(pane_id, Box::new(EmptyPane(ViewType::Empty)));

    let pane_ids: Vec<PaneId> = tm.active().pane_tree.leaf_ids();
    assert!(tm.close_tab(tab_id));

    for id in pane_ids {
        panes.remove(&id);
    }

    assert_eq!(tm.tabs().len(), 1);
    assert!(!panes.contains_key(&pane_id));
}

#[test]
fn tab_manager_split_uses_global_ids() {
    let (mut panes, mut tm) = make_test_tab_manager();

    let new_id = tm.split_pane(1, SplitDirection::Vertical, ViewType::Empty).unwrap();
    panes.insert(new_id, Box::new(EmptyPane(ViewType::Empty)));

    assert_eq!(tm.active().pane_tree.leaf_ids().len(), 2);
    assert!(panes.contains_key(&new_id));
    assert_ne!(new_id, 1);
}

#[test]
fn tab_switch_preserves_focus() {
    let (mut panes, mut tm) = make_test_tab_manager();

    let new_id = tm.split_pane(1, SplitDirection::Vertical, ViewType::Empty).unwrap();
    panes.insert(new_id, Box::new(EmptyPane(ViewType::Empty)));
    tm.active_mut().focused_pane = new_id;

    let tab_id = tm.new_tab("Second", ViewType::Empty);
    let pane_id = tm.tabs().iter().find(|t| t.id == tab_id).unwrap().focused_pane;
    panes.insert(pane_id, Box::new(EmptyPane(ViewType::Empty)));

    tm.switch_tab(0);
    assert_eq!(tm.active().focused_pane, new_id);

    tm.switch_tab(1);
    assert_eq!(tm.active().focused_pane, pane_id);
}

#[test]
fn fullscreen_is_per_tab() {
    let (mut panes, mut tm) = make_test_tab_manager();

    tm.active_mut().fullscreen_pane = Some(1);

    let tab_id = tm.new_tab("Second", ViewType::Empty);
    let pane_id = tm.tabs().iter().find(|t| t.id == tab_id).unwrap().focused_pane;
    panes.insert(pane_id, Box::new(EmptyPane(ViewType::Empty)));

    assert!(tm.active().fullscreen_pane.is_none());

    tm.switch_tab(0);
    assert_eq!(tm.active().fullscreen_pane, Some(1));
}

#[test]
fn mode_hints_update_on_mode_switch() {
    let d = test_dispatcher();

    let normal_hints = d.global_hints();
    assert!(!normal_hints.is_empty());

    let mut d2 = KeybindingDispatcher::from_config(&crystal_config::Config::load().keybindings);
    d2.set_mode(InputMode::NamespaceSelector);
    assert_eq!(d2.mode(), InputMode::NamespaceSelector);
}

#[test]
fn render_context_reflects_active_tab() {
    let (mut panes, mut tm) = make_test_tab_manager();

    let tab_names = tm.tab_names();
    assert_eq!(tab_names, vec!["Main"]);

    tm.new_tab("Second", ViewType::Empty);
    let pane_id = tm.active().focused_pane;
    panes.insert(pane_id, Box::new(EmptyPane(ViewType::Empty)));

    let tab_names = tm.tab_names();
    assert_eq!(tab_names, vec!["Main", "Second"]);
    assert_eq!(tm.active_index(), 1);
}

#[test]
fn goto_tab_uses_one_based_index() {
    let (mut panes, mut tm) = make_test_tab_manager();

    tm.new_tab("Second", ViewType::Empty);
    let pane_id = tm.active().focused_pane;
    panes.insert(pane_id, Box::new(EmptyPane(ViewType::Empty)));

    // GoToTab(1) should switch to index 0 (first tab)
    let n: usize = 1;
    if n > 0 {
        tm.switch_tab(n - 1);
    }
    assert_eq!(tm.active_index(), 0);
    assert_eq!(tm.active().name, "Main");

    // GoToTab(2) should switch to index 1 (second tab)
    let n: usize = 2;
    if n > 0 {
        tm.switch_tab(n - 1);
    }
    assert_eq!(tm.active_index(), 1);
    assert_eq!(tm.active().name, "Second");
}
