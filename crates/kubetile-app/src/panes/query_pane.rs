use std::any::Any;
use std::cell::Cell;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use kubetile_core::{QueryConfig, QueryResult, SavedQuery};
use kubetile_tui::pane::{Pane, PaneCommand, ViewType};
use kubetile_tui::theme::Theme;

struct QueryHistoryState {
    entries: Vec<String>,
    selected: usize,
}

struct SavedQueriesState {
    entries: Vec<SavedQuery>,
    selected: usize,
    filter_input: Option<String>,
    rename_input: Option<String>,
}

enum QueryPaneStatus {
    Connecting,
    Connected(String),
    Executing,
    Error(String),
}

pub struct QueryPane {
    view_type: ViewType,
    pod_name: String,
    namespace: String,
    status: QueryPaneStatus,
    pub config: QueryConfig,
    connected_version: Option<String>,
    editor_lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    editor_scroll: usize,
    result: Option<QueryResult>,
    col_widths: Vec<usize>,
    result_selected_row: usize,
    result_scroll: usize,
    result_h_col_offset: usize,
    result_row_count: Cell<usize>,
    result_visible_rows: Cell<usize>,
    result_last_visible_col: Cell<usize>,
    editor_area_height: Cell<usize>,
    last_executed_sql: Option<String>,
    history: Option<QueryHistoryState>,
    pending_save_name: Option<String>,
    saved_queries: Option<SavedQueriesState>,
    export_dialog_path: Option<String>,
}

impl QueryPane {
    pub fn new(config: &QueryConfig) -> Self {
        Self {
            view_type: ViewType::Query(config.pod.clone()),
            pod_name: config.pod.clone(),
            namespace: config.namespace.clone(),
            status: QueryPaneStatus::Connecting,
            config: config.clone(),
            connected_version: None,
            editor_lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            editor_scroll: 0,
            result: None,
            col_widths: Vec::new(),
            result_selected_row: 0,
            result_scroll: 0,
            result_h_col_offset: 0,
            result_row_count: Cell::new(0),
            result_visible_rows: Cell::new(0),
            result_last_visible_col: Cell::new(0),
            editor_area_height: Cell::new(5),
            last_executed_sql: None,
            history: None,
            pending_save_name: None,
            saved_queries: None,
            export_dialog_path: None,
        }
    }

    pub fn is_connecting(&self) -> bool {
        matches!(self.status, QueryPaneStatus::Connecting)
    }

    pub fn set_connected(&mut self, version: String) {
        self.connected_version = Some(version.clone());
        self.status = QueryPaneStatus::Connected(version);
    }

    pub fn set_executing(&mut self, sql: &str) {
        self.last_executed_sql = Some(sql.to_string());
        self.result = None;
        self.col_widths.clear();
        self.result_selected_row = 0;
        self.result_scroll = 0;
        self.result_h_col_offset = 0;
        self.status = QueryPaneStatus::Executing;
    }

    pub fn last_executed_sql(&self) -> Option<&str> {
        self.last_executed_sql.as_deref()
    }

    pub fn set_editor_content(&mut self, sql: &str) {
        self.editor_lines = sql.split('\n').map(|s| s.to_string()).collect();
        if self.editor_lines.is_empty() {
            self.editor_lines = vec![String::new()];
        }
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.editor_scroll = 0;
    }

    pub fn open_history(&mut self, entries: Vec<String>) {
        let selected = 0;
        self.history = Some(QueryHistoryState { entries, selected });
    }

    pub fn close_history(&mut self) {
        self.history = None;
    }

    pub fn history_next(&mut self) {
        if let Some(ref mut h) = self.history {
            if h.selected + 1 < h.entries.len() {
                h.selected += 1;
            }
        }
    }

    pub fn history_prev(&mut self) {
        if let Some(ref mut h) = self.history {
            h.selected = h.selected.saturating_sub(1);
        }
    }

