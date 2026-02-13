use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::theme;

pub struct ConfirmDialogWidget<'a> {
    pub message: &'a str,
}

impl<'a> ConfirmDialogWidget<'a> {
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let lines: Vec<&str> = self.message.lines().collect();
        let max_line_width = lines.iter().map(|l| l.len()).max().unwrap_or(0);
        let width = (max_line_width as u16 + 6).max(40).min(area.width.saturating_sub(4));
        let height = (lines.len() as u16 + 6).min(area.height.saturating_sub(2));

        let popup = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .title(" Confirm ")
            .title_style(Style::default().fg(theme::STATUS_FAILED).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::STATUS_FAILED))
            .style(Style::default().bg(theme::OVERLAY_BG));

        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1), Constraint::Length(1)])
            .split(inner);

        let msg =
            Paragraph::new(self.message).style(Style::default().fg(theme::HEADER_FG)).alignment(Alignment::Center);
        frame.render_widget(msg, chunks[0]);

        let buttons = Paragraph::new(Line::from(vec![
            Span::styled("[y]", Style::default().fg(theme::STATUS_RUNNING).bold()),
            Span::styled(" Confirm  ", Style::default().fg(theme::STATUS_FG)),
            Span::styled("[n/Esc]", Style::default().fg(theme::STATUS_FAILED).bold()),
            Span::styled(" Cancel", Style::default().fg(theme::STATUS_FG)),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(buttons, chunks[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    #[test]
    fn confirm_dialog_renders_message_and_buttons() {
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let widget = ConfirmDialogWidget { message: "Delete pod nginx-abc123\nin namespace default?" };
                widget.render(frame, frame.area());
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buf);
        assert!(content.contains("Confirm"), "should show Confirm title/button");
        assert!(content.contains("nginx-abc123"), "should show resource name");
        assert!(content.contains("Cancel"), "should show cancel button");
        assert!(content.contains("[y]"), "should show y key hint");
    }

    fn buffer_to_string(buf: &Buffer) -> String {
        let mut s = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                let cell = &buf[(x, y)];
                s.push_str(cell.symbol());
            }
            s.push('\n');
        }
        s
    }
}
