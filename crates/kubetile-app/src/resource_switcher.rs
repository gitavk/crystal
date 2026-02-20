use kubetile_tui::pane::ResourceKind;

pub struct ResourceSwitcher {
    input: String,
    all_kinds: Vec<ResourceKind>,
    filtered_kinds: Vec<ResourceKind>,
    selected: usize,
}

impl ResourceSwitcher {
    pub fn new() -> Self {
        let all_kinds: Vec<ResourceKind> = ResourceKind::all().to_vec();
        let filtered_kinds = all_kinds.clone();
        Self { input: String::new(), all_kinds, filtered_kinds, selected: 0 }
    }

    pub fn on_input(&mut self, ch: char) {
        self.input.push(ch);
        self.filter();
    }

    pub fn on_backspace(&mut self) {
        self.input.pop();
        self.filter();
    }

    pub fn select_next(&mut self) {
        if !self.filtered_kinds.is_empty() {
            self.selected = (self.selected + 1) % self.filtered_kinds.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.filtered_kinds.is_empty() {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.filtered_kinds.len() - 1);
        }
    }

    pub fn confirm(&self) -> Option<ResourceKind> {
        self.filtered_kinds.get(self.selected).cloned()
    }

    fn filter(&mut self) {
        let query = self.input.to_lowercase();
        if query.is_empty() {
            self.filtered_kinds = self.all_kinds.clone();
        } else {
            self.filtered_kinds = self
                .all_kinds
                .iter()
                .filter(|k| {
                    k.short_name().to_lowercase().contains(&query) || k.display_name().to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }
        if self.selected >= self.filtered_kinds.len() {
            self.selected = self.filtered_kinds.len().saturating_sub(1);
        }
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn filtered(&self) -> &[ResourceKind] {
        &self.filtered_kinds
    }

    pub fn selected(&self) -> usize {
        self.selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_shows_all_kinds() {
        let sw = ResourceSwitcher::new();
        assert_eq!(sw.filtered().len(), ResourceKind::all().len());
    }

    #[test]
    fn filter_po_matches_pods() {
        let mut sw = ResourceSwitcher::new();
        sw.on_input('p');
        sw.on_input('o');
        assert_eq!(sw.filtered().len(), 1);
        assert_eq!(sw.filtered()[0], ResourceKind::Pods);
    }

    #[test]
    fn filter_dep_matches_deployments() {
        let mut sw = ResourceSwitcher::new();
        for c in "dep".chars() {
            sw.on_input(c);
        }
        assert_eq!(sw.filtered().len(), 1);
        assert_eq!(sw.filtered()[0], ResourceKind::Deployments);
    }

    #[test]
    fn filter_s_matches_multiple() {
        let mut sw = ResourceSwitcher::new();
        sw.on_input('s');
        assert!(sw.filtered().len() > 1);
        let names: Vec<&str> = sw.filtered().iter().map(|k| k.display_name()).collect();
        assert!(names.contains(&"Services"));
        assert!(names.contains(&"StatefulSets"));
        assert!(names.contains(&"Secrets"));
    }

    #[test]
    fn filter_xyz_matches_none() {
        let mut sw = ResourceSwitcher::new();
        for c in "xyz".chars() {
            sw.on_input(c);
        }
        assert!(sw.filtered().is_empty());
    }

    #[test]
    fn select_next_wraps() {
        let mut sw = ResourceSwitcher::new();
        let len = sw.filtered().len();
        for _ in 0..len {
            sw.select_next();
        }
        assert_eq!(sw.selected(), 0);
    }

    #[test]
    fn select_prev_wraps() {
        let mut sw = ResourceSwitcher::new();
        sw.select_prev();
        assert_eq!(sw.selected(), sw.filtered().len() - 1);
    }

    #[test]
    fn confirm_returns_none_when_empty() {
        let mut sw = ResourceSwitcher::new();
        for c in "xyz".chars() {
            sw.on_input(c);
        }
        assert!(sw.confirm().is_none());
    }

    #[test]
    fn backspace_restores_filter() {
        let mut sw = ResourceSwitcher::new();
        for c in "xyz".chars() {
            sw.on_input(c);
        }
        assert!(sw.filtered().is_empty());
        sw.on_backspace();
        sw.on_backspace();
        sw.on_backspace();
        assert_eq!(sw.filtered().len(), ResourceKind::all().len());
    }
}
