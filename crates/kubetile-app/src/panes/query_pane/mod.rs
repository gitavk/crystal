use std::any::Any;
use std::cell::Cell;
use std::collections::HashMap;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use kubetile_core::{QueryConfig, QueryResult};
use kubetile_tui::pane::{Pane, PaneCommand, ViewType};
use kubetile_tui::theme::Theme;

mod completion;
mod editor;
mod popups;
mod result;

use completion::CompletionState;
use popups::{QueryHistoryState, SavedQueriesState};

pub(super) enum QueryPaneStatus {
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
    completion: Option<CompletionState>,
    schema_tables: Vec<(String, String)>,
    column_cache: HashMap<String, Vec<(String, String)>>,
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
            completion: None,
            schema_tables: Vec::new(),
            column_cache: HashMap::new(),
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

    pub fn set_schema(&mut self, tables: Vec<(String, String)>, columns: HashMap<String, Vec<(String, String)>>) {
        self.schema_tables = tables;
        self.column_cache = columns;
    }

    pub fn set_error(&mut self, error: String) {
        self.status = QueryPaneStatus::Error(error);
    }
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
                    editor::render_cursor_line(line, self.cursor_col, normal_style, cursor_style)
                } else {
                    Line::from(Span::styled(line.clone(), normal_style))
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(editor_content), editor_area);

        let sep_text = "─".repeat(inner.width as usize);
        frame.render_widget(Paragraph::new(sep_text).style(theme.text_dim), sep_area);

        // Results — also produces col_range for the status line
        let mut col_range: Option<(usize, usize, usize)> = None;
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

                    let text_width = results_area.width.saturating_sub(1) as usize;
                    let scrollbar_area = Rect {
                        x: results_area.x + results_area.width.saturating_sub(1),
                        y: results_area.y,
                        width: 1,
                        height: results_height,
                    };
                    let text_area = Rect { width: text_width as u16, ..results_area };

                    let data_visible = (results_height as usize).saturating_sub(2);
                    self.result_visible_rows.set(data_visible);

                    let scroll = self.result_scroll.min(row_count.saturating_sub(1));
                    let data_end = (scroll + data_visible).min(row_count);

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

        if let Some(ref c) = self.completion {
            let visible_row = self.cursor_row.saturating_sub(editor_scroll);
            let popup_x = editor_area.x + self.cursor_col as u16;
            let popup_y = editor_area.y + visible_row as u16 + 1;
            completion::render_completion_popup(frame, area, popup_x, popup_y, c, theme);
        }
        if let Some(ref h) = self.history {
            popups::render_history_popup(frame, area, h, theme);
        }
        if let Some(ref name_buf) = self.pending_save_name {
            popups::render_save_name_popup(frame, area, name_buf, theme);
        }
        if let Some(ref sq) = self.saved_queries {
            popups::render_saved_queries_popup(frame, area, sq, theme);
        }
        if let Some(ref path_buf) = self.export_dialog_path {
            popups::render_export_dialog_popup(frame, area, path_buf, theme);
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
