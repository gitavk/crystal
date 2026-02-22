use ratatui::prelude::Rect;

use super::resource::ViewType;
use super::types::{PaneId, SplitDirection};

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
        Self { root: PaneNode::Leaf { id: 1, view }, next_id: 2 }
    }

    pub fn with_initial_id(id: PaneId, view: ViewType) -> Self {
        Self { root: PaneNode::Leaf { id, view }, next_id: id + 1 }
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
        if self.root.split_at(target, direction, new_id, new_view, 0.5) {
            self.next_id += 1;
            Some(new_id)
        } else {
            None
        }
    }

    /// Split using an externally-allocated pane ID (for TabManager global ID control).
    pub fn split_with_id(
        &mut self,
        target: PaneId,
        direction: SplitDirection,
        new_view: ViewType,
        new_id: PaneId,
    ) -> bool {
        self.root.split_at(target, direction, new_id, new_view, 0.5)
    }

    /// Split with an externally-allocated pane ID and a custom split ratio.
    pub fn split_with_id_and_ratio(
        &mut self,
        target: PaneId,
        direction: SplitDirection,
        new_view: ViewType,
        new_id: PaneId,
        ratio: f32,
    ) -> bool {
        self.root.split_at(target, direction, new_id, new_view, ratio)
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
    /// Amount is clamped so ratio stays within 0.1..0.9.
    pub fn resize(&mut self, target: PaneId, amount: f32, grow: bool) {
        self.root.resize_at(target, amount, grow);
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

    fn split_at(
        &mut self,
        target: PaneId,
        direction: SplitDirection,
        new_id: PaneId,
        new_view: ViewType,
        ratio: f32,
    ) -> bool {
        match self {
            PaneNode::Leaf { id, .. } if *id == target => {
                let old = std::mem::replace(self, PaneNode::Leaf { id: 0, view: ViewType::Empty });
                *self = PaneNode::Split {
                    direction,
                    ratio,
                    first: Box::new(old),
                    second: Box::new(PaneNode::Leaf { id: new_id, view: new_view }),
                };
                true
            }
            PaneNode::Split { first, second, .. } => {
                if first.contains_leaf(target) {
                    first.split_at(target, direction, new_id, new_view, ratio)
                } else if second.contains_leaf(target) {
                    second.split_at(target, direction, new_id, new_view, ratio)
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

    fn resize_at(&mut self, target: PaneId, amount: f32, grow: bool) -> bool {
        match self {
            PaneNode::Split { first, second, ratio, .. } => {
                let is_direct_first = matches!(first.as_ref(), PaneNode::Leaf { id, .. } if *id == target);
                let is_direct_second = matches!(second.as_ref(), PaneNode::Leaf { id, .. } if *id == target);

                if is_direct_first || is_direct_second {
                    let applied = match (is_direct_first, grow) {
                        (true, true) => amount,
                        (true, false) => -amount,
                        (false, true) => -amount,
                        (false, false) => amount,
                    };
                    *ratio = (*ratio + applied).clamp(0.1, 0.9);
                    return true;
                }

                if first.resize_at(target, amount, grow) {
                    return true;
                }
                second.resize_at(target, amount, grow)
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

pub(super) fn split_rect(area: Rect, direction: SplitDirection, ratio: f32) -> (Rect, Rect) {
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
