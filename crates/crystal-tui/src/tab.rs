use crate::pane::{PaneId, PaneTree, SplitDirection, ViewType};

pub struct Tab {
    pub id: u32,
    pub name: String,
    pub pane_tree: PaneTree,
    pub focused_pane: PaneId,
}

pub struct TabManager {
    tabs: Vec<Tab>,
    active_tab: usize,
    next_pane_id: PaneId,
    next_tab_id: u32,
}

impl TabManager {
    pub fn new(initial_view: ViewType) -> Self {
        let pane_id = 1;
        let tab = Tab {
            id: 1,
            name: "Main".to_string(),
            pane_tree: PaneTree::with_initial_id(pane_id, initial_view),
            focused_pane: pane_id,
        };
        Self { tabs: vec![tab], active_tab: 0, next_pane_id: 2, next_tab_id: 2 }
    }

    pub fn new_tab(&mut self, name: &str, initial_view: ViewType) -> u32 {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;
        let pane_id = self.alloc_pane_id();
        let tab = Tab {
            id: tab_id,
            name: name.to_string(),
            pane_tree: PaneTree::with_initial_id(pane_id, initial_view),
            focused_pane: pane_id,
        };
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        tab_id
    }

    pub fn close_tab(&mut self, id: u32) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(pos);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
            true
        } else {
            false
        }
    }

    pub fn active(&self) -> &Tab {
        &self.tabs[self.active_tab]
    }

    pub fn active_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active_tab]
    }

    pub fn switch_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab = index;
        }
    }

    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
    }

    pub fn prev_tab(&mut self) {
        if self.active_tab == 0 {
            self.active_tab = self.tabs.len() - 1;
        } else {
            self.active_tab -= 1;
        }
    }

    pub fn rename_tab(&mut self, id: u32, name: &str) {
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == id) {
            tab.name = name.to_string();
        }
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active_index(&self) -> usize {
        self.active_tab
    }

    pub fn alloc_pane_id(&mut self) -> PaneId {
        let id = self.next_pane_id;
        self.next_pane_id += 1;
        id
    }

    pub fn split_pane(&mut self, target: PaneId, direction: SplitDirection, new_view: ViewType) -> Option<PaneId> {
        let new_id = self.alloc_pane_id();
        let tab = &mut self.tabs[self.active_tab];
        if tab.pane_tree.split_with_id(target, direction, new_view, new_id) {
            Some(new_id)
        } else {
            self.next_pane_id -= 1;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane::{ResourceKind, ViewType};

    fn pods_view() -> ViewType {
        ViewType::ResourceList(ResourceKind::Pods)
    }

    fn empty_view() -> ViewType {
        ViewType::Empty
    }

    #[test]
    fn starts_with_one_main_tab() {
        let tm = TabManager::new(pods_view());
        assert_eq!(tm.tabs().len(), 1);
        assert_eq!(tm.active().name, "Main");
        assert_eq!(tm.active().id, 1);
        assert_eq!(tm.active().focused_pane, 1);
        assert_eq!(tm.active_index(), 0);
    }

    #[test]
    fn new_tab_creates_with_correct_view() {
        let mut tm = TabManager::new(pods_view());
        let tab_id = tm.new_tab("Logs", empty_view());
        assert_eq!(tab_id, 2);
        assert_eq!(tm.tabs().len(), 2);
        assert_eq!(tm.active().name, "Logs");
        assert_eq!(tm.active_index(), 1);

        let leaf_ids = tm.active().pane_tree.leaf_ids();
        assert_eq!(leaf_ids.len(), 1);
        assert!(tm.active().pane_tree.find(tm.active().focused_pane).is_some());
    }

    #[test]
    fn close_tab_on_last_tab_is_noop() {
        let mut tm = TabManager::new(pods_view());
        assert!(!tm.close_tab(1));
        assert_eq!(tm.tabs().len(), 1);
    }

    #[test]
    fn close_tab_removes_and_adjusts_active() {
        let mut tm = TabManager::new(pods_view());
        tm.new_tab("Second", empty_view());
        tm.new_tab("Third", empty_view());
        // active is now "Third" at index 2
        assert_eq!(tm.active_index(), 2);

        assert!(tm.close_tab(tm.active().id));
        // Should fall back to index 1
        assert_eq!(tm.tabs().len(), 2);
        assert_eq!(tm.active().name, "Second");
    }

    #[test]
    fn close_middle_tab_preserves_active_when_possible() {
        let mut tm = TabManager::new(pods_view());
        let second_id = tm.new_tab("Second", empty_view());
        tm.new_tab("Third", empty_view());
        // active is "Third" at index 2
        assert!(tm.close_tab(second_id));
        // "Third" was at index 2, now at index 1 — active_tab was 2, clamped to 1
        assert_eq!(tm.active().name, "Third");
    }

    #[test]
    fn switch_tab_changes_active() {
        let mut tm = TabManager::new(pods_view());
        tm.new_tab("Second", empty_view());
        assert_eq!(tm.active_index(), 1);

        tm.switch_tab(0);
        assert_eq!(tm.active_index(), 0);
        assert_eq!(tm.active().name, "Main");
    }

    #[test]
    fn switch_tab_preserves_previous_state() {
        let mut tm = TabManager::new(pods_view());
        tm.new_tab("Second", empty_view());

        // Modify focus in second tab via a split
        let new_pane = tm.split_pane(tm.active().focused_pane, SplitDirection::Vertical, pods_view());
        assert!(new_pane.is_some());
        let second_pane_count = tm.active().pane_tree.leaf_ids().len();
        assert_eq!(second_pane_count, 2);

        // Switch to first tab
        tm.switch_tab(0);
        assert_eq!(tm.active().name, "Main");
        assert_eq!(tm.active().pane_tree.leaf_ids().len(), 1);

        // Switch back — second tab still has 2 panes
        tm.switch_tab(1);
        assert_eq!(tm.active().pane_tree.leaf_ids().len(), 2);
    }

    #[test]
    fn switch_tab_out_of_bounds_is_noop() {
        let mut tm = TabManager::new(pods_view());
        tm.switch_tab(99);
        assert_eq!(tm.active_index(), 0);
    }

    #[test]
    fn next_tab_wraps_around() {
        let mut tm = TabManager::new(pods_view());
        tm.new_tab("Second", empty_view());
        tm.new_tab("Third", empty_view());

        // Currently at index 2 (Third)
        tm.next_tab();
        assert_eq!(tm.active_index(), 0);
        assert_eq!(tm.active().name, "Main");
    }

    #[test]
    fn prev_tab_wraps_around() {
        let mut tm = TabManager::new(pods_view());
        tm.new_tab("Second", empty_view());
        tm.switch_tab(0);

        tm.prev_tab();
        assert_eq!(tm.active_index(), 1);
        assert_eq!(tm.active().name, "Second");
    }

    #[test]
    fn rename_tab() {
        let mut tm = TabManager::new(pods_view());
        tm.rename_tab(1, "Renamed");
        assert_eq!(tm.active().name, "Renamed");
    }

    #[test]
    fn pane_ids_unique_across_tabs() {
        let mut tm = TabManager::new(pods_view());

        // Split in first tab
        let p1 = tm.split_pane(1, SplitDirection::Vertical, empty_view()).unwrap();

        // Create second tab and split there
        tm.new_tab("Second", pods_view());
        let second_root = tm.active().focused_pane;
        let p2 = tm.split_pane(second_root, SplitDirection::Horizontal, empty_view()).unwrap();

        // Collect all pane IDs across both tabs
        tm.switch_tab(0);
        let mut all_ids: Vec<PaneId> = tm.active().pane_tree.leaf_ids();
        tm.switch_tab(1);
        all_ids.extend(tm.active().pane_tree.leaf_ids());

        // All IDs must be unique
        let unique_count = {
            let mut sorted = all_ids.clone();
            sorted.sort();
            sorted.dedup();
            sorted.len()
        };
        assert_eq!(all_ids.len(), unique_count);

        // Verify specific IDs: 1, p1 in tab0; second_root, p2 in tab1
        assert!(all_ids.contains(&1));
        assert!(all_ids.contains(&p1));
        assert!(all_ids.contains(&second_root));
        assert!(all_ids.contains(&p2));
        assert_eq!(all_ids.len(), 4);
    }

    #[test]
    fn close_nonexistent_tab_returns_false() {
        let mut tm = TabManager::new(pods_view());
        tm.new_tab("Second", empty_view());
        assert!(!tm.close_tab(99));
        assert_eq!(tm.tabs().len(), 2);
    }
}
