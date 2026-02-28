use kubetile_core::QueryResult;

use super::{QueryPane, QueryPaneStatus};

impl QueryPane {
    pub fn set_result(&mut self, result: QueryResult) {
        self.col_widths = compute_col_widths(&result);
        self.result_selected_row = 0;
        self.result_scroll = 0;
        self.result_h_col_offset = 0;
        let label = self.connected_version.clone().unwrap_or_else(|| "Ready".to_string());
        self.status = QueryPaneStatus::Connected(label);
        self.result = Some(result);
    }

    pub fn size_hint(&self) -> (usize, usize) {
        let row_count = self.result.as_ref().map(|r| r.rows.len()).unwrap_or(0);
        let est_bytes = self.col_widths.iter().sum::<usize>() * row_count.max(1);
        (row_count, est_bytes)
    }

    pub(super) fn export_hint_text(&self) -> Option<&'static str> {
        if !matches!(self.status, QueryPaneStatus::Connected(_)) || self.result.is_none() {
            return None;
        }
        let (rows, bytes) = self.size_hint();
        if rows >= 100 || bytes >= 64_000 {
            Some("Y copies all · E exports to file")
        } else {
            None
        }
    }

    pub fn row_count(&self) -> usize {
        self.result.as_ref().map(|r| r.rows.len()).unwrap_or(0)
    }

    pub fn selected_row_csv(&self) -> Option<String> {
        let result = self.result.as_ref()?;
        let row = result.rows.get(self.result_selected_row)?;
        Some(row.iter().map(|c| csv_escape(c)).collect::<Vec<_>>().join(","))
    }

    pub fn all_rows_csv(&self) -> String {
        let Some(result) = &self.result else {
            return String::new();
        };
        let mut out = String::new();
        let header = result.headers.iter().map(|h| csv_escape(h)).collect::<Vec<_>>().join(",");
        out.push_str(&header);
        out.push('\n');
        for row in &result.rows {
            let line = row.iter().map(|c| csv_escape(c)).collect::<Vec<_>>().join(",");
            out.push_str(&line);
            out.push('\n');
        }
        out
    }

    pub fn scroll_up(&mut self) {
        let count = self.result_row_count.get();
        if count == 0 {
            return;
        }
        if self.result_selected_row + 1 < count {
            self.result_selected_row += 1;
            self.adjust_result_scroll();
        }
    }

    pub fn scroll_down(&mut self) {
        if self.result_selected_row > 0 {
            self.result_selected_row -= 1;
            self.adjust_result_scroll();
        }
    }

    pub fn result_page_down(&mut self) {
        let count = self.result_row_count.get();
        if count == 0 {
            return;
        }
        let page = self.result_visible_rows.get().max(1);
        self.result_selected_row = (self.result_selected_row + page).min(count - 1);
        self.adjust_result_scroll();
    }

    pub fn result_page_up(&mut self) {
        let page = self.result_visible_rows.get().max(1);
        self.result_selected_row = self.result_selected_row.saturating_sub(page);
        self.adjust_result_scroll();
    }

    pub fn scroll_h_left(&mut self) {
        if self.result_h_col_offset > 0 {
            self.result_h_col_offset -= 1;
        }
    }

    pub fn scroll_h_right(&mut self) {
        let total = self.col_widths.len();
        let last_visible = self.result_last_visible_col.get();
        if total > 0 && last_visible + 1 < total {
            self.result_h_col_offset += 1;
        }
    }

    fn adjust_result_scroll(&mut self) {
        let visible = self.result_visible_rows.get().max(1);
        let sel = self.result_selected_row;
        if sel < self.result_scroll {
            self.result_scroll = sel;
        } else if sel >= self.result_scroll + visible {
            self.result_scroll = sel + 1 - visible;
        }
    }
}

pub(super) fn compute_col_widths(result: &QueryResult) -> Vec<usize> {
    result
        .headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let max_data = result.rows.iter().map(|row| row.get(i).map(|c| c.len()).unwrap_or(0)).max().unwrap_or(0);
            h.len().max(max_data)
        })
        .collect()
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
