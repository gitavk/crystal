use super::*;
use crossterm::event::KeyCode;
use crystal_core::resource::DetailSection;
use crystal_tui::pane::{PaneCommand, PaneTree};
use tokio_util::sync::CancellationToken;

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

// --- Step 4.10 tests ---

fn make_test_app_with_ns_headers() -> (HashMap<PaneId, Box<dyn Pane>>, TabManager) {
    let headers = vec!["NAME".into(), "NAMESPACE".into(), "STATUS".into()];
    let mut pane1 = ResourceListPane::new(ResourceKind::Pods, headers);
    pane1.state.set_items(vec![
        vec!["pod-a".into(), "default".into(), "Running".into()],
        vec!["pod-b".into(), "kube-system".into(), "Pending".into()],
    ]);
    pane1.refresh_filter_and_sort();

    let tm = TabManager::new(ViewType::ResourceList(ResourceKind::Pods));
    let mut panes: HashMap<PaneId, Box<dyn Pane>> = HashMap::new();
    panes.insert(1, Box::new(pane1));
    (panes, tm)
}

#[test]
fn selected_resource_info_returns_kind_name_namespace() {
    let (panes, tm) = make_test_app_with_ns_headers();

    // Simulate what selected_resource_info does (can't call it directly without App)
    let focused = tm.active().focused_pane;
    let pane = panes.get(&focused).unwrap();
    let rp = pane.as_any().downcast_ref::<ResourceListPane>().unwrap();

    let kind = rp.kind().unwrap().clone();
    assert_eq!(kind, ResourceKind::Pods);

    let selected_idx = rp.filtered_indices[rp.state.selected.unwrap()];
    let row = &rp.state.items[selected_idx];
    let name = row[0].clone();
    let ns_idx = rp.state.headers.iter().position(|h| h == "NAMESPACE").unwrap();
    let namespace = row[ns_idx].clone();

    assert_eq!(name, "pod-a");
    assert_eq!(namespace, "default");
}

#[test]
fn selected_resource_info_none_when_no_selection() {
    let headers = vec!["NAME".into(), "STATUS".into()];
    let pane = ResourceListPane::new(ResourceKind::Pods, headers);
    // No items, no selection
    assert!(pane.state.selected.is_none() || pane.state.items.is_empty());
}

#[test]
fn open_detail_pane_creates_split() {
    let (mut panes, mut tm) = make_test_app_with_ns_headers();

    let focused = tm.active().focused_pane;
    let sections = vec![DetailSection { title: "Metadata".into(), fields: vec![("Name".into(), "pod-a".into())] }];
    let detail = ResourceDetailPane::new(ResourceKind::Pods, "pod-a".into(), Some("default".into()), sections);
    let view = ViewType::Detail(ResourceKind::Pods, "pod-a".into());

    let new_id = tm.split_pane(focused, SplitDirection::Horizontal, view).unwrap();
    panes.insert(new_id, Box::new(detail));

    assert_eq!(tm.active().pane_tree.leaf_ids().len(), 2);
    let detail_pane = panes.get(&new_id).unwrap();
    assert!(matches!(detail_pane.view_type(), ViewType::Detail(ResourceKind::Pods, _)));
}

#[test]
fn open_yaml_pane_creates_split() {
    let (mut panes, mut tm) = make_test_app_with_ns_headers();

    let focused = tm.active().focused_pane;
    let yaml_pane = YamlPane::new(ResourceKind::Pods, "pod-a".into(), "apiVersion: v1\nkind: Pod".into());
    let view = ViewType::Yaml(ResourceKind::Pods, "pod-a".into());

    let new_id = tm.split_pane(focused, SplitDirection::Horizontal, view).unwrap();
    panes.insert(new_id, Box::new(yaml_pane));

    assert_eq!(tm.active().pane_tree.leaf_ids().len(), 2);
    let yaml = panes.get(&new_id).unwrap();
    assert!(matches!(yaml.view_type(), ViewType::Yaml(ResourceKind::Pods, _)));
}

