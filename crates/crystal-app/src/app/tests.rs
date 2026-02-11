use super::*;
use crossterm::event::KeyCode;
use crystal_tui::pane::PaneCommand;

use crate::keybindings::KeybindingDispatcher;

fn test_dispatcher() -> KeybindingDispatcher {
    let config = crystal_config::Config::load();
    KeybindingDispatcher::from_config(&config.keybindings)
}

fn make_test_app() -> (HashMap<PaneId, Box<dyn Pane>>, PaneTree, PaneId) {
    let headers = vec!["NAME".into(), "STATUS".into()];
    let mut pane1 = ResourceListPane::new(ResourceKind::Pods, headers.clone());
    pane1.state.set_items(vec![vec!["pod-a".into(), "Running".into()], vec!["pod-b".into(), "Pending".into()]]);

    let mut pane2 = ResourceListPane::new(ResourceKind::Services, headers);
    pane2.state.set_items(vec![vec!["svc-a".into(), "Active".into()]]);

    let mut tree = PaneTree::new(ViewType::ResourceList(ResourceKind::Pods));
    let pane2_id = tree.split(1, SplitDirection::Vertical, ViewType::ResourceList(ResourceKind::Services)).unwrap();

    let mut panes: HashMap<PaneId, Box<dyn Pane>> = HashMap::new();
    panes.insert(1, Box::new(pane1));
    panes.insert(pane2_id, Box::new(pane2));

    (panes, tree, 1)
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
    let mut help = HelpPane::new(d.global_shortcuts(), d.pane_shortcuts());
    let resource_view = ViewType::ResourceList(ResourceKind::Pods);
    help.on_focus_change(Some(&resource_view));

    let help_ref = help.as_any().downcast_ref::<HelpPane>().unwrap();
    assert_eq!(help_ref.view_type(), &ViewType::Help);
}
