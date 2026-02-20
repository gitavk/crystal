use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::layout::PortForwardFieldView;
use crate::theme::Theme;

pub struct PortForwardDialogWidget<'a> {
    pub pod: &'a str,
    pub namespace: &'a str,
    pub local_port: &'a str,
    pub remote_port: &'a str,
    pub active_field: PortForwardFieldView,
    pub theme: &'a Theme,
}

impl<'a> PortForwardDialogWidget<'a> {
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let t = self.theme;
        let width = 56.min(area.width.saturating_sub(4));
        let height = 10.min(area.height.saturating_sub(2));
        let popup = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .title(" Port Forward ")
            .title_style(Style::default().fg(t.accent).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent))
            .style(t.overlay);

        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
            .split(inner);

        let target = Paragraph::new(format!("Pod: {}   Namespace: {}", self.pod, self.namespace))
            .style(Style::default().fg(t.fg));
        frame.render_widget(target, chunks[0]);

        let local_style = if matches!(self.active_field, PortForwardFieldView::Local) {
            Style::default().fg(t.accent).bold()
        } else {
            Style::default().fg(t.fg)
        };
        let remote_style = if matches!(self.active_field, PortForwardFieldView::Remote) {
            Style::default().fg(t.accent).bold()
        } else {
            Style::default().fg(t.fg)
        };

        let local_text = if self.local_port.is_empty() { "_" } else { self.local_port };
        let remote_text = if self.remote_port.is_empty() { "_" } else { self.remote_port };

        frame.render_widget(Paragraph::new(format!("Local port : {local_text}")).style(local_style), chunks[1]);
        frame.render_widget(Paragraph::new(format!("Remote port: {remote_text}")).style(remote_style), chunks[2]);

        let help = Paragraph::new("Tab switch field | Enter start | Esc cancel")
            .style(t.text_dim)
            .alignment(Alignment::Center);
        frame.render_widget(help, chunks[3]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    #[test]
    fn dialog_renders_both_ports_and_target() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::default();

        terminal
            .draw(|frame| {
                let widget = PortForwardDialogWidget {
                    pod: "api-7d8b6f5c9f",
                    namespace: "default",
                    local_port: "3715",
                    remote_port: "8080",
                    active_field: PortForwardFieldView::Remote,
                    theme: &theme,
                };
                widget.render(frame, frame.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Port Forward"));
        assert!(content.contains("api-7d8b6f5c9f"));
        assert!(content.contains("Local port : 3715"));
        assert!(content.contains("Remote port: 8080"));
    }

    fn buffer_to_string(buf: &Buffer) -> String {
        let mut s = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }
}
