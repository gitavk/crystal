use crate::pane::{PaneId, PaneTree, SplitDirection, ViewType};

pub struct Tab {
    pub id: u32,
    pub name: String,
    pub pane_tree: PaneTree,
    pub focused_pane: PaneId,
    pub fullscreen_pane: Option<PaneId>,
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
            fullscreen_pane: None,
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
            fullscreen_pane: None,
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

    pub fn tab_names(&self) -> Vec<String> {
        self.tabs.iter().map(|t| t.name.clone()).collect()
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
mod tests;