    pub fn history_selected_sql(&self) -> Option<&str> {
        self.history.as_ref()?.entries.get(self.history.as_ref()?.selected).map(|s| s.as_str())
    }

    pub fn history_selected_index(&self) -> usize {
        self.history.as_ref().map(|h| h.selected).unwrap_or(0)
    }

    // --- Save-name dialog ---

    pub fn open_save_name(&mut self) {
        self.pending_save_name = Some(String::new());
    }

    pub fn close_save_name(&mut self) {
        self.pending_save_name = None;
    }

    pub fn save_name_input(&mut self, c: char) {
        if let Some(ref mut buf) = self.pending_save_name {
            buf.push(c);
        }
    }

    pub fn save_name_backspace(&mut self) {
        if let Some(ref mut buf) = self.pending_save_name {
            buf.pop();
        }
    }

    pub fn current_save_name(&self) -> Option<&str> {
        self.pending_save_name.as_deref()
    }

    // --- Saved-queries popup ---

    pub fn open_saved_queries(&mut self, entries: Vec<SavedQuery>) {
        self.saved_queries = Some(SavedQueriesState { entries, selected: 0, filter_input: None, rename_input: None });
    }

    pub fn close_saved_queries(&mut self) {
        self.saved_queries = None;
    }

    pub fn saved_queries_next(&mut self) {
        if let Some(ref mut sq) = self.saved_queries {
            let count = saved_queries_filtered(sq).len();
            if sq.selected + 1 < count {
                sq.selected += 1;
            }
        }
    }

    pub fn saved_queries_prev(&mut self) {
        if let Some(ref mut sq) = self.saved_queries {
            sq.selected = sq.selected.saturating_sub(1);
        }
    }

    pub fn saved_queries_start_filter(&mut self) {
        if let Some(ref mut sq) = self.saved_queries {
            sq.rename_input = None;
            sq.filter_input = Some(String::new());
            sq.selected = 0;
        }
    }

    pub fn saved_queries_start_rename(&mut self) {
        if let Some(ref mut sq) = self.saved_queries {
            let current_name =
                saved_queries_filtered(sq).get(sq.selected).map(|(_, e)| e.name.clone()).unwrap_or_default();
            sq.rename_input = Some(current_name);
        }
    }

    pub fn saved_queries_input(&mut self, c: char) {
        if let Some(ref mut sq) = self.saved_queries {
            if let Some(ref mut buf) = sq.rename_input {
                buf.push(c);
            } else if let Some(ref mut buf) = sq.filter_input {
                buf.push(c);
                sq.selected = 0;
            }
        }
    }

    pub fn saved_queries_backspace(&mut self) {
        if let Some(ref mut sq) = self.saved_queries {
            if let Some(ref mut buf) = sq.rename_input {
                buf.pop();
            } else if let Some(ref mut buf) = sq.filter_input {
                buf.pop();
                sq.selected = 0;
            }
        }
    }

    /// Returns false if neither sub-mode nor popup was open (caller should change mode to Normal).
    pub fn saved_queries_close_sub_mode(&mut self) -> bool {
        if let Some(ref mut sq) = self.saved_queries {
            if sq.rename_input.is_some() {
                sq.rename_input = None;
                return true;
            }
            if sq.filter_input.is_some() {
                sq.filter_input = None;
                sq.selected = 0;
                return true;
            }
            self.saved_queries = None;
            return false;
        }
        false
    }

    pub fn saved_queries_is_renaming(&self) -> bool {
        self.saved_queries.as_ref().is_some_and(|sq| sq.rename_input.is_some())
    }

    /// Returns `(real_index, name, sql)` for the currently selected entry.
    pub fn saved_queries_selected(&self) -> Option<(usize, &str, &str)> {
        let sq = self.saved_queries.as_ref()?;
        let filtered = saved_queries_filtered(sq);
        let (real_idx, entry) = filtered.get(sq.selected)?;
        Some((*real_idx, entry.name.as_str(), entry.sql.as_str()))
    }

