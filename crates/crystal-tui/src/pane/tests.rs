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
    tree.resize(1, 10.0);
    let rects = tree.layout(area(100, 50));
    assert_eq!(rects[0].1.width, 90);
    assert_eq!(rects[1].1.width, 10);

    // Resize far negative - should clamp to 0.1
    tree.resize(1, -10.0);
    let rects = tree.layout(area(100, 50));
    assert_eq!(rects[0].1.width, 10);
    assert_eq!(rects[1].1.width, 90);
}

#[test]
fn resize_adjusts_ratio() {
    let mut tree = PaneTree::new(pods_view());
    tree.split(1, SplitDirection::Vertical, logs_view());

    tree.resize(1, 0.1);
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
