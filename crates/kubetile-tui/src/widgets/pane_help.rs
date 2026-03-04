use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::theme::Theme;

pub struct PaneHelpView<'a> {
    pub title: &'a str,
    pub entries: &'a [(String, String)],
}

pub struct PaneHelpWidget<'a> {
    pub view: &'a PaneHelpView<'a>,
    pub theme: &'a Theme,
}

impl<'a> PaneHelpWidget<'a> {
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let t = self.theme;
        let entries = self.view.entries;

        let key_col_width: usize = entries.iter().map(|(k, _)| k.len()).max().unwrap_or(8) + 2;
        let desc_col_width: usize = entries.iter().map(|(_, d)| d.len()).max().unwrap_or(8) + 2;

        let content_width = (key_col_width + desc_col_width) as u16;
        let width = (content_width + 4).max(40).min(area.width.saturating_sub(4));
        let row_count = entries.len() as u16;
        let height = (row_count + 4).max(6).min(area.height.saturating_sub(2));

        let popup = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        frame.render_widget(Clear, popup);

        let title = format!(" {} ", self.view.title);
        let block = Block::default()
            .title(title.as_str())
            .title_style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(t.border_active)
            .style(t.overlay);

        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        let mut lines: Vec<Line> = Vec::new();
        for (key, desc) in entries {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{key:<key_col_width$}"),
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc.as_str(), t.text_dim),
            ]));
        }

        let content = Paragraph::new(lines);
        frame.render_widget(content, chunks[0]);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("Esc", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            Span::styled(" / ", t.text_dim),
            Span::styled("q", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)),
            Span::styled("  close", t.text_dim),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(footer, chunks[1]);
    }
}
