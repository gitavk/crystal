use std::any::Any;
use std::cell::Cell;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use kubetile_core::{QueryConfig, QueryResult};
use kubetile_tui::pane::{Pane, PaneCommand, ViewType};
use kubetile_tui::theme::Theme;

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
        }
    }

    pub fn is_connecting(&self) -> bool {
        matches!(self.status, QueryPaneStatus::Connecting)
    }

    pub fn set_connected(&mut self, version: String) {
        self.connected_version = Some(version.clone());
        self.status = QueryPaneStatus::Connected(version);
    }

    pub fn set_executing(&mut self) {
        self.result = None;
        self.col_widths.clear();
        self.result_selected_row = 0;
        self.result_scroll = 0;
        self.result_h_col_offset = 0;
        self.status = QueryPaneStatus::Executing;
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
        frame.render_widget(Paragraph::new(status_text).style(status_style), status_area);
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
