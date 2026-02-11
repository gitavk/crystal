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
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_tab_bar(tabs: &[String], active: usize, width: u16) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                let widget = TabBarWidget { tabs, active };
                widget.render(frame, area);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn renders_correct_number_of_tabs() {
        let tabs: Vec<String> = vec!["Pods".into(), "Services".into(), "Terminal".into()];
        let buf = render_tab_bar(&tabs, 0, 60);
        let content: String = buf.content().iter().map(|c| c.symbol().chars().next().unwrap_or(' ')).collect();
        assert!(content.contains("[1] Pods"));
        assert!(content.contains("[2] Services"));
        assert!(content.contains("[3] Terminal"));
    }

    #[test]
    fn active_tab_is_visually_distinct() {
        let tabs: Vec<String> = vec!["Pods".into(), "Services".into()];
        let buf = render_tab_bar(&tabs, 1, 40);

        let pods_cell = &buf.cell((0, 0)).unwrap();
        let pods_fg = pods_cell.fg;

        let sep_width = " │ ".len() as u16;
        let first_tab_width = "[1] Pods".len() as u16;
        let services_x = first_tab_width + sep_width;
        let services_cell = &buf.cell((services_x, 0)).unwrap();
        let services_fg = services_cell.fg;

        assert_ne!(pods_fg, services_fg, "active and inactive tabs should have different colors");
    }

    #[test]
    fn single_tab_renders() {
        let tabs: Vec<String> = vec!["Main".into()];
        let buf = render_tab_bar(&tabs, 0, 30);
        let content: String = buf.content().iter().map(|c| c.symbol().chars().next().unwrap_or(' ')).collect();
        assert!(content.contains("[1] Main"));
        assert!(!content.contains("│"));
    }
}
