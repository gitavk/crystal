use std::any::Any;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use kubetile_core::QueryConfig;
use kubetile_tui::pane::{Pane, PaneCommand, ViewType};
use kubetile_tui::theme::Theme;

enum QueryPaneStatus {
    Connecting,
    Connected(String),
    Error(String),
}

pub struct QueryPane {
    view_type: ViewType,
    pod_name: String,
    namespace: String,
    status: QueryPaneStatus,
}

impl QueryPane {
    pub fn new(config: &QueryConfig) -> Self {
        Self {
            view_type: ViewType::Query(config.pod.clone()),
            pod_name: config.pod.clone(),
            namespace: config.namespace.clone(),
            status: QueryPaneStatus::Connecting,
        }
    }

    pub fn set_connected(&mut self, version: String) {
        self.status = QueryPaneStatus::Connected(version);
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

        if inner.height == 0 {
            return;
        }

        let status_area =
            Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(1), width: inner.width, height: 1 };

        let (status_text, status_style) = match &self.status {
            QueryPaneStatus::Connecting => ("Connecting…".to_string(), theme.text_dim),
            QueryPaneStatus::Connected(version) => {
                (format!("Connected — {version}"), Style::default().fg(theme.accent))
            }
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
