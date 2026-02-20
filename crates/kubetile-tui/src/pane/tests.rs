use super::*;

fn pods_view() -> ViewType {
    ViewType::ResourceList(ResourceKind::Pods)
}

fn logs_view() -> ViewType {
    ViewType::Logs("nginx".into())
}

fn help_view() -> ViewType {
    ViewType::Help
}

fn area(w: u16, h: u16) -> Rect {
    Rect { x: 0, y: 0, width: w, height: h }
}

fn rect(x: u16, y: u16, w: u16, h: u16) -> Rect {
    Rect { x, y, width: w, height: h }
}

#[test]
fn single_leaf_layout_returns_full_area() {
    let tree = PaneTree::new(pods_view());
    let rects = tree.layout(area(100, 50));
    assert_eq!(rects.len(), 1);
    assert_eq!(rects[0], (1, area(100, 50)));
}

#[test]
fn split_single_leaf_produces_split_with_two_children() {
    let mut tree = PaneTree::new(pods_view());
    let new_id = tree.split(1, SplitDirection::Vertical, logs_view());
    assert_eq!(new_id, Some(2));

    let ids = tree.leaf_ids();
    assert_eq!(ids, vec![1, 2]);

    assert!(matches!(tree.find(1), Some(PaneNode::Leaf { view: ViewType::ResourceList(ResourceKind::Pods), .. })));
    assert!(matches!(tree.find(2), Some(PaneNode::Leaf { view: ViewType::Logs(_), .. })));
    assert!(matches!(tree.root(), PaneNode::Split { direction: SplitDirection::Vertical, .. }));
}

#[test]
fn split_nested_leaf_targets_correct_node() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());
    // Split the second pane (logs) horizontally
    let new_id = tree.split(2, SplitDirection::Horizontal, help_view());
    assert_eq!(new_id, Some(3));

    let ids = tree.leaf_ids();
    assert_eq!(ids, vec![1, 2, 3]);

    // Root should still be vertical split
    assert!(matches!(tree.root(), PaneNode::Split { direction: SplitDirection::Vertical, .. }));
}

#[test]
fn split_nonexistent_pane_returns_none() {
    let mut tree = PaneTree::new(pods_view());
    assert_eq!(tree.split(99, SplitDirection::Vertical, logs_view()), None);
}

#[test]
fn close_promotes_sibling() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());
    assert_eq!(tree.leaf_ids(), vec![1, 2]);

    let closed = tree.close(1);
    assert!(closed);

    let ids = tree.leaf_ids();
    assert_eq!(ids, vec![2]);
    assert!(matches!(tree.root(), PaneNode::Leaf { id: 2, .. }));
}

#[test]
fn close_second_child_promotes_first() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());

    let closed = tree.close(2);
    assert!(closed);
    assert_eq!(tree.leaf_ids(), vec![1]);
    assert!(matches!(tree.root(), PaneNode::Leaf { id: 1, .. }));
}

#[test]
fn close_last_pane_returns_false() {
    let mut tree = PaneTree::new(pods_view());
    let closed = tree.close(1);
    assert!(!closed);
    assert_eq!(tree.leaf_ids(), vec![1]);
}

#[test]
fn close_nested_pane_promotes_sibling() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());
    tree.split(2, SplitDirection::Horizontal, help_view());
    // Tree: Split(V) -> [Leaf(1), Split(H) -> [Leaf(2), Leaf(3)]]
    assert_eq!(tree.leaf_ids(), vec![1, 2, 3]);

    let closed = tree.close(2);
    assert!(closed);
    // Split(H) should be replaced by Leaf(3)
    // Tree: Split(V) -> [Leaf(1), Leaf(3)]
    assert_eq!(tree.leaf_ids(), vec![1, 3]);
}

#[test]
fn layout_vertical_split_divides_width() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());

    let rects = tree.layout(area(100, 50));
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0], (1, Rect { x: 0, y: 0, width: 50, height: 50 }));
    assert_eq!(rects[1], (2, Rect { x: 50, y: 0, width: 50, height: 50 }));
}

