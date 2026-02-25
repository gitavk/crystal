use std::any::Any;
use std::cell::Cell;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

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
    result_lines: Vec<String>,
    result_scroll: usize,
    result_max_scroll: Cell<usize>,
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
            result_lines: Vec::new(),
            result_scroll: 0,
            result_max_scroll: Cell::new(0),
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
        self.status = QueryPaneStatus::Executing;
    }

    pub fn set_result(&mut self, result: QueryResult) {
        self.result_lines = format_result(&result);
        self.result_scroll = 0;
        let label = self.connected_version.clone().unwrap_or_else(|| "Ready".to_string());
        self.status = QueryPaneStatus::Connected(label);
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

    pub fn editor_home(&mut self) {
        self.cursor_col = 0;
    }

    pub fn editor_end(&mut self) {
        self.cursor_col = self.editor_lines[self.cursor_row].chars().count();
    }

    pub fn editor_content(&self) -> String {
        self.editor_lines.join("\n")
    }

    pub fn scroll_up(&mut self) {
        let max = self.result_max_scroll.get();
        self.result_scroll = self.result_scroll.saturating_add(1).min(max);
    }

    pub fn scroll_down(&mut self) {
        self.result_scroll = self.result_scroll.saturating_sub(1);
    }

    fn adjust_editor_scroll(&mut self) {
        let h = self.editor_area_height.get().max(1);
        if self.cursor_row < self.editor_scroll {
            self.editor_scroll = self.cursor_row;
        } else if self.cursor_row >= self.editor_scroll + h {
            self.editor_scroll = self.cursor_row + 1 - h;
        }
    }
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(s.len())
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

fn format_result(result: &QueryResult) -> Vec<String> {
    let mut lines = Vec::new();
    if !result.headers.is_empty() {
        lines.push(result.headers.join(" | "));
    }
    for row in &result.rows {
        lines.push(row.join(" | "));
    }
    lines
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

        // Layout:
        //   rows 0..editor_height-1:  editor lines (scrollable)
        //   row  editor_height:       separator
        //   rows editor_height+1..h-2: results (scrollable)
        //   row  h-1:                 status bar

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

        if results_height > 0 {
            let total = self.result_lines.len();
            let visible = results_height as usize;
            let max_scroll = total.saturating_sub(visible);
            self.result_max_scroll.set(max_scroll);
            let offset = self.result_scroll.min(max_scroll);
            let end = (offset + visible).min(total);

            let content: Vec<Line> = if self.result_lines.is_empty() {
                vec![Line::from(Span::styled("No results yet", theme.text_dim))]
            } else {
                self.result_lines[offset..end].iter().map(|s| Line::from(s.as_str())).collect()
            };
            frame.render_widget(Paragraph::new(content), results_area);
        }

        let (status_text, status_style) = match &self.status {
            QueryPaneStatus::Connecting => ("Connecting…".to_string(), theme.text_dim),
            QueryPaneStatus::Connected(version) => {
                (format!("Connected — {version}"), Style::default().fg(theme.accent))
            }
            QueryPaneStatus::Executing => ("Executing…".to_string(), theme.text_dim),
            QueryPaneStatus::Error(msg) => (format!("Connection failed: {msg}"), theme.status_failed),
        };
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