    pub fn saved_queries_rename_input(&self) -> Option<&str> {
        self.saved_queries.as_ref()?.rename_input.as_deref()
    }

    // --- Export dialog ---

    pub fn open_export_dialog(&mut self, pre_filled: String) {
        self.export_dialog_path = Some(pre_filled);
    }

    pub fn close_export_dialog(&mut self) {
        self.export_dialog_path = None;
    }

    pub fn export_path_input(&mut self, c: char) {
        if let Some(ref mut buf) = self.export_dialog_path {
            buf.push(c);
        }
    }

    pub fn export_path_backspace(&mut self) {
        if let Some(ref mut buf) = self.export_dialog_path {
            buf.pop();
        }
    }

    pub fn current_export_path(&self) -> Option<&str> {
        self.export_dialog_path.as_deref()
    }

    pub fn size_hint(&self) -> (usize, usize) {
        let row_count = self.result.as_ref().map(|r| r.rows.len()).unwrap_or(0);
        let est_bytes = self.col_widths.iter().sum::<usize>() * row_count.max(1);
        (row_count, est_bytes)
    }

    fn export_hint_text(&self) -> Option<&'static str> {
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

    pub fn set_result(&mut self, result: QueryResult) {
        self.col_widths = compute_col_widths(&result);
        self.result_selected_row = 0;
        self.result_scroll = 0;
        self.result_h_col_offset = 0;
        let label = self.connected_version.clone().unwrap_or_else(|| "Ready".to_string());
        self.status = QueryPaneStatus::Connected(label);
        self.result = Some(result);
    }

    pub fn set_error(&mut self, error: String) {
        self.status = QueryPaneStatus::Error(error);
    }

    pub fn editor_push(&mut self, c: char) {
        let byte = char_to_byte(&self.editor_lines[self.cursor_row], self.cursor_col);
        self.editor_lines[self.cursor_row].insert(byte, c);
        self.cursor_col += 1;
    }

    pub fn editor_pop(&mut self) {
        if self.cursor_col > 0 {
            let byte = char_to_byte(&self.editor_lines[self.cursor_row], self.cursor_col - 1);
            self.editor_lines[self.cursor_row].remove(byte);
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            let current = self.editor_lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.editor_lines[self.cursor_row].chars().count();
            self.editor_lines[self.cursor_row].push_str(&current);
            self.adjust_editor_scroll();
        }
    }

    pub fn editor_newline(&mut self) {
        let byte = char_to_byte(&self.editor_lines[self.cursor_row], self.cursor_col);
        let tail = self.editor_lines[self.cursor_row].split_off(byte);
        self.cursor_row += 1;
        self.editor_lines.insert(self.cursor_row, tail);
        self.cursor_col = 0;
        self.adjust_editor_scroll();
    }