#[test]
fn layout_horizontal_split_divides_height() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Horizontal, logs_view());

    let rects = tree.layout(area(100, 50));
    assert_eq!(rects.len(), 2);
    assert_eq!(rects[0], (1, Rect { x: 0, y: 0, width: 100, height: 25 }));
    assert_eq!(rects[1], (2, Rect { x: 0, y: 25, width: 100, height: 25 }));
}

#[test]
fn layout_nested_splits() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());
    tree.split(2, SplitDirection::Horizontal, help_view());

    let rects = tree.layout(area(100, 50));
    assert_eq!(rects.len(), 3);
    // Left half: pane 1
    assert_eq!(rects[0], (1, Rect { x: 0, y: 0, width: 50, height: 50 }));
    // Right half, top: pane 2
    assert_eq!(rects[1], (2, Rect { x: 50, y: 0, width: 50, height: 25 }));
    // Right half, bottom: pane 3
    assert_eq!(rects[2], (3, Rect { x: 50, y: 25, width: 50, height: 25 }));
}

#[test]
fn leaf_ids_depth_first_order() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());
    tree.split(1, SplitDirection::Horizontal, help_view());
    // Tree: Split(V) -> [Split(H) -> [Leaf(1), Leaf(3)], Leaf(2)]
    assert_eq!(tree.leaf_ids(), vec![1, 3, 2]);
}

#[test]
fn resize_clamps_ratio() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());

    // Resize far positive - should clamp to 0.9
    tree.resize(1, 10.0, true);
    let rects = tree.layout(area(100, 50));
    assert_eq!(rects[0].1.width, 90);
    assert_eq!(rects[1].1.width, 10);

    // Resize far negative - should clamp to 0.1
    tree.resize(1, 10.0, false);
    let rects = tree.layout(area(100, 50));
    assert_eq!(rects[0].1.width, 10);
    assert_eq!(rects[1].1.width, 90);
}

#[test]
fn resize_adjusts_ratio() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());

    tree.resize(1, 0.1, true);
    let rects = tree.layout(area(100, 50));
    // 0.5 + 0.1 = 0.6 -> 60 width
    assert_eq!(rects[0].1.width, 60);
    assert_eq!(rects[1].1.width, 40);
}

#[test]
fn find_returns_correct_node() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());

    assert!(tree.find(1).is_some());
    assert!(tree.find(2).is_some());
    assert!(tree.find(99).is_none());
}

#[test]
fn close_nonexistent_pane_returns_false() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());
    assert!(!tree.close(99));
}

// --- Directional navigation tests ---

#[test]
fn direction_right_from_left_pane() {
    let all = vec![(1, rect(0, 0, 50, 50)), (2, rect(50, 0, 50, 50))];
    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Right), Some(2));
}

#[test]
fn direction_left_from_right_pane() {
    let all = vec![(1, rect(0, 0, 50, 50)), (2, rect(50, 0, 50, 50))];
    assert_eq!(find_pane_in_direction((2, all[1].1), &all, Direction::Left), Some(1));
}

#[test]
fn direction_down_from_top_pane() {
    let all = vec![(1, rect(0, 0, 100, 25)), (2, rect(0, 25, 100, 25))];
    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Down), Some(2));
}

#[test]
fn direction_up_from_bottom_pane() {
    let all = vec![(1, rect(0, 0, 100, 25)), (2, rect(0, 25, 100, 25))];
    assert_eq!(find_pane_in_direction((2, all[1].1), &all, Direction::Up), Some(1));
}

#[test]
fn direction_l_shape_finds_correct_neighbors() {
    // Layout:
    // [  1  ] [  2  ]
    // [     3       ]
    let all = vec![(1, rect(0, 0, 50, 25)), (2, rect(50, 0, 50, 25)), (3, rect(0, 25, 100, 25))];

    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Right), Some(2));
    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Down), Some(3));
    assert_eq!(find_pane_in_direction((2, all[1].1), &all, Direction::Left), Some(1));
    assert_eq!(find_pane_in_direction((2, all[1].1), &all, Direction::Down), Some(3));
    assert_eq!(find_pane_in_direction((3, all[2].1), &all, Direction::Up), Some(1));
}

