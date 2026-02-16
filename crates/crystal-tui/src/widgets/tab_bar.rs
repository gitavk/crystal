use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme::Theme;

pub struct TabBarWidget<'a> {
    pub tabs: &'a [String],
    pub active: usize,
    pub theme: &'a Theme,
}

impl<'a> TabBarWidget<'a> {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let t = self.theme;
        let header_bg = t.header.bg.unwrap_or(Color::Reset);
        let mut spans = Vec::new();

        for (i, name) in self.tabs.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", t.border.bg(header_bg)));
            }

            let label = format!("[{}] {}", i + 1, name);
            let style = if i == self.active {
                Style::default().fg(t.accent).bg(header_bg).add_modifier(Modifier::BOLD)
            } else {
                t.text_dim.bg(header_bg)
            };
            spans.push(Span::styled(label, style));
        }

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Style::default().bg(header_bg));
        frame.render_widget(bar, area);
    }
}

#[cfg(test)]
mod tests;