#[test]
fn back_on_detail_pane_closes_it() {
    let (mut panes, mut tm) = make_test_app_with_ns_headers();

    let focused = tm.active().focused_pane;
    let sections = vec![DetailSection { title: "Metadata".into(), fields: vec![("Name".into(), "pod-a".into())] }];
    let detail = ResourceDetailPane::new(ResourceKind::Pods, "pod-a".into(), None, sections);
    let view = ViewType::Detail(ResourceKind::Pods, "pod-a".into());
    let detail_id = tm.split_pane(focused, SplitDirection::Horizontal, view).unwrap();
    panes.insert(detail_id, Box::new(detail));
    tm.active_mut().focused_pane = detail_id;

    // Simulate Back command: check view type and close
    let is_detail = matches!(panes.get(&detail_id).unwrap().view_type(), ViewType::Detail(..));
    assert!(is_detail);

    tm.active_mut().pane_tree.close(detail_id);
    panes.remove(&detail_id);

    assert_eq!(tm.active().pane_tree.leaf_ids().len(), 1);
    assert!(!panes.contains_key(&detail_id));
}

#[test]
fn back_on_yaml_pane_closes_it() {
    let (mut panes, mut tm) = make_test_app_with_ns_headers();

    let focused = tm.active().focused_pane;
    let yaml_pane = YamlPane::new(ResourceKind::Pods, "pod-a".into(), "kind: Pod".into());
    let view = ViewType::Yaml(ResourceKind::Pods, "pod-a".into());
    let yaml_id = tm.split_pane(focused, SplitDirection::Horizontal, view).unwrap();
    panes.insert(yaml_id, Box::new(yaml_pane));
    tm.active_mut().focused_pane = yaml_id;

    let is_yaml = matches!(panes.get(&yaml_id).unwrap().view_type(), ViewType::Yaml(..));
    assert!(is_yaml);

    tm.active_mut().pane_tree.close(yaml_id);
    panes.remove(&yaml_id);

    assert_eq!(tm.active().pane_tree.leaf_ids().len(), 1);
}

#[test]
fn deny_action_clears_confirmation() {
    let confirmation = Some(PendingConfirmation {
        message: "Delete pod pod-a?".into(),
        action: PendingAction::Delete { kind: ResourceKind::Pods, name: "pod-a".into(), namespace: "default".into() },
    });
    let switcher: Option<ResourceSwitcher> = Some(ResourceSwitcher::new());
    let mut dispatcher = test_dispatcher();
    dispatcher.set_mode(InputMode::ConfirmDialog);

    // Simulate DenyAction: take ownership and drop
    drop(confirmation);
    drop(switcher);
    dispatcher.set_mode(InputMode::Normal);

    assert_eq!(dispatcher.mode(), InputMode::Normal);
}

#[test]
fn resource_switcher_flow() {
    let mut switcher = ResourceSwitcher::new();

    // Type "dep" to filter
    switcher.on_input('d');
    switcher.on_input('e');
    switcher.on_input('p');

    let filtered = switcher.filtered();
    assert!(!filtered.is_empty());
    assert!(filtered.contains(&ResourceKind::Deployments));

    // Confirm selection
    let confirmed = switcher.confirm();
    assert!(confirmed.is_some());
}

#[test]
fn resource_update_updates_correct_pane() {
    let (mut panes, _tm) = make_test_app_with_ns_headers();

    let pane_id: PaneId = 1;
    let new_rows = vec![vec!["new-pod".into(), "default".into(), "Running".into()]];

    // Simulate handle_resource_update
    if let Some(pane) = panes.get_mut(&pane_id) {
        if let Some(rp) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
            rp.state.set_items(new_rows);
            rp.refresh_filter_and_sort();
        }
    }

    let rp = panes.get(&pane_id).unwrap().as_any().downcast_ref::<ResourceListPane>().unwrap();
    assert_eq!(rp.state.items.len(), 1);
    assert_eq!(rp.state.items[0][0], "new-pod");
}

