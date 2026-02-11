use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme;

pub struct TabBarWidget<'a> {
    pub tabs: &'a [String],
    pub active: usize,
}

impl<'a> TabBarWidget<'a> {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut spans = Vec::new();

        for (i, name) in self.tabs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(theme::BORDER_COLOR).bg(theme::HEADER_BG)));
            }

            let label = format!("[{}] {}", i + 1, name);
            let style = if i == self.active {
                Style::default().fg(theme::ACCENT).bg(theme::HEADER_BG).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT_DIM).bg(theme::HEADER_BG)
            };
            spans.push(Span::styled(label, style));
        }

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Style::default().bg(theme::HEADER_BG));
        frame.render_widget(bar, area);
    }
}

#[cfg(test)]
mod tests;
