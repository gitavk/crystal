use crystal_tui::pane::{Pane, PaneCommand, ResourceKind};

use super::ResourceListPane;

fn sample_pane() -> ResourceListPane {
    let mut pane = ResourceListPane::new(ResourceKind::Pods, vec!["NAME".into(), "NAMESPACE".into(), "STATUS".into()]);
    pane.state.set_items(vec![
        vec!["nginx-pod-abc123".into(), "default".into(), "Running".into()],
        vec!["redis-master-0".into(), "cache".into(), "Running".into()],
        vec!["api-gateway-xyz".into(), "default".into(), "Pending".into()],
        vec!["nginx-sidecar-1".into(), "web".into(), "Failed".into()],
    ]);
    pane.refresh_filter_and_sort();
    pane
}

#[test]
fn filter_by_name_matches_substring() {
    let mut pane = sample_pane();
    pane.handle_command(&PaneCommand::Filter("ngi".into()));
    assert_eq!(pane.filtered_indices, vec![0, 3]);
}

#[test]
fn filter_matches_across_any_column() {
    let mut pane = sample_pane();
    pane.handle_command(&PaneCommand::Filter("cache".into()));
    assert_eq!(pane.filtered_indices, vec![1]);
}

#[test]
fn empty_filter_shows_all_items() {
    let mut pane = sample_pane();
    pane.handle_command(&PaneCommand::Filter("ngi".into()));
    assert_eq!(pane.filtered_indices.len(), 2);
    pane.handle_command(&PaneCommand::ClearFilter);
    assert_eq!(pane.filtered_indices.len(), 4);
}

#[test]
fn sort_by_column_ascending() {
    let mut pane = sample_pane();
    pane.sort_by_column(0);
    let names: Vec<&str> = pane.filtered_indices.iter().map(|&i| pane.state.items[i][0].as_str()).collect();
    assert_eq!(names, vec!["api-gateway-xyz", "nginx-pod-abc123", "nginx-sidecar-1", "redis-master-0"]);
    assert!(pane.sort_ascending);
}

#[test]
fn sort_toggle_flips_direction() {
    let mut pane = sample_pane();
    pane.sort_by_column(0);
    assert!(pane.sort_ascending);
    pane.sort_by_column(0);
    assert!(!pane.sort_ascending);
    let names: Vec<&str> = pane.filtered_indices.iter().map(|&i| pane.state.items[i][0].as_str()).collect();
    assert_eq!(names, vec!["redis-master-0", "nginx-sidecar-1", "nginx-pod-abc123", "api-gateway-xyz"]);
}

#[test]
fn different_column_resets_to_ascending() {
    let mut pane = sample_pane();
    pane.sort_by_column(0);
    pane.sort_by_column(0); // now descending
    assert!(!pane.sort_ascending);
    pane.sort_by_column(1); // switch column → ascending
    assert!(pane.sort_ascending);
    assert_eq!(pane.sort_column, Some(1));
}

#[test]
fn filter_then_sort_composes() {
    let mut pane = sample_pane();
    pane.handle_command(&PaneCommand::Filter("nginx".into()));
    assert_eq!(pane.filtered_indices.len(), 2);
    pane.sort_by_column(0);
    let names: Vec<&str> = pane.filtered_indices.iter().map(|&i| pane.state.items[i][0].as_str()).collect();
    assert_eq!(names, vec!["nginx-pod-abc123", "nginx-sidecar-1"]);
}

#[test]
fn selection_resets_to_zero_after_filter() {
    let mut pane = sample_pane();
    pane.state.selected = Some(3);
    pane.handle_command(&PaneCommand::Filter("redis".into()));
    assert_eq!(pane.state.selected, Some(0));
}

#[test]
fn selection_none_when_filter_has_no_matches() {
    let mut pane = sample_pane();
    pane.handle_command(&PaneCommand::Filter("nonexistent".into()));
    assert_eq!(pane.state.selected, None);
    assert!(pane.filtered_indices.is_empty());
}

#[test]
fn filtered_indices_stay_in_bounds_on_watcher_update() {
    let mut pane = sample_pane();
    pane.handle_command(&PaneCommand::Filter("nginx".into()));
    assert_eq!(pane.filtered_indices.len(), 2);

    // Simulate watcher update with fewer items
    pane.state.set_items(vec![
        vec!["nginx-new".into(), "default".into(), "Running".into()],
        vec!["other-pod".into(), "default".into(), "Running".into()],
    ]);
    pane.refresh_filter_and_sort();
    assert_eq!(pane.filtered_indices, vec![0]);
    assert_eq!(pane.state.selected, Some(0));
}

#[test]
fn nav_next_wraps_within_filtered() {
    let mut pane = sample_pane();
    pane.handle_command(&PaneCommand::Filter("nginx".into()));
    assert_eq!(pane.state.selected, Some(0));
    pane.handle_command(&PaneCommand::SelectNext);
    assert_eq!(pane.state.selected, Some(1));
    pane.handle_command(&PaneCommand::SelectNext);
    assert_eq!(pane.state.selected, Some(0)); // wraps
}

