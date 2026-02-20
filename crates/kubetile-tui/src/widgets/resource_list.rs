use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table};

use crate::theme::Theme;

pub struct ResourceListWidget<'a> {
    pub title: &'a str,
    pub headers: &'a [String],
    pub items: &'a [&'a Vec<String>],
    pub selected: Option<usize>,
    pub scroll_offset: usize,
    pub loading: bool,
    pub error: Option<&'a str>,
    pub focused: bool,
    pub filter_text: Option<&'a str>,
    pub sort_column: Option<usize>,
    pub sort_ascending: bool,
    pub total_count: usize,
    pub all_namespaces: bool,
    pub theme: &'a Theme,
}

impl<'a> ResourceListWidget<'a> {
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let t = self.theme;
        let border_color = if self.focused { t.accent } else { t.border.fg.unwrap_or(Color::Reset) };

        let title_suffix = if self.all_namespaces { " (All Namespaces)" } else { "" };
        let count_display = if self.filter_text.is_some() {
            format!(" {}/{} ", self.items.len(), self.total_count)
        } else {
            format!(" {} ", self.total_count)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(format!(" {}{} ", self.title, title_suffix))
            .title_style(Style::default().fg(t.accent).bold())
            .title_bottom(Line::from(count_display).right_aligned().style(t.text_dim));

        if self.loading {
            let msg = Paragraph::new("Loading...").style(t.text_dim).block(block);
            frame.render_widget(msg, area);
            return;
        }

        if let Some(err) = self.error {
            let msg = Paragraph::new(format!("Error: {err}")).style(t.status_failed).block(block);
            frame.render_widget(msg, area);
            return;
        }

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut content_area = inner;

        if let Some(filter) = self.filter_text {
            let filter_area = Rect { height: 1, ..content_area };
            content_area =
                Rect { y: content_area.y + 1, height: content_area.height.saturating_sub(1), ..content_area };

            let filter_line = Line::from(vec![
                Span::styled("Filter: ", t.text_dim),
                Span::styled(filter, Style::default().fg(t.accent)),
                Span::styled("_", Style::default().fg(t.accent)),
            ]);
            frame.render_widget(Paragraph::new(filter_line), filter_area);
        }

        if self.items.is_empty() && self.filter_text.is_none() {
            let msg = Paragraph::new("No resources found").style(t.text_dim);
            frame.render_widget(msg, content_area);
            return;
        }

        if self.items.is_empty() {
            let msg = Paragraph::new("No matches").style(t.text_dim);
            frame.render_widget(msg, content_area);
            return;
        }

        let header_fg = t.accent;
        let header_cells: Vec<Cell> = self
            .headers
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let label = if self.sort_column == Some(i) {
                    let arrow = if self.sort_ascending { " ▲" } else { " ▼" };
                    format!("{h}{arrow}")
                } else {
                    h.clone()
                };
                Cell::from(label).style(Style::default().fg(header_fg).bold())
            })
            .collect();
        let header = Row::new(header_cells).height(1);

        let status_col = self.headers.iter().position(|h| h == "STATUS");
        let rows: Vec<Row> = self
            .items
            .iter()
            .map(|item| {
                let cells: Vec<Cell> = item
                    .iter()
                    .enumerate()
                    .map(|(col_idx, val)| {
                        let style = if Some(col_idx) == status_col { status_style(val, t) } else { Style::default() };
                        Cell::from(val.as_str()).style(style)
                    })
                    .collect();
                Row::new(cells)
            })
            .collect();

        let widths: Vec<Constraint> = self
            .headers
            .iter()
            .enumerate()
            .map(|(i, h)| {
                if h == "PF" {
                    Constraint::Length(3)
                } else if i == 0 || (i == 1 && self.headers.first().is_some_and(|x| x == "PF")) {
                    Constraint::Min(20)
                } else {
                    Constraint::Min(12)
                }
            })
            .collect();

        let table = Table::new(rows, &widths).header(header).row_highlight_style(t.selection).highlight_symbol("▶ ");

        let mut table_state = ratatui::widgets::TableState::default().with_selected(self.selected);
        frame.render_stateful_widget(table, content_area, &mut table_state);

        if self.items.len() > content_area.height.saturating_sub(2) as usize {
            let mut scrollbar_state = ScrollbarState::new(self.items.len()).position(self.selected.unwrap_or(0));
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight).style(t.border);
            frame.render_stateful_widget(
                scrollbar,
                content_area.inner(Margin { vertical: 1, horizontal: 0 }),
                &mut scrollbar_state,
            );
        }
    }
}

fn status_style(status: &str, theme: &Theme) -> Style {
    match status {
        "Running" | "Succeeded" => theme.status_running,
        "Pending" | "ContainerCreating" => theme.status_pending,
        "Failed" | "Error" | "CrashLoopBackOff" | "ImagePullBackOff" => theme.status_failed,
        _ => theme.status_pending,
    }
}
