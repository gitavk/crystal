use std::any::Any;
use std::cell::Cell;
use std::collections::VecDeque;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app_log;
use kubetile_tui::pane::{Pane, PaneCommand, ViewType};

const LOG_LINE_LIMIT: usize = 1500;

pub struct AppLogsPane {
    view_type: ViewType,
    lines: VecDeque<String>,
    scroll: usize,
    follow: bool,
    last_cursor: usize,
    visible_height: Cell<u16>,
}

impl AppLogsPane {
    pub fn new() -> Self {
        Self {
            view_type: ViewType::Plugin("AppLogs".into()),
            lines: VecDeque::new(),
            scroll: 0,
            follow: true,
            last_cursor: 0,
            visible_height: Cell::new(0),
        }
    }

    pub fn poll(&mut self) {
        if self.lines.is_empty() {
            let (lines, cursor) = app_log::recent_lines_with_cursor(LOG_LINE_LIMIT);
            self.lines = VecDeque::from(lines);
            self.last_cursor = cursor;
        } else {
            let (new_lines, cursor) = app_log::fetch_since(self.last_cursor);
            self.last_cursor = cursor;
            if !new_lines.is_empty() {
                self.lines.extend(new_lines);
                while self.lines.len() > LOG_LINE_LIMIT {
                    self.lines.pop_front();
                }
            }
        }
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
        self.visible_height.set(inner.height);
        frame.render_widget(block, area);

        let text = if self.lines.is_empty() {
            "No app logs yet".to_string()
        } else {
            self.lines.iter().cloned().collect::<Vec<_>>().join("\n")
        };
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
            PaneCommand::PageUp => {
                self.follow = false;
                let page = self.visible_height.get().max(1) as usize;
                self.scroll = self.scroll.saturating_sub(page);
            }
            PaneCommand::PageDown => {
                self.follow = false;
                let page = self.visible_height.get().max(1) as usize;
                self.scroll = self.scroll.saturating_add(page).min(self.max_scroll());
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
