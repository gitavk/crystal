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
    editor_line: String,
    result_lines: Vec<String>,
    result_scroll: usize,
    result_max_scroll: Cell<usize>,
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
            editor_line: String::new(),
            result_lines: Vec::new(),
            result_scroll: 0,
            result_max_scroll: Cell::new(0),
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
        self.editor_line.push(c);
    }

    pub fn editor_pop(&mut self) {
        self.editor_line.pop();
    }

    pub fn editor_content(&self) -> &str {
        &self.editor_line
    }

    pub fn scroll_up(&mut self) {
        let max = self.result_max_scroll.get();
        self.result_scroll = self.result_scroll.saturating_add(1).min(max);
    }

    pub fn scroll_down(&mut self) {
        self.result_scroll = self.result_scroll.saturating_sub(1);
    }
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

        if inner.height < 3 {
            return;
        }

        // Layout:
        //   row 0:        editor line
        //   row 1:        separator
        //   rows 2..h-2:  results (scrollable)
        //   row h-1:      status bar

        let editor_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 };
        let sep_area = Rect { x: inner.x, y: inner.y + 1, width: inner.width, height: 1 };
        let results_height = inner.height.saturating_sub(3);
        let results_area = Rect { x: inner.x, y: inner.y + 2, width: inner.width, height: results_height };
        let status_area =
            Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(1), width: inner.width, height: 1 };

        let editor_text = format!("> {}_", self.editor_line);
        let editor_style = if focused { Style::default().fg(theme.accent) } else { Style::default().fg(theme.fg) };
        frame.render_widget(Paragraph::new(editor_text).style(editor_style), editor_area);

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