#[test]
fn nav_prev_wraps_within_filtered() {
    let mut pane = sample_pane();
    pane.handle_command(&PaneCommand::Filter("nginx".into()));
    assert_eq!(pane.state.selected, Some(0));
    pane.handle_command(&PaneCommand::SelectPrev);
    assert_eq!(pane.state.selected, Some(1)); // wraps to last
}

#[test]
fn sort_by_column_via_pane_command() {
    let mut pane = sample_pane();
    pane.handle_command(&PaneCommand::SortByColumn(2));
    let statuses: Vec<&str> = pane.filtered_indices.iter().map(|&i| pane.state.items[i][2].as_str()).collect();
    assert_eq!(statuses, vec!["Failed", "Pending", "Running", "Running"]);
}

#[test]
fn toggle_sort_order_via_pane_command() {
    let mut pane = sample_pane();
    pane.handle_command(&PaneCommand::SortByColumn(0));
    assert!(pane.sort_ascending);
    pane.handle_command(&PaneCommand::ToggleSortOrder);
    assert!(!pane.sort_ascending);
}

#[test]
fn all_namespaces_defaults_to_false() {
    let pane = ResourceListPane::new(ResourceKind::Pods, vec!["NAME".into()]);
    assert!(!pane.all_namespaces);
}

#[test]
fn filter_is_case_insensitive() {
    let mut pane = sample_pane();
    pane.handle_command(&PaneCommand::Filter("NGINX".into()));
    assert_eq!(pane.filtered_indices, vec![0, 3]);
}

#[test]
fn age_column_sorts_by_duration_ascending() {
    let mut pane = ResourceListPane::new(ResourceKind::Pods, vec!["NAME".into(), "AGE".into()]);
    pane.state.set_items(vec![
        vec!["pod-a".into(), "2h".into()],
        vec!["pod-b".into(), "10m".into()],
        vec!["pod-c".into(), "1d".into()],
        vec!["pod-d".into(), "45s".into()],
    ]);
    pane.refresh_filter_and_sort();

    pane.sort_by_column(1);
    let names: Vec<&str> = pane.filtered_indices.iter().map(|&i| pane.state.items[i][0].as_str()).collect();
    assert_eq!(names, vec!["pod-d", "pod-b", "pod-a", "pod-c"]);
}

#[test]
fn age_column_sorts_by_duration_descending() {
    let mut pane = ResourceListPane::new(ResourceKind::Pods, vec!["NAME".into(), "AGE".into()]);
    pane.state.set_items(vec![
        vec!["pod-a".into(), "2h".into()],
        vec!["pod-b".into(), "10m".into()],
        vec!["pod-c".into(), "1d".into()],
        vec!["pod-d".into(), "45s".into()],
    ]);
    pane.refresh_filter_and_sort();

    pane.sort_by_column(1);
    pane.sort_by_column(1);
    let names: Vec<&str> = pane.filtered_indices.iter().map(|&i| pane.state.items[i][0].as_str()).collect();
    assert_eq!(names, vec!["pod-c", "pod-a", "pod-b", "pod-d"]);
}

#[test]
fn restarts_column_sorts_numerically_ascending() {
    let mut pane = ResourceListPane::new(ResourceKind::Pods, vec!["NAME".into(), "RESTARTS".into()]);
    pane.state.set_items(vec![
        vec!["pod-a".into(), "2".into()],
        vec!["pod-b".into(), "10".into()],
        vec!["pod-c".into(), "1".into()],
        vec!["pod-d".into(), "0".into()],
    ]);
    pane.refresh_filter_and_sort();

    pane.sort_by_column(1);
    let names: Vec<&str> = pane.filtered_indices.iter().map(|&i| pane.state.items[i][0].as_str()).collect();
    assert_eq!(names, vec!["pod-d", "pod-c", "pod-a", "pod-b"]);
}

#[test]
fn restarts_column_sorts_numerically_descending() {
    let mut pane = ResourceListPane::new(ResourceKind::Pods, vec!["NAME".into(), "RESTARTS".into()]);
    pane.state.set_items(vec![
        vec!["pod-a".into(), "2".into()],
        vec!["pod-b".into(), "10".into()],
        vec!["pod-c".into(), "1".into()],
        vec!["pod-d".into(), "0".into()],
    ]);
    pane.refresh_filter_and_sort();

    pane.sort_by_column(1);
    pane.sort_by_column(1);
    let names: Vec<&str> = pane.filtered_indices.iter().map(|&i| pane.state.items[i][0].as_str()).collect();
    assert_eq!(names, vec!["pod-b", "pod-a", "pod-c", "pod-d"]);
}
