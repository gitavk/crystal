use ratatui::prelude::Rect;

pub type PaneId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal, // top/bottom
    Vertical,   // left/right
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceKind {
    Pods,
    Deployments,
    Services,
    ConfigMaps,
    Secrets,
    Nodes,
    Namespaces,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    ResourceList(ResourceKind),
    Detail(ResourceKind, String), // kind + resource name
    Terminal,
    Logs(String),   // pod name
    Exec(String),   // pod name
    Help,
    Empty,
    Plugin(String), // plugin name
}

#[derive(Debug)]
pub enum PaneNode {
    Leaf {
        id: PaneId,
        view: ViewType,
    },
    Split {
        direction: SplitDirection,
        ratio: f32, // 0.0..1.0, position of the divider
        first: Box<PaneNode>,
        second: Box<PaneNode>,
    },
}

/// Manages a pane layout tree with automatic ID generation.
pub struct PaneTree {
    root: PaneNode,
    next_id: PaneId,
}

impl PaneTree {
    pub fn new(view: ViewType) -> Self {
        Self {
            root: PaneNode::Leaf { id: 1, view },
            next_id: 2,
        }
    }

    pub fn root(&self) -> &PaneNode {
        &self.root
    }

    /// Split the pane with given ID.
    /// The target leaf becomes a Split node; the original leaf becomes `first`,
    /// a new leaf with `new_view` becomes `second`.
    /// Returns the new pane's ID, or None if target was not found.
    pub fn split(&mut self, target: PaneId, direction: SplitDirection, new_view: ViewType) -> Option<PaneId> {
        let new_id = self.next_id;
        if self.root.split_at(target, direction, new_id, new_view) {
            self.next_id += 1;
            Some(new_id)
        } else {
            None
        }
    }

    /// Close a pane, promoting its sibling to take the parent's place.
    /// Returns false if the target is the last remaining pane.
    pub fn close(&mut self, target: PaneId) -> bool {
        if matches!(self.root, PaneNode::Leaf { .. }) {
            return false;
        }
        self.root.close_leaf(target)
    }

    /// Resize: adjust the ratio of the split containing the target pane.
    /// Delta is clamped so ratio stays within 0.1..0.9.
    pub fn resize(&mut self, target: PaneId, delta: f32) {
        self.root.resize_at(target, delta);
    }

    /// Get all leaf pane IDs in depth-first order (for focus cycling).
    pub fn leaf_ids(&self) -> Vec<PaneId> {
        self.root.leaf_ids()
    }

    /// Calculate the Rect for each pane given the total area.
    pub fn layout(&self, area: Rect) -> Vec<(PaneId, Rect)> {
        self.root.layout(area)
    }

    /// Find pane by ID.
    pub fn find(&self, id: PaneId) -> Option<&PaneNode> {
        self.root.find(id)
    }
}

impl PaneNode {
    fn contains_leaf(&self, target: PaneId) -> bool {
        match self {
            PaneNode::Leaf { id, .. } => *id == target,
            PaneNode::Split { first, second, .. } => first.contains_leaf(target) || second.contains_leaf(target),
        }
    }

