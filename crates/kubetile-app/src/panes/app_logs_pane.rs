use std::any::Any;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app_log;
use kubetile_tui::pane::{Pane, PaneCommand, ViewType};

pub struct AppLogsPane {
    view_type: ViewType,
    lines: Vec<String>,
    scroll: usize,
    follow: bool,
}

impl AppLogsPane {
    pub fn new() -> Self {
        Self { view_type: ViewType::Plugin("AppLogs".into()), lines: Vec::new(), scroll: 0, follow: true }
    }

    pub fn poll(&mut self) {
        self.lines = app_log::recent_lines(1500);
        if self.follow {
            self.scroll = self.max_scroll();
        } else {
            self.scroll = self.scroll.min(self.max_scroll());
        }
    }

    fn max_scroll(&self) -> usize {
        self.lines.len().saturating_sub(1)
    }
}

impl Pane for AppLogsPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &kubetile_tui::theme::Theme) {
        let border_style = if focused { theme.border_active } else { theme.border };
        let mode = if self.follow { "follow" } else { "paused" };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" App Logs ({mode}) "))
            .title_style(Style::default().fg(theme.accent).bold());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let text = if self.lines.is_empty() { "No app logs yet".to_string() } else { self.lines.join("\n") };
        let paragraph = Paragraph::new(text).scroll((self.scroll as u16, 0));
        frame.render_widget(paragraph, inner);
    }

    fn handle_command(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::SelectNext | PaneCommand::ScrollDown => {
                self.follow = false;
                self.scroll = self.scroll.saturating_add(1).min(self.max_scroll());
            }
            PaneCommand::SelectPrev | PaneCommand::ScrollUp => {
                self.follow = false;
                self.scroll = self.scroll.saturating_sub(1);
            }
            PaneCommand::ToggleFollow => {
                self.follow = !self.follow;
                if self.follow {
                    self.scroll = self.max_scroll();
                }
            }
            _ => {}
        }
    }

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