    pub fn cursor_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            let max_col = self.editor_lines[self.cursor_row].chars().count();
            self.cursor_col = self.cursor_col.min(max_col);
            self.adjust_editor_scroll();
        }
    }

    pub fn cursor_down(&mut self) {
        if self.cursor_row + 1 < self.editor_lines.len() {
            self.cursor_row += 1;
            let max_col = self.editor_lines[self.cursor_row].chars().count();
            self.cursor_col = self.cursor_col.min(max_col);
            self.adjust_editor_scroll();
        }
    }

    pub fn cursor_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.editor_lines[self.cursor_row].chars().count();
            self.adjust_editor_scroll();
        }
    }

    pub fn cursor_right(&mut self) {
        let line_len = self.editor_lines[self.cursor_row].chars().count();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.editor_lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
            self.adjust_editor_scroll();
        }
    }

    pub fn editor_indent(&mut self) {
        self.editor_lines[self.cursor_row].insert_str(0, "  ");
        self.cursor_col += 2;
    }

    pub fn editor_deindent(&mut self) {
        let spaces = self.editor_lines[self.cursor_row].chars().take(2).take_while(|&c| c == ' ').count();
        if spaces == 0 {
            return;
        }
        self.editor_lines[self.cursor_row].drain(..spaces);
        self.cursor_col = self.cursor_col.saturating_sub(spaces);
    }

    pub fn editor_home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn editor_end(&mut self) {
        self.cursor_col = self.editor_lines[self.cursor_row].chars().count();
    }

    pub fn editor_content(&self) -> String {
        self.editor_lines.join("\n")
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

    fn adjust_editor_scroll(&mut self) {
        let h = self.editor_area_height.get().max(1);
        if self.cursor_row < self.editor_scroll {
            self.editor_scroll = self.cursor_row;
        } else if self.cursor_row >= self.editor_scroll + h {
            self.editor_scroll = self.cursor_row + 1 - h;
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

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(s.len())
}

fn compute_col_widths(result: &QueryResult) -> Vec<usize> {
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

fn render_history_popup(frame: &mut Frame, area: Rect, h: &QueryHistoryState, theme: &Theme) {
    let popup_w = (area.width.saturating_sub(4)).min(area.width * 9 / 10).max(20);
    let popup_h = (area.height.saturating_sub(2)).min(area.height * 4 / 5).max(6);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_w)) / 2,
        y: area.y + (area.height.saturating_sub(popup_h)) / 2,
        width: popup_w,
        height: popup_h,
    };
    frame.render_widget(Clear, popup);

    let count = h.entries.len();
    let title = format!(" Query History ({count}) ");
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(theme.accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(theme.overlay);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 2 {
        return;
    }

    // Bottom hint line
    let hint_y = inner.y + inner.height.saturating_sub(1);
    let hint_area = Rect { x: inner.x, y: hint_y, width: inner.width, height: 1 };
    let list_area = Rect { height: inner.height.saturating_sub(1), ..inner };

    frame.render_widget(
        Paragraph::new("j/k navigate  Enter select  d delete  Esc cancel").style(theme.text_dim),
        hint_area,
    );

    // Split list area: left 40% list, right 60% preview
    let list_w = (list_area.width * 2 / 5).max(10);
    let preview_w = list_area.width.saturating_sub(list_w + 1);
    let left_area = Rect { width: list_w, ..list_area };
    let divider_area = Rect { x: list_area.x + list_w, y: list_area.y, width: 1, height: list_area.height };
    let right_area = Rect { x: list_area.x + list_w + 1, y: list_area.y, width: preview_w, height: list_area.height };

    // Divider — one │ per row, stops before the hint line
    let divider_lines: Vec<Line> = std::iter::repeat_n(Line::from("│"), list_area.height as usize).collect();
    frame.render_widget(Paragraph::new(divider_lines).style(theme.text_dim), divider_area);

    // List
    let visible = list_area.height as usize;
    let scroll = if h.selected >= visible { h.selected + 1 - visible } else { 0 };
    let list_lines: Vec<Line> = h
        .entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible)
        .map(|(i, sql)| {
            let first_line = sql.lines().next().unwrap_or("").chars().take(list_w as usize - 3).collect::<String>();
            let prefix = if i == h.selected { "> " } else { "  " };
            let text = format!("{prefix}{first_line}");
            let style = if i == h.selected { Style::default().fg(theme.accent).bold() } else { Style::default() };
            Line::from(Span::styled(text, style))
        })
        .collect();
    frame.render_widget(Paragraph::new(list_lines), left_area);

    // Preview
    if let Some(sql) = h.entries.get(h.selected) {
        let preview_lines: Vec<Line> = sql
            .lines()
            .flat_map(|line| {
                if line.is_empty() {
                    vec![Line::from("")]
                } else {
                    line.chars()
                        .collect::<Vec<_>>()
                        .chunks(preview_w as usize)
                        .map(|chunk| Line::from(chunk.iter().collect::<String>()))
                        .collect()
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(preview_lines).style(Style::default().fg(theme.fg)), right_area);
    }
}

fn saved_queries_filtered(sq: &SavedQueriesState) -> Vec<(usize, &SavedQuery)> {
    match &sq.filter_input {
        Some(f) if !f.is_empty() => {
            let f = f.to_lowercase();
            sq.entries.iter().enumerate().filter(|(_, e)| e.name.to_lowercase().contains(&f)).collect()
        }
        _ => sq.entries.iter().enumerate().collect(),
    }
}

fn render_save_name_popup(frame: &mut Frame, area: Rect, name_buf: &str, theme: &Theme) {
    let popup_w = (area.width.saturating_sub(4)).clamp(30, 60);
    let popup_h = 5u16;
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_w)) / 2,
        y: area.y + (area.height.saturating_sub(popup_h)) / 2,
        width: popup_w,
        height: popup_h,
    };
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Save Query ")
        .title_style(Style::default().fg(theme.accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(theme.overlay);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 3 {
        return;
    }

    let input_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 };
    let hint_area = Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(1), width: inner.width, height: 1 };

    let display_name: String = name_buf.chars().take(inner.width.saturating_sub(8) as usize).collect();
    let label = format!("Name: {display_name}▌");
    frame.render_widget(Paragraph::new(label).style(Style::default().fg(theme.accent)), input_area);
    frame.render_widget(Paragraph::new("Enter confirm  Esc cancel").style(theme.text_dim), hint_area);
}