    fn split_at(&mut self, target: PaneId, direction: SplitDirection, new_id: PaneId, new_view: ViewType) -> bool {
        match self {
            PaneNode::Leaf { id, .. } if *id == target => {
                let old = std::mem::replace(self, PaneNode::Leaf { id: 0, view: ViewType::Empty });
                *self = PaneNode::Split {
                    direction,
                    ratio: 0.5,
                    first: Box::new(old),
                    second: Box::new(PaneNode::Leaf { id: new_id, view: new_view }),
                };
                true
            }
            PaneNode::Split { first, second, .. } => {
                if first.contains_leaf(target) {
                    first.split_at(target, direction, new_id, new_view)
                } else if second.contains_leaf(target) {
                    second.split_at(target, direction, new_id, new_view)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn close_leaf(&mut self, target: PaneId) -> bool {
        let target_is_first = matches!(self,
            PaneNode::Split { first, .. }
            if matches!(first.as_ref(), PaneNode::Leaf { id, .. } if *id == target)
        );
        let target_is_second = matches!(self,
            PaneNode::Split { second, .. }
            if matches!(second.as_ref(), PaneNode::Leaf { id, .. } if *id == target)
        );

        if target_is_first || target_is_second {
            let old = std::mem::replace(self, PaneNode::Leaf { id: 0, view: ViewType::Empty });
            if let PaneNode::Split { first, second, .. } = old {
                *self = if target_is_first { *second } else { *first };
            }
            return true;
        }

        match self {
            PaneNode::Split { first, second, .. } => {
                if first.close_leaf(target) {
                    return true;
                }
                second.close_leaf(target)
            }
            _ => false,
        }
    }

    fn resize_at(&mut self, target: PaneId, delta: f32) -> bool {
        match self {
            PaneNode::Split { first, second, ratio, .. } => {
                let is_direct_child = matches!(first.as_ref(), PaneNode::Leaf { id, .. } if *id == target)
                    || matches!(second.as_ref(), PaneNode::Leaf { id, .. } if *id == target);

                if is_direct_child {
                    *ratio = (*ratio + delta).clamp(0.1, 0.9);
                    return true;
                }

                if first.resize_at(target, delta) {
                    return true;
                }
                second.resize_at(target, delta)
            }
            _ => false,
        }
    }

    pub fn leaf_ids(&self) -> Vec<PaneId> {
        let mut result = Vec::new();
        self.collect_leaf_ids(&mut result);
        result
    }

    fn collect_leaf_ids(&self, out: &mut Vec<PaneId>) {
        match self {
            PaneNode::Leaf { id, .. } => out.push(*id),
            PaneNode::Split { first, second, .. } => {
                first.collect_leaf_ids(out);
                second.collect_leaf_ids(out);
            }
        }
    }

    pub fn layout(&self, area: Rect) -> Vec<(PaneId, Rect)> {
        let mut result = Vec::new();
        self.layout_inner(area, &mut result);
        result
    }

    fn layout_inner(&self, area: Rect, out: &mut Vec<(PaneId, Rect)>) {
        match self {
            PaneNode::Leaf { id, .. } => {
                out.push((*id, area));
            }
            PaneNode::Split { direction, ratio, first, second } => {
                let (first_area, second_area) = split_rect(area, *direction, *ratio);
                first.layout_inner(first_area, out);
                second.layout_inner(second_area, out);
            }
        }
    }

    pub fn find(&self, target: PaneId) -> Option<&PaneNode> {
        match self {
            PaneNode::Leaf { id, .. } if *id == target => Some(self),
            PaneNode::Split { first, second, .. } => first.find(target).or_else(|| second.find(target)),
            _ => None,
        }
    }
}

fn split_rect(area: Rect, direction: SplitDirection, ratio: f32) -> (Rect, Rect) {
    match direction {
        SplitDirection::Horizontal => {
            let first_height = (area.height as f32 * ratio).round() as u16;
            let second_height = area.height.saturating_sub(first_height);
            (
                Rect { x: area.x, y: area.y, width: area.width, height: first_height },
                Rect { x: area.x, y: area.y + first_height, width: area.width, height: second_height },
            )
        }
        SplitDirection::Vertical => {
            let first_width = (area.width as f32 * ratio).round() as u16;
            let second_width = area.width.saturating_sub(first_width);
            (
                Rect { x: area.x, y: area.y, width: first_width, height: area.height },
                Rect { x: area.x + first_width, y: area.y, width: second_width, height: area.height },
            )
        }
    }
}

#[cfg(test)]
mod tests {
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

        // Resize far positive — should clamp to 0.9
        tree.resize(1, 10.0);
        let rects = tree.layout(area(100, 50));
        assert_eq!(rects[0].1.width, 90);
        assert_eq!(rects[1].1.width, 10);

        // Resize far negative — should clamp to 0.1
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
        // 0.5 + 0.1 = 0.6 → 60 width
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
}
