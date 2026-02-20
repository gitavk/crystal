use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme::Theme;

pub struct StatusBarWidget<'a> {
    pub mode: &'a str,
    pub context: Option<&'a str>,
    pub help_key: Option<&'a str>,
    pub namespace_key: Option<&'a str>,
    pub context_key: Option<&'a str>,
    pub close_pane_key: Option<&'a str>,
    pub new_tab_key: Option<&'a str>,
    pub quit_key: Option<&'a str>,
    pub theme: &'a Theme,
}

impl<'a> StatusBarWidget<'a> {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let t = self.theme;
        let header_bg = t.header.bg.unwrap_or(Color::Reset);
        let status_bg = t.status_bar.bg.unwrap_or(Color::Reset);
        let status_fg = t.status_bar.fg.unwrap_or(Color::Reset);
        let sep = Style::default().fg(t.border.fg.unwrap_or(Color::Reset)).bg(status_bg);
        let key_style = Style::default().fg(t.accent).bg(status_bg);
        let desc_style = Style::default().fg(status_fg).bg(status_bg);
        let mut spans = Vec::new();

        let is_insert = self.mode.eq_ignore_ascii_case("insert");
        let mode_style = if is_insert {
            t.insert_mode.add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(header_bg).bg(t.accent).add_modifier(Modifier::BOLD)
        };

        spans.push(Span::styled(format!(" {} ", self.mode.to_uppercase()), mode_style));

        let ctx_raw = self.context.unwrap_or("no-context");
        let ctx_text: String = if ctx_raw.len() > 15 { format!("{}…", &ctx_raw[..14]) } else { ctx_raw.to_string() };
        spans.push(Span::styled(" │ ", sep));
        spans.push(Span::styled(ctx_text, Style::default().fg(status_fg).bg(status_bg).add_modifier(Modifier::DIM)));

        let keybindings: &[(&str, Option<&str>)] = &[
            ("Help", self.help_key),
            ("Namespace", self.namespace_key),
            ("Context", self.context_key),
            ("Close pane", self.close_pane_key),
            ("New tab", self.new_tab_key),
            ("Quit", self.quit_key),
        ];

        for (desc, key) in keybindings {
            if let Some(k) = key {
                spans.push(Span::styled(" │ ", sep));
                spans.push(Span::styled(k.to_string(), key_style));
                spans.push(Span::styled(format!(" {desc}"), desc_style));
            }
        }

        let left_used: u16 = spans.iter().map(|s| s.width() as u16).sum();
        let fill = area.width.saturating_sub(left_used);
        if fill > 0 {
            spans.push(Span::styled(" ".repeat(fill as usize), Style::default().bg(status_bg)));
        }

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Style::default().bg(status_bg));
        frame.render_widget(bar, area);
    }
}

#[cfg(test)]
mod tests;