#[test]
fn direction_no_neighbor_returns_none() {
    let all = vec![(1, rect(0, 0, 50, 50)), (2, rect(50, 0, 50, 50))];
    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Left), None);
    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Up), None);
    assert_eq!(find_pane_in_direction((2, all[1].1), &all, Direction::Right), None);
    assert_eq!(find_pane_in_direction((2, all[1].1), &all, Direction::Down), None);
}

#[test]
fn direction_single_pane_returns_none() {
    let all = vec![(1, rect(0, 0, 100, 50))];
    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Right), None);
    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Left), None);
    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Up), None);
    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Down), None);
}

#[test]
fn direction_prefers_overlap_candidate() {
    // Layout:
    // [  1  ] [  2  ]
    //         [  3  ]
    // Pane 3 is below-right, not aligned with pane 1
    // Moving right from 1 should pick 2 (has overlap) over 3
    let all = vec![(1, rect(0, 0, 50, 25)), (2, rect(50, 0, 50, 25)), (3, rect(50, 25, 50, 25))];
    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Right), Some(2));
}

#[test]
fn direction_2x2_grid_navigation() {
    // [1] [2]
    // [3] [4]
    let all =
        vec![(1, rect(0, 0, 50, 25)), (2, rect(50, 0, 50, 25)), (3, rect(0, 25, 50, 25)), (4, rect(50, 25, 50, 25))];

    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Right), Some(2));
    assert_eq!(find_pane_in_direction((1, all[0].1), &all, Direction::Down), Some(3));
    assert_eq!(find_pane_in_direction((4, all[3].1), &all, Direction::Left), Some(3));
    assert_eq!(find_pane_in_direction((4, all[3].1), &all, Direction::Up), Some(2));
    assert_eq!(find_pane_in_direction((3, all[2].1), &all, Direction::Right), Some(4));
    assert_eq!(find_pane_in_direction((2, all[1].1), &all, Direction::Down), Some(4));
}

#[test]
fn focus_cycling_wraps_forward() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());
    tree.split(2, SplitDirection::Horizontal, help_view());
    let ids = tree.leaf_ids();
    // ids = [1, 2, 3]

    let focused = 3;
    let pos = ids.iter().position(|&id| id == focused).unwrap();
    let next = ids[(pos + 1) % ids.len()];
    assert_eq!(next, 1);
}

#[test]
fn focus_cycling_wraps_backward() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());
    tree.split(2, SplitDirection::Horizontal, help_view());
    let ids = tree.leaf_ids();

    let focused = 1;
    let pos = ids.iter().position(|&id| id == focused).unwrap();
    let prev = ids[(pos + ids.len() - 1) % ids.len()];
    assert_eq!(prev, 3);
}

// --- ResourceKind tests ---

#[test]
fn resource_kind_all_returns_14_variants() {
    assert_eq!(ResourceKind::all().len(), 14);
}

#[test]
fn resource_kind_short_names_are_unique() {
    let all = ResourceKind::all();
    let mut names: Vec<&str> = all.iter().map(|k| k.short_name()).collect();
    let count = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), count);
}

#[test]
fn resource_kind_is_namespaced() {
    let cluster_scoped = [ResourceKind::Nodes, ResourceKind::Namespaces, ResourceKind::PersistentVolumes];
    for kind in ResourceKind::all() {
        if cluster_scoped.contains(kind) {
            assert!(!kind.is_namespaced(), "{:?} should be cluster-scoped", kind);
        } else {
            assert!(kind.is_namespaced(), "{:?} should be namespaced", kind);
        }
    }
}

#[test]
fn resource_kind_round_trip_via_short_name() {
    for kind in ResourceKind::all() {
        let short = kind.short_name();
        let resolved = ResourceKind::from_short_name(short);
        assert_eq!(resolved.as_ref(), Some(kind), "round-trip failed for {:?} (short={})", kind, short);
    }
}

#[test]
fn resource_kind_from_short_name_unknown_returns_none() {
    assert_eq!(ResourceKind::from_short_name("bogus"), None);
}

#[test]
fn resource_kind_display_name_matches_variant() {
    assert_eq!(ResourceKind::Pods.display_name(), "Pods");
    assert_eq!(ResourceKind::PersistentVolumeClaims.display_name(), "PersistentVolumeClaims");
    assert_eq!(ResourceKind::Custom("CRD".into()).display_name(), "CRD");
}
