pub struct ResourceListState {
    pub items: Vec<Vec<String>>,
    pub headers: Vec<String>,
    pub selected: Option<usize>,
    pub scroll_offset: usize,
    pub loading: bool,
    pub error: Option<String>,
}

impl ResourceListState {
    pub fn new(headers: Vec<String>) -> Self {
        Self { items: Vec::new(), headers, selected: None, scroll_offset: 0, loading: true, error: None }
    }

    pub fn set_items(&mut self, items: Vec<Vec<String>>) {
        self.loading = false;
        self.error = None;
        self.items = items;
        if self.items.is_empty() {
            self.selected = None;
        } else if let Some(sel) = self.selected {
            if sel >= self.items.len() {
                self.selected = Some(self.items.len() - 1);
            }
        } else {
            self.selected = Some(0);
        }
    }

    pub fn set_error(&mut self, err: String) {
        self.loading = false;
        self.error = Some(err);
    }

    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            Some(i) => (i + 1) % self.items.len(),
            None => 0,
        });
    }

    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            Some(0) | None => self.items.len().saturating_sub(1),
            Some(i) => i - 1,
        });
    }

    #[allow(dead_code)]
    pub fn selected_item(&self) -> Option<&Vec<String>> {
        self.selected.and_then(|i| self.items.get(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> ResourceListState {
        let mut state = ResourceListState::new(vec!["A".into(), "B".into()]);
        state.set_items(vec![vec!["r0".into()], vec!["r1".into()], vec!["r2".into()]]);
        state
    }

    #[test]
    fn next_wraps_from_last_to_first() {
        let mut state = sample_state();
        state.selected = Some(2);
        state.next();
        assert_eq!(state.selected, Some(0));
    }

    #[test]
    fn previous_wraps_from_first_to_last() {
        let mut state = sample_state();
        state.selected = Some(0);
        state.previous();
        assert_eq!(state.selected, Some(2));
    }

    #[test]
    fn next_on_empty_is_noop() {
        let mut state = ResourceListState::new(vec![]);
        state.next();
        assert_eq!(state.selected, None);
    }

    #[test]
    fn set_items_initializes_selection_to_zero() {
        let mut state = ResourceListState::new(vec!["A".into()]);
        assert_eq!(state.selected, None);
        state.set_items(vec![vec!["row".into()]]);
        assert_eq!(state.selected, Some(0));
    }

    #[test]
    fn set_items_clamps_selection_when_items_shrink() {
        let mut state = sample_state();
        state.selected = Some(2);
        state.set_items(vec![vec!["only".into()]]);
        assert_eq!(state.selected, Some(0));
    }

    #[test]
    fn set_items_empty_clears_selection() {
        let mut state = sample_state();
        state.set_items(vec![]);
        assert_eq!(state.selected, None);
    }

    #[test]
    fn set_error_clears_loading() {
        let mut state = ResourceListState::new(vec![]);
        assert!(state.loading);
        state.set_error("timeout".into());
        assert!(!state.loading);
        assert_eq!(state.error.as_deref(), Some("timeout"));
    }

    #[test]
    fn selected_item_returns_correct_row() {
        let state = sample_state();
        assert_eq!(state.selected_item().unwrap(), &vec!["r0".to_string()]);
    }
}