fn render_saved_queries_popup(frame: &mut Frame, area: Rect, sq: &SavedQueriesState, theme: &Theme) {
    let popup_w = (area.width.saturating_sub(4)).min(area.width * 9 / 10).max(20);
    let popup_h = (area.height.saturating_sub(2)).min(area.height * 4 / 5).max(6);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_w)) / 2,
        y: area.y + (area.height.saturating_sub(popup_h)) / 2,
        width: popup_w,
        height: popup_h,
    };
    frame.render_widget(Clear, popup);

    let filtered = saved_queries_filtered(sq);
    let count = sq.entries.len();
    let title = format!(" Saved Queries ({count}) ");
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(theme.accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(theme.overlay);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 2 {
        return;
    }

    let hint_y = inner.y + inner.height.saturating_sub(1);
    let hint_area = Rect { x: inner.x, y: hint_y, width: inner.width, height: 1 };

    let (content_h, filter_h) = if sq.filter_input.is_some() {
        (inner.height.saturating_sub(2), 1u16)
    } else {
        (inner.height.saturating_sub(1), 0u16)
    };
    let content_area = Rect { height: content_h, ..inner };

    // Hint text
    let hint_text = if sq.rename_input.is_some() {
        "Enter confirm  Esc cancel"
    } else if sq.filter_input.is_some() {
        "j/k nav  Enter load  d del  e rename  Esc clear filter"
    } else {
        "j/k nav  Enter load  d del  e rename  / filter  Esc close"
    };
    frame.render_widget(Paragraph::new(hint_text).style(theme.text_dim), hint_area);

    // Filter input line (just above hint)
    if let Some(ref filter) = sq.filter_input {
        let filter_area = Rect { x: inner.x, y: hint_y.saturating_sub(1), width: inner.width, height: 1 };
        let display_filter: String = filter.chars().take(inner.width.saturating_sub(10) as usize).collect();
        frame.render_widget(
            Paragraph::new(format!("Filter: {display_filter}▌")).style(Style::default().fg(theme.accent)),
            filter_area,
        );
    }
    let _ = filter_h; // used implicitly via content_h

    if content_area.height == 0 {
        return;
    }

    // Split content area: left 40% list, right 60% preview
    let list_w = (content_area.width * 2 / 5).max(10);
    let preview_w = content_area.width.saturating_sub(list_w + 1);
    let left_area = Rect { width: list_w, ..content_area };
    let divider_area = Rect { x: content_area.x + list_w, y: content_area.y, width: 1, height: content_area.height };
    let right_area =
        Rect { x: content_area.x + list_w + 1, y: content_area.y, width: preview_w, height: content_area.height };

    let divider_lines: Vec<Line> = std::iter::repeat_n(Line::from("│"), content_area.height as usize).collect();
    frame.render_widget(Paragraph::new(divider_lines).style(theme.text_dim), divider_area);

    // List
    let visible = content_area.height as usize;
    let scroll = if sq.selected >= visible { sq.selected + 1 - visible } else { 0 };
    let list_lines: Vec<Line> = filtered
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible)
        .map(|(fi, (_, entry))| {
            let is_selected = fi == sq.selected;
            let max_name_w = (list_w as usize).saturating_sub(3);
            let display_name = if is_selected {
                if let Some(ref rename_buf) = sq.rename_input {
                    let n: String = rename_buf.chars().take(max_name_w.saturating_sub(1)).collect();
                    format!("{n}▌")
                } else {
                    entry.name.chars().take(max_name_w).collect()
                }
            } else {
                entry.name.chars().take(max_name_w).collect()
            };
            let prefix = if is_selected { "> " } else { "  " };
            let text = format!("{prefix}{display_name}");
            let style = if is_selected { Style::default().fg(theme.accent).bold() } else { Style::default() };
            Line::from(Span::styled(text, style))
        })
        .collect();
    frame.render_widget(Paragraph::new(list_lines), left_area);

    // Preview: show SQL of selected entry
    if let Some((_, entry)) = filtered.get(sq.selected) {
        let preview_lines: Vec<Line> = entry
            .sql
            .lines()
            .flat_map(|line| {
                if line.is_empty() {
                    vec![Line::from("")]
                } else {
                    line.chars()
                        .collect::<Vec<_>>()
                        .chunks(preview_w as usize)
                        .map(|chunk| Line::from(chunk.iter().collect::<String>()))
                        .collect()
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(preview_lines).style(Style::default().fg(theme.fg)), right_area);
    } else if filtered.is_empty() {
        frame.render_widget(Paragraph::new("No matches").style(theme.text_dim), left_area);
    }
}

fn render_export_dialog_popup(frame: &mut Frame, area: Rect, path_buf: &str, theme: &Theme) {
    let popup_w = (area.width.saturating_sub(4)).clamp(30, 70);
    let popup_h = 5u16;
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_w)) / 2,
        y: area.y + (area.height.saturating_sub(popup_h)) / 2,
        width: popup_w,
        height: popup_h,
    };
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Export to File ")
        .title_style(Style::default().fg(theme.accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(theme.overlay);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 3 {
        return;
    }

    let input_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 };
    let hint_area = Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(1), width: inner.width, height: 1 };

    // Show end of path when it's too long to fit
    let prefix = "Path: ";
    let max_path_w = inner.width.saturating_sub(prefix.len() as u16) as usize;
    let display = if path_buf.len() > max_path_w {
        format!("…{}", &path_buf[path_buf.len().saturating_sub(max_path_w.saturating_sub(1))..])
    } else {
        path_buf.to_string()
    };
    let label = format!("{prefix}{display}");
    frame.render_widget(Paragraph::new(label).style(Style::default().fg(theme.accent)), input_area);
    frame.render_widget(Paragraph::new("Enter confirm  Esc cancel").style(theme.text_dim), hint_area);
}