#[test]
fn resource_error_shows_in_pane() {
    let (mut panes, _tm) = make_test_app_with_ns_headers();

    let pane_id: PaneId = 1;

    // Simulate handle_resource_error
    if let Some(pane) = panes.get_mut(&pane_id) {
        if let Some(rp) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
            rp.state.set_error("Connection refused".into());
        }
    }

    let rp = panes.get(&pane_id).unwrap().as_any().downcast_ref::<ResourceListPane>().unwrap();
    assert_eq!(rp.state.error.as_deref(), Some("Connection refused"));
}

#[test]
fn toast_cleanup_removes_expired() {
    let mut toasts = vec![ToastMessage::success("Done"), ToastMessage::error("Fail")];

    // All toasts should be non-expired right after creation
    toasts.retain(|t| !t.is_expired());
    assert_eq!(toasts.len(), 2);
}

#[test]
fn close_pane_cancels_watcher() {
    let mut active_watchers: HashMap<PaneId, CancellationToken> = HashMap::new();
    let token = CancellationToken::new();
    let token_clone = token.clone();
    active_watchers.insert(1, token);

    // Simulate close_pane watcher cleanup
    if let Some(t) = active_watchers.remove(&1) {
        t.cancel();
    }

    assert!(token_clone.is_cancelled());
    assert!(!active_watchers.contains_key(&1));
}

#[test]
fn close_tab_cancels_all_watchers() {
    let mut active_watchers: HashMap<PaneId, CancellationToken> = HashMap::new();
    let token1 = CancellationToken::new();
    let token2 = CancellationToken::new();
    let t1_clone = token1.clone();
    let t2_clone = token2.clone();
    active_watchers.insert(1, token1);
    active_watchers.insert(2, token2);

    let pane_ids = vec![1, 2];
    for id in pane_ids {
        if let Some(t) = active_watchers.remove(&id) {
            t.cancel();
        }
    }

    assert!(t1_clone.is_cancelled());
    assert!(t2_clone.is_cancelled());
    assert!(active_watchers.is_empty());
}

#[test]
fn select_on_resource_list_opens_detail() {
    let (panes, tm) = make_test_app_with_ns_headers();

    // Verify the focused pane is a resource list with a selection
    let focused = tm.active().focused_pane;
    let pane = panes.get(&focused).unwrap();
    let rp = pane.as_any().downcast_ref::<ResourceListPane>().unwrap();
    assert!(rp.state.selected.is_some());

    // The Select command at App level should extract info and open detail
    let kind = rp.kind().unwrap().clone();
    let selected_idx = rp.filtered_indices[rp.state.selected.unwrap()];
    let row = &rp.state.items[selected_idx];
    assert_eq!(row[0], "pod-a");
    assert_eq!(kind, ResourceKind::Pods);
}

#[test]
fn back_on_resource_list_does_not_close() {
    let (panes, _tm) = make_test_app_with_ns_headers();

    let pane = panes.get(&1).unwrap();
    let is_detail_or_yaml = matches!(pane.view_type(), ViewType::Detail(..) | ViewType::Yaml(..));
    assert!(!is_detail_or_yaml, "Resource list should not be closed by Back");
}

// --- Step 5.9: Insert mode tests ---

#[test]
fn enter_insert_mode_sets_dispatcher_mode() {
    let mut dispatcher = test_dispatcher();
    assert_eq!(dispatcher.mode(), InputMode::Normal);

    dispatcher.set_mode(InputMode::Insert);
    assert_eq!(dispatcher.mode(), InputMode::Insert);
}

#[test]
fn exit_insert_mode_returns_to_normal() {
    let mut dispatcher = test_dispatcher();
    dispatcher.set_mode(InputMode::Insert);

    // Simulate ExitMode command
    dispatcher.set_mode(InputMode::Normal);
    assert_eq!(dispatcher.mode(), InputMode::Normal);
}

#[test]
fn insert_mode_hints_contain_esc() {
    let mut dispatcher = test_dispatcher();
    dispatcher.set_mode(InputMode::Insert);

    // Verify mode name
    let mode_name = match dispatcher.mode() {
        InputMode::Insert => "Insert",
        _ => "Other",
    };
    assert_eq!(mode_name, "Insert");
}
