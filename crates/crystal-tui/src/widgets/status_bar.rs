use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme::Theme;

pub struct StatusBarWidget<'a> {
    pub mode: &'a str,
    pub hints: &'a [(String, String)],
    pub cluster: Option<&'a str>,
    pub namespace: Option<&'a str>,
    pub theme: &'a Theme,
}

impl<'a> StatusBarWidget<'a> {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let t = self.theme;
        let header_bg = t.header.bg.unwrap_or(Color::Reset);
        let status_bg = t.status_bar.bg.unwrap_or(Color::Reset);
        let status_fg = t.status_bar.fg.unwrap_or(Color::Reset);
        let mut spans = Vec::new();

        let is_insert = self.mode.eq_ignore_ascii_case("insert");
        let mode_style = if is_insert {
            t.insert_mode.add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(header_bg).bg(t.accent).add_modifier(Modifier::BOLD)
        };

        spans.push(Span::styled(format!(" {} ", self.mode.to_uppercase()), mode_style));

        for (key, desc) in self.hints {
            spans.push(Span::styled(" │ ", t.border.bg(status_bg)));
            spans.push(Span::styled(format!("<{key}>"), Style::default().fg(t.accent).bg(status_bg)));
            spans.push(Span::styled(format!(" {desc}"), Style::default().fg(status_fg).bg(status_bg)));
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
            spans.push(Span::styled(" ".repeat(fill as usize), Style::default().bg(status_bg)));
        }

        spans.push(Span::styled(right_text, Style::default().fg(status_fg).bg(status_bg).add_modifier(Modifier::DIM)));

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Style::default().bg(status_bg));
        frame.render_widget(bar, area);
    }
}

#[cfg(test)]
mod tests;
