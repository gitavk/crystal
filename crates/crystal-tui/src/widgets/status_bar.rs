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

        spans.push(Span::styled(
            format!(" {} ", self.mode.to_uppercase()),
            Style::default().fg(theme::HEADER_BG).bg(theme::ACCENT).add_modifier(Modifier::BOLD),
        ));

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
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_status_bar(
        mode: &str,
        hints: &[(String, String)],
        cluster: Option<&str>,
        namespace: Option<&str>,
        width: u16,
    ) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                let widget = StatusBarWidget { mode, hints, cluster, namespace };
                widget.render(frame, area);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buf_text(buf: &ratatui::buffer::Buffer) -> String {
        buf.content().iter().map(|c| c.symbol().chars().next().unwrap_or(' ')).collect()
    }

    #[test]
    fn shows_hints() {
        let hints = vec![("Alt+v".into(), "Split V".into()), ("?".into(), "Help".into())];
        let buf = render_status_bar("Normal", &hints, Some("minikube"), Some("default"), 120);
        let text = buf_text(&buf);
        assert!(text.contains("NORMAL"));
        assert!(text.contains("<Alt+v>"));
        assert!(text.contains("Split V"));
        assert!(text.contains("<?>"));
        assert!(text.contains("Help"));
    }

    #[test]
    fn shows_cluster_info() {
        let buf = render_status_bar("Normal", &[], Some("minikube"), Some("default"), 80);
        let text = buf_text(&buf);
        assert!(text.contains("minikube / default"));
    }

    #[test]
    fn shows_no_cluster_when_disconnected() {
        let buf = render_status_bar("Normal", &[], None, None, 80);
        let text = buf_text(&buf);
        assert!(text.contains("No cluster"));
    }

    #[test]
    fn mode_label_is_uppercased() {
        let buf = render_status_bar("Normal", &[], Some("ctx"), Some("ns"), 80);
        let text = buf_text(&buf);
        assert!(text.contains("NORMAL"));
    }
}
