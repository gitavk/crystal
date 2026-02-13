use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table};

use crate::theme;

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
}

impl<'a> ResourceListWidget<'a> {
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let border_color = if self.focused { theme::ACCENT } else { theme::BORDER_COLOR };

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
            .title_style(Style::default().fg(theme::ACCENT).bold())
            .title_bottom(Line::from(count_display).right_aligned().style(Style::default().fg(theme::TEXT_DIM)));

        if self.loading {
            let msg = Paragraph::new("Loading...").style(Style::default().fg(theme::TEXT_DIM)).block(block);
            frame.render_widget(msg, area);
            return;
        }

        if let Some(err) = self.error {
            let msg =
                Paragraph::new(format!("Error: {err}")).style(Style::default().fg(theme::STATUS_FAILED)).block(block);
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
                Span::styled("Filter: ", Style::default().fg(theme::TEXT_DIM)),
                Span::styled(filter, Style::default().fg(theme::ACCENT)),
                Span::styled("_", Style::default().fg(theme::ACCENT)),
            ]);
            frame.render_widget(Paragraph::new(filter_line), filter_area);
        }

        if self.items.is_empty() && self.filter_text.is_none() {
            let msg = Paragraph::new("No resources found").style(Style::default().fg(theme::TEXT_DIM));
            frame.render_widget(msg, content_area);
            return;
        }

        if self.items.is_empty() {
            let msg = Paragraph::new("No matches").style(Style::default().fg(theme::TEXT_DIM));
            frame.render_widget(msg, content_area);
            return;
        }

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
                Cell::from(label).style(Style::default().fg(theme::TABLE_HEADER_FG).bold())
            })
            .collect();
        let header = Row::new(header_cells).height(1);

        let rows: Vec<Row> = self
            .items
            .iter()
            .map(|item| {
                let cells: Vec<Cell> = item
                    .iter()
                    .enumerate()
                    .map(|(col_idx, val)| {
                        let style =
                            if col_idx == 2 { Style::default().fg(status_color(val)) } else { Style::default() };
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
            .map(|(i, _)| if i == 0 { Constraint::Min(20) } else { Constraint::Min(12) })
            .collect();

        let table = Table::new(rows, &widths)
            .header(header)
            .row_highlight_style(Style::default().bg(theme::SELECTION_BG))
            .highlight_symbol("▶ ");

        let mut table_state = ratatui::widgets::TableState::default().with_selected(self.selected);
        frame.render_stateful_widget(table, content_area, &mut table_state);

        if self.items.len() > content_area.height.saturating_sub(2) as usize {
            let mut scrollbar_state = ScrollbarState::new(self.items.len()).position(self.selected.unwrap_or(0));
            let scrollbar =
                Scrollbar::new(ScrollbarOrientation::VerticalRight).style(Style::default().fg(theme::BORDER_COLOR));
            frame.render_stateful_widget(
                scrollbar,
                content_area.inner(Margin { vertical: 1, horizontal: 0 }),
                &mut scrollbar_state,
            );
        }
    }
}

fn status_color(status: &str) -> Color {
    match status {
        "Running" => theme::STATUS_RUNNING,
        "Succeeded" => theme::STATUS_RUNNING,
        "Pending" | "ContainerCreating" => theme::STATUS_PENDING,
        "Failed" | "Error" | "CrashLoopBackOff" | "ImagePullBackOff" => theme::STATUS_FAILED,
        _ => theme::STATUS_PENDING,
    }
}
