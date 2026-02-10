use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table};

use crate::theme;

pub struct ResourceListWidget<'a> {
    pub title: &'a str,
    pub headers: &'a [String],
    pub items: &'a [Vec<String>],
    pub selected: Option<usize>,
    pub scroll_offset: usize,
    pub loading: bool,
    pub error: Option<&'a str>,
    pub focused: bool,
}

impl<'a> ResourceListWidget<'a> {
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let border_color = if self.focused { theme::ACCENT } else { theme::BORDER_COLOR };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(format!(" {} ", self.title))
            .title_style(Style::default().fg(theme::ACCENT).bold());

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

        if self.items.is_empty() {
            let msg = Paragraph::new("No resources found").style(Style::default().fg(theme::TEXT_DIM)).block(block);
            frame.render_widget(msg, area);
            return;
        }

        let header_cells: Vec<Cell> = self
            .headers
            .iter()
            .map(|h| Cell::from(h.as_str()).style(Style::default().fg(theme::TABLE_HEADER_FG).bold()))
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
            .block(block)
            .row_highlight_style(Style::default().bg(theme::SELECTION_BG))
            .highlight_symbol("▶ ");

        let mut table_state = ratatui::widgets::TableState::default().with_selected(self.selected);
        frame.render_stateful_widget(table, area, &mut table_state);

        if self.items.len() > area.height.saturating_sub(3) as usize {
            let mut scrollbar_state = ScrollbarState::new(self.items.len()).position(self.selected.unwrap_or(0));
            let scrollbar =
                Scrollbar::new(ScrollbarOrientation::VerticalRight).style(Style::default().fg(theme::BORDER_COLOR));
            frame.render_stateful_widget(
                scrollbar,
                area.inner(Margin { vertical: 1, horizontal: 0 }),
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
