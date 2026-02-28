use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::layout::QueryDialogFieldView;
use crate::theme::Theme;

pub struct QueryDialogWidget<'a> {
    pub pod: &'a str,
    pub namespace: &'a str,
    pub database: &'a str,
    pub user: &'a str,
    pub password: &'a str,
    pub port: &'a str,
    pub active_field: QueryDialogFieldView,
    pub theme: &'a Theme,
}

impl<'a> QueryDialogWidget<'a> {
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let t = self.theme;
        let width = 60.min(area.width.saturating_sub(4));
        let height = 11.min(area.height.saturating_sub(2));
        let popup = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .title(" Query Database ")
            .title_style(Style::default().fg(t.accent).bold())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent))
            .style(t.overlay);

        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // pod/namespace
                Constraint::Length(1), // blank
                Constraint::Length(1), // database
                Constraint::Length(1), // user
                Constraint::Length(1), // password
                Constraint::Length(1), // port
                Constraint::Length(1), // help
            ])
            .split(inner);

        frame.render_widget(
            Paragraph::new(format!("Pod: {}   Namespace: {}", self.pod, self.namespace))
                .style(Style::default().fg(t.fg)),
            chunks[0],
        );

        let field_style = |active: bool| {
            if active {
                Style::default().fg(t.accent).bold()
            } else {
                Style::default().fg(t.fg)
            }
        };

        let db_text = if self.database.is_empty() { "_" } else { self.database };
        let user_text = if self.user.is_empty() { "_" } else { self.user };
        let pw_text = if self.password.is_empty() { "_" } else { "***" };
        let port_text = if self.port.is_empty() { "_" } else { self.port };

        frame.render_widget(
            Paragraph::new(format!("Database : {db_text}"))
                .style(field_style(matches!(self.active_field, QueryDialogFieldView::Database))),
            chunks[2],
        );
        frame.render_widget(
            Paragraph::new(format!("User     : {user_text}"))
                .style(field_style(matches!(self.active_field, QueryDialogFieldView::User))),
            chunks[3],
        );
        frame.render_widget(
            Paragraph::new(format!("Password : {pw_text}"))
                .style(field_style(matches!(self.active_field, QueryDialogFieldView::Password))),
            chunks[4],
        );
        frame.render_widget(
            Paragraph::new(format!("Port     : {port_text}"))
                .style(field_style(matches!(self.active_field, QueryDialogFieldView::Port))),
            chunks[5],
        );
        frame.render_widget(
            Paragraph::new("Tab next field │ Enter confirm │ Esc cancel")
                .style(t.text_dim)
                .alignment(Alignment::Center),
            chunks[6],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

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

    #[test]
    fn dialog_renders_all_fields() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::default();

        terminal
            .draw(|frame| {
                let widget = QueryDialogWidget {
                    pod: "postgres-0",
                    namespace: "kubetile-prod",
                    database: "appdb",
                    user: "postgres",
                    password: "secret",
                    port: "5432",
                    active_field: QueryDialogFieldView::Database,
                    theme: &theme,
                };
                widget.render(frame, frame.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend().buffer());
        assert!(content.contains("Query Database"));
        assert!(content.contains("postgres-0"));
        assert!(content.contains("Database : appdb"));
        assert!(content.contains("User     : postgres"));
        assert!(content.contains("Password : ***"));
        assert!(content.contains("Port     : 5432"));
    }

    #[test]
    fn password_always_masked() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::default();

        terminal
            .draw(|frame| {
                let widget = QueryDialogWidget {
                    pod: "pg",
                    namespace: "ns",
                    database: "db",
                    user: "u",
                    password: "super-secret",
                    port: "5432",
                    active_field: QueryDialogFieldView::Password,
                    theme: &theme,
                };
                widget.render(frame, frame.area());
            })
            .unwrap();

        let content = buffer_to_string(terminal.backend().buffer());
        assert!(!content.contains("super-secret"));
        assert!(content.contains("***"));
    }
}
