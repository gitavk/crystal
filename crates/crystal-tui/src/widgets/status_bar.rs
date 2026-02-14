use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme;

pub struct StatusBarWidget<'a> {
    pub mode: &'a str,
    pub hints: &'a [(String, String)],
    pub cluster: Option<&'a str>,
    pub namespace: Option<&'a str>,
}

impl<'a> StatusBarWidget<'a> {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let mut spans = Vec::new();

        let is_insert = self.mode.eq_ignore_ascii_case("insert");
        let mode_style = if is_insert {
            Style::default().fg(theme::INSERT_MODE_FG).bg(theme::INSERT_MODE_BG).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::HEADER_BG).bg(theme::ACCENT).add_modifier(Modifier::BOLD)
        };

        spans.push(Span::styled(format!(" {} ", self.mode.to_uppercase()), mode_style));

        for (key, desc) in self.hints {
            spans.push(Span::styled(" │ ", Style::default().fg(theme::BORDER_COLOR).bg(theme::STATUS_BG)));
            spans.push(Span::styled(format!("<{key}>"), Style::default().fg(theme::ACCENT).bg(theme::STATUS_BG)));
            spans.push(Span::styled(format!(" {desc}"), Style::default().fg(theme::STATUS_FG).bg(theme::STATUS_BG)));
        }

        let right_text = match (self.cluster, self.namespace) {
            (Some(c), Some(ns)) => format!("{c} / {ns} "),
            (Some(c), None) => format!("{c} / n/a "),
            _ => "No cluster ".into(),
        };
        let right_width = right_text.len() as u16;
        let left_used: u16 = spans.iter().map(|s| s.width() as u16).sum();
        let fill = area.width.saturating_sub(left_used + right_width);

        if fill > 0 {
            spans.push(Span::styled(" ".repeat(fill as usize), Style::default().bg(theme::STATUS_BG)));
        }

        spans.push(Span::styled(
            right_text,
            Style::default().fg(theme::STATUS_FG).bg(theme::STATUS_BG).add_modifier(Modifier::DIM),
        ));

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Style::default().bg(theme::STATUS_BG));
        frame.render_widget(bar, area);
    }
}

#[cfg(test)]
mod tests;
