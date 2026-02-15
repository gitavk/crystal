use std::any::Any;

use ratatui::prelude::{Frame, Rect};

pub type PaneId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaneCommand {
    ScrollUp,
    ScrollDown,
    SelectNext,
    SelectPrev,
    Select,
    Back,
    GoToTop,
    GoToBottom,
    PageUp,
    PageDown,
    ToggleFollow,
    SendInput(String),
    SearchInput(char),
    SearchConfirm,
    SearchClear,

    Filter(String),
    ClearFilter,
    SortByColumn(usize),
    ToggleSortOrder,
}

/// Every pane must satisfy this contract:
/// - Render itself within a given Rect
/// - React to focus state (styling only — no behavior change)
/// - Accept PaneCommands and update internal state
/// - Never affect other panes or access global state directly
pub trait Pane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool);
    fn handle_command(&mut self, cmd: &PaneCommand);
    fn view_type(&self) -> &ViewType;
    fn on_focus_change(&mut self, _previous: Option<&ViewType>) {}
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal, // top/bottom
    Vertical,   // left/right
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Pods,
    Deployments,
    Services,
    StatefulSets,
    DaemonSets,
    Jobs,
    CronJobs,
    ConfigMaps,
    Secrets,
    Ingresses,
    Nodes,
    Namespaces,
    PersistentVolumes,
    PersistentVolumeClaims,
    Custom(String),
}

impl ResourceKind {
    pub fn short_name(&self) -> &str {
        match self {
            Self::Pods => "po",
            Self::Deployments => "deploy",
            Self::Services => "svc",
            Self::StatefulSets => "sts",
            Self::DaemonSets => "ds",
            Self::Jobs => "job",
            Self::CronJobs => "cj",
            Self::ConfigMaps => "cm",
            Self::Secrets => "secret",
            Self::Ingresses => "ing",
            Self::Nodes => "no",
            Self::Namespaces => "ns",
            Self::PersistentVolumes => "pv",
            Self::PersistentVolumeClaims => "pvc",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Pods => "Pods",
            Self::Deployments => "Deployments",
            Self::Services => "Services",
            Self::StatefulSets => "StatefulSets",
            Self::DaemonSets => "DaemonSets",
            Self::Jobs => "Jobs",
            Self::CronJobs => "CronJobs",
            Self::ConfigMaps => "ConfigMaps",
            Self::Secrets => "Secrets",
            Self::Ingresses => "Ingresses",
            Self::Nodes => "Nodes",
            Self::Namespaces => "Namespaces",
            Self::PersistentVolumes => "PersistentVolumes",
            Self::PersistentVolumeClaims => "PersistentVolumeClaims",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn all() -> &'static [ResourceKind] {
        &[
            Self::Pods,
            Self::Deployments,
            Self::Services,
            Self::StatefulSets,
            Self::DaemonSets,
            Self::Jobs,
            Self::CronJobs,
            Self::ConfigMaps,
            Self::Secrets,
            Self::Ingresses,
            Self::Nodes,
            Self::Namespaces,
            Self::PersistentVolumes,
            Self::PersistentVolumeClaims,
        ]
    }

    pub fn from_short_name(s: &str) -> Option<Self> {
        Self::all().iter().find(|k| k.short_name() == s).cloned()
    }

    pub fn is_namespaced(&self) -> bool {
        !matches!(self, Self::Nodes | Self::Namespaces | Self::PersistentVolumes)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewType {
    ResourceList(ResourceKind),
    Detail(ResourceKind, String), // kind + resource name
    Terminal,
    Logs(String),               // pod name
    Exec(String),               // pod name
    Yaml(ResourceKind, String), // kind + resource name
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
        if self.root.split_at(target, direction, new_id, new_view) {
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
        self.root.split_at(target, direction, new_id, new_view)
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

/// Given current pane Rect and all pane Rects, find the best pane
/// in the given direction (up/down/left/right).
///
/// Algorithm:
/// 1. Filter panes in the correct relative direction
/// 2. Score by overlap on the perpendicular axis
/// 3. Among candidates with overlap > 0, pick the closest
/// 4. If no overlap candidates, pick the nearest by center distance
pub fn find_pane_in_direction(current: (PaneId, Rect), all: &[(PaneId, Rect)], direction: Direction) -> Option<PaneId> {
    let (cur_id, cur) = current;

    let candidates: Vec<_> = all
        .iter()
        .filter(|(id, _)| *id != cur_id)
        .filter(|(_, r)| match direction {
            Direction::Right => r.x >= cur.x + cur.width,
            Direction::Left => r.x + r.width <= cur.x,
            Direction::Down => r.y >= cur.y + cur.height,
            Direction::Up => r.y + r.height <= cur.y,
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    let with_overlap: Vec<_> = candidates
        .iter()
        .filter_map(|(id, r)| {
            let overlap = perpendicular_overlap(cur, *r, direction);
            if overlap > 0 {
                let dist = edge_distance(cur, *r, direction);
                Some((*id, dist, overlap))
            } else {
                None
            }
        })
        .collect();

    if !with_overlap.is_empty() {
        return with_overlap.iter().min_by_key(|(_, dist, overlap)| (*dist, -(*overlap as i32))).map(|(id, _, _)| *id);
    }

    let cx = cur.x as i32 + cur.width as i32 / 2;
    let cy = cur.y as i32 + cur.height as i32 / 2;
    candidates
        .iter()
        .min_by_key(|(_, r)| {
            let rx = r.x as i32 + r.width as i32 / 2;
            let ry = r.y as i32 + r.height as i32 / 2;
            (cx - rx).pow(2) + (cy - ry).pow(2)
        })
        .map(|(id, _)| *id)
}

fn perpendicular_overlap(a: Rect, b: Rect, direction: Direction) -> u16 {
    match direction {
        Direction::Left | Direction::Right => {
            let a_start = a.y;
            let a_end = a.y + a.height;
            let b_start = b.y;
            let b_end = b.y + b.height;
            a_end.min(b_end).saturating_sub(a_start.max(b_start))
        }
        Direction::Up | Direction::Down => {
            let a_start = a.x;
            let a_end = a.x + a.width;
            let b_start = b.x;
            let b_end = b.x + b.width;
            a_end.min(b_end).saturating_sub(a_start.max(b_start))
        }
    }
}

fn edge_distance(from: Rect, to: Rect, direction: Direction) -> u16 {
    match direction {
        Direction::Right => to.x.saturating_sub(from.x + from.width),
        Direction::Left => from.x.saturating_sub(to.x + to.width),
        Direction::Down => to.y.saturating_sub(from.y + from.height),
        Direction::Up => from.y.saturating_sub(to.y + to.height),
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
mod tests;
