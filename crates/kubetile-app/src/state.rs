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

    #[allow(dead_code)]
    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            Some(i) => (i + 1) % self.items.len(),
            None => 0,
        });
    }

    #[allow(dead_code)]
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
mod tests;