fn render_cursor_line(line: &str, cursor_col: usize, normal_style: Style, cursor_style: Style) -> Line<'static> {
    let char_count = line.chars().count();
    let byte = char_to_byte(line, cursor_col);
    let before = line[..byte].to_string();
    let (cursor_ch, after) = if cursor_col < char_count {
        let ch = line[byte..].chars().next().unwrap();
        (ch.to_string(), line[byte + ch.len_utf8()..].to_string())
    } else {
        (" ".to_string(), String::new())
    };
    Line::from(vec![
        Span::styled(before, normal_style),
        Span::styled(cursor_ch, cursor_style),
        Span::styled(after, normal_style),
    ])
}

impl Pane for QueryPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme) {
        let border_style = if focused { theme.border_active } else { theme.border };
        let title = format!(" [query:{}/{}] ", self.pod_name, self.namespace);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title)
            .title_style(Style::default().fg(theme.accent).bold());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 4 {
            return;
        }

        let editor_height = ((inner.height as usize) / 3).clamp(3, 10) as u16;
        self.editor_area_height.set(editor_height as usize);

        let editor_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: editor_height };
        let sep_y = inner.y + editor_height;
        let sep_area = Rect { x: inner.x, y: sep_y, width: inner.width, height: 1 };
        let results_top = sep_y + 1;
        let results_height = inner.height.saturating_sub(editor_height + 2);
        let results_area = Rect { x: inner.x, y: results_top, width: inner.width, height: results_height };
        let status_area =
            Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(1), width: inner.width, height: 1 };

        let normal_style = if focused { Style::default().fg(theme.accent) } else { Style::default().fg(theme.fg) };
        let cursor_style = normal_style.add_modifier(Modifier::REVERSED);

        // Editor
        let editor_scroll = self.editor_scroll;
        let end_line = (editor_scroll + editor_height as usize).min(self.editor_lines.len());
        let editor_content: Vec<Line> = (editor_scroll..end_line)
            .map(|row| {
                let line = &self.editor_lines[row];
                if focused && row == self.cursor_row {
                    render_cursor_line(line, self.cursor_col, normal_style, cursor_style)
                } else {
                    Line::from(Span::styled(line.clone(), normal_style))
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(editor_content), editor_area);

        let sep_text = "─".repeat(inner.width as usize);
        frame.render_widget(Paragraph::new(sep_text).style(theme.text_dim), sep_area);

        // Results — also produces col_range for the status line
        let mut col_range: Option<(usize, usize, usize)> = None; // (first, last, total) 1-indexed
        if results_height > 0 {
            match &self.result {
                None => {
                    frame.render_widget(Paragraph::new("No results yet").style(theme.text_dim), results_area);
                }
                Some(result) if result.headers.is_empty() => {
                    frame.render_widget(Paragraph::new("No results yet").style(theme.text_dim), results_area);
                }
                Some(result) => {
                    let row_count = result.rows.len();
                    self.result_row_count.set(row_count);

                    // Reserve 1 col on the right for the scrollbar
                    let text_width = results_area.width.saturating_sub(1) as usize;
                    let scrollbar_area = Rect {
                        x: results_area.x + results_area.width.saturating_sub(1),
                        y: results_area.y,
                        width: 1,
                        height: results_height,
                    };
                    let text_area = Rect { width: text_width as u16, ..results_area };

                    // Header and separator take 2 lines; remaining are data rows
                    let data_visible = (results_height as usize).saturating_sub(2);
                    self.result_visible_rows.set(data_visible);

                    let scroll = self.result_scroll.min(row_count.saturating_sub(1));
                    let data_end = (scroll + data_visible).min(row_count);

                    // Compute which columns are visible given h_col_offset and text_width
                    let h_offset = self.result_h_col_offset;
                    let total_cols = result.headers.len();
                    let mut visible_cols: Vec<usize> = Vec::new();
                    let mut used = 0usize;
                    for col_i in h_offset..total_cols {
                        let w = self.col_widths.get(col_i).copied().unwrap_or(0);
                        let needed = if visible_cols.is_empty() { w } else { 2 + w };
                        if used + needed > text_width && !visible_cols.is_empty() {
                            break;
                        }
                        used += needed;
                        visible_cols.push(col_i);
                    }
                    let last_visible = visible_cols.last().copied().unwrap_or(h_offset);
                    self.result_last_visible_col.set(last_visible);

                    let header_style = Style::default().fg(theme.accent).bold();
                    let sep_style = theme.text_dim;

                    let header_str: String = visible_cols
                        .iter()
                        .enumerate()
                        .map(|(vi, &col_i)| {
                            let h = &result.headers[col_i];
                            let w = self.col_widths.get(col_i).copied().unwrap_or(h.len());
                            if vi == 0 {
                                format!("{:<width$}", h, width = w)
                            } else {
                                format!("  {:<width$}", h, width = w)
                            }
                        })
                        .collect();

                    let sep_str: String = visible_cols
                        .iter()
                        .enumerate()
                        .map(|(vi, &col_i)| {
                            let w = self.col_widths.get(col_i).copied().unwrap_or(0);
                            if vi == 0 {
                                "─".repeat(w)
                            } else {
                                format!("──{}", "─".repeat(w))
                            }
                        })
                        .collect();

                    let mut lines: Vec<Line> = Vec::with_capacity(2 + data_visible);
                    lines.push(Line::from(Span::styled(
                        header_str.chars().take(text_width).collect::<String>(),
                        header_style,
                    )));
                    lines.push(Line::from(Span::styled(
                        sep_str.chars().take(text_width).collect::<String>(),
                        sep_style,
                    )));

                    for (idx, row) in result.rows[scroll..data_end].iter().enumerate() {
                        let abs_row = scroll + idx;
                        let text: String = visible_cols
                            .iter()
                            .enumerate()
                            .map(|(vi, &col_i)| {
                                let cell = row.get(col_i).map(|s| s.as_str()).unwrap_or("");
                                let w = self.col_widths.get(col_i).copied().unwrap_or(cell.len());
                                if vi == 0 {
                                    format!("{:<width$}", cell, width = w)
                                } else {
                                    format!("  {:<width$}", cell, width = w)
                                }
                            })
                            .collect();
                        let style =
                            if abs_row == self.result_selected_row { theme.selection } else { Style::default() };
                        lines.push(Line::from(Span::styled(text.chars().take(text_width).collect::<String>(), style)));
                    }

                    frame.render_widget(Paragraph::new(lines), text_area);

                    if row_count > 0 {
                        let mut scrollbar_state = ScrollbarState::new(row_count).position(self.result_selected_row);
                        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight).style(theme.border);
                        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
                    }

                    col_range = Some((h_offset + 1, last_visible + 1, total_cols));
                }
            }
        }

        let (mut status_text, status_style) = match &self.status {
            QueryPaneStatus::Connecting => ("Connecting…".to_string(), theme.text_dim),
            QueryPaneStatus::Connected(version) => {
                (format!("Connected — {version}"), Style::default().fg(theme.accent))
            }
            QueryPaneStatus::Executing => ("Executing…".to_string(), theme.text_dim),
            QueryPaneStatus::Error(msg) => (format!("Connection failed: {msg}"), theme.status_failed),
        };
        if let Some((first, last, total)) = col_range {
            if total > 1 {
                status_text.push_str(&format!("  cols {first}–{last} of {total}"));
            }
        }
        if let Some(hint) = self.export_hint_text() {
            status_text.push_str(&format!("  {hint}"));
        }
        frame.render_widget(Paragraph::new(status_text).style(status_style), status_area);

        if let Some(ref h) = self.history {
            render_history_popup(frame, area, h, theme);
        }
        if let Some(ref name_buf) = self.pending_save_name {
            render_save_name_popup(frame, area, name_buf, theme);
        }
        if let Some(ref sq) = self.saved_queries {
            render_saved_queries_popup(frame, area, sq, theme);
        }
        if let Some(ref path_buf) = self.export_dialog_path {
            render_export_dialog_popup(frame, area, path_buf, theme);
        }
    }

    fn handle_command(&mut self, _cmd: &PaneCommand) {}

    fn view_type(&self) -> &ViewType {
        &self.view_type
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
