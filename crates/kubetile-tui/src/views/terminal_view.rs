use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme::Theme;
use kubetile_terminal::render_terminal_screen;

pub type SessionId = u64;

pub struct TerminalView {
    session_id: SessionId,
    scrollback_offset: usize,
    title: String,
}

impl TerminalView {
    pub fn new(session_id: SessionId, title: String) -> Self {
        Self { session_id, scrollback_offset: 0, title }
    }

    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }

    pub fn scrollback_offset(&self) -> usize {
        self.scrollback_offset
    }

    pub fn render(&self, screen: &vt100::Screen, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme) {
        if area.height < 2 || area.width < 4 {
            return;
        }

        let t = theme;
        let header_bg = t.header.bg.unwrap_or(Color::Reset);
        let text_dim_color = t.text_dim.fg.unwrap_or(Color::Reset);

        let title_style = if focused { Style::new().fg(t.fg).bg(header_bg) } else { t.text_dim.bg(header_bg) };

        let title_bar =
            Paragraph::new(Line::from(vec![Span::styled(&self.title, title_style)])).style(Style::new().bg(header_bg));
        let title_area = Rect { x: area.x, y: area.y, width: area.width, height: 1 };
        frame.render_widget(title_bar, title_area);

        let content_area = Rect { x: area.x, y: area.y + 1, width: area.width, height: area.height - 1 };

        render_terminal_screen(screen, content_area, frame.buffer_mut());

        if focused {
            if self.scrollback_offset == 0 && !screen.hide_cursor() {
                let (cursor_row, cursor_col) = screen.cursor_position();
                let cx = content_area.x + cursor_col;
                let cy = content_area.y + cursor_row;
                if cx < content_area.x + content_area.width && cy < content_area.y + content_area.height {
                    frame.set_cursor_position((cx, cy));
                }
            }
        } else {
            dim_area(frame.buffer_mut(), content_area, text_dim_color);
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scrollback_offset = self.scrollback_offset.saturating_add(lines);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scrollback_offset = self.scrollback_offset.saturating_sub(lines);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scrollback_offset = 0;
    }
}

fn dim_area(buf: &mut Buffer, area: Rect, dim_color: Color) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_fg(dim_color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_screen(rows: u16, cols: u16, input: &[u8]) -> vt100::Parser {
        let mut parser = vt100::Parser::new(rows, cols, 100);
        parser.process(input);
        parser
    }

    #[test]
    fn title_bar_renders_in_first_row() {
        let view = TerminalView::new(1, "[kubetile:minikube/default] /bin/bash".into());
        let parser = make_screen(24, 80, b"");
        let area = Rect::new(0, 0, 80, 25);
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 25);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.render(parser.screen(), frame, area, true, &theme);
            })
            .unwrap();

        let result_buf = terminal.backend().buffer().clone();
        let title = "[kubetile:minikube/default] /bin/bash";
        for (i, ch) in title.chars().enumerate() {
            assert_eq!(result_buf[(i as u16, 0)].symbol(), ch.to_string(), "char at position {i}");
        }
    }

    #[test]
    fn focused_pane_shows_cursor() {
        let view = TerminalView::new(1, "test".into());
        let parser = make_screen(24, 80, b"AB");
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 25);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.render(parser.screen(), frame, Rect::new(0, 0, 80, 25), true, &theme);
            })
            .unwrap();

        let cursor = terminal.backend_mut().get_cursor_position().unwrap();
        assert_eq!(cursor, Position::new(2, 1), "cursor should be at col 2 (after 'AB'), row 1 (below title)");
    }

    #[test]
    fn unfocused_pane_does_not_show_cursor() {
        let view = TerminalView::new(1, "test".into());
        let parser = make_screen(24, 80, b"AB");
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 25);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.render(parser.screen(), frame, Rect::new(0, 0, 80, 25), false, &theme);
            })
            .unwrap();

        let result_buf = terminal.backend().buffer().clone();
        let cell = &result_buf[(0, 1)];
        let text_dim_color = theme.text_dim.fg.unwrap_or(Color::Reset);
        assert_eq!(cell.fg, text_dim_color, "unfocused content should be dimmed");
    }

    #[test]
    fn scroll_up_increases_offset() {
        let mut view = TerminalView::new(1, "test".into());
        assert_eq!(view.scrollback_offset(), 0);
        view.scroll_up(5);
        assert_eq!(view.scrollback_offset(), 5);
        view.scroll_up(3);
        assert_eq!(view.scrollback_offset(), 8);
    }

    #[test]
    fn scroll_down_decreases_offset_clamped() {
        let mut view = TerminalView::new(1, "test".into());
        view.scroll_up(10);
        view.scroll_down(3);
        assert_eq!(view.scrollback_offset(), 7);
        view.scroll_down(100);
        assert_eq!(view.scrollback_offset(), 0);
    }

    #[test]
    fn scroll_to_bottom_resets_offset() {
        let mut view = TerminalView::new(1, "test".into());
        view.scroll_up(42);
        view.scroll_to_bottom();
        assert_eq!(view.scrollback_offset(), 0);
    }

    #[test]
    fn tiny_area_does_not_panic() {
        let view = TerminalView::new(1, "test".into());
        let parser = make_screen(24, 80, b"Hello");
        let theme = Theme::default();

        let backend = ratatui::backend::TestBackend::new(80, 25);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.render(parser.screen(), frame, Rect::new(0, 0, 2, 1), true, &theme);
            })
            .unwrap();
    }
}
