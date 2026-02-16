use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use crate::theme::Theme;

pub struct NamespaceSelectorWidget<'a> {
    pub namespaces: &'a [String],
    pub filter: &'a str,
    pub selected: usize,
    pub theme: &'a Theme,
}

impl<'a> NamespaceSelectorWidget<'a> {
    pub fn filtered_namespaces(&self) -> Vec<&'a str> {
        let mut result: Vec<&str> = Vec::new();
        let filter_lower = self.filter.to_lowercase();

        if filter_lower.is_empty() || "all namespaces".contains(&filter_lower) {
            result.push("All Namespaces");
        }

        for ns in self.namespaces {
            if filter_lower.is_empty() || ns.to_lowercase().contains(&filter_lower) {
                result.push(ns);
            }
        }

        result
    }

    pub fn render(self, frame: &mut Frame, area: Rect) {
        let t = self.theme;
        let popup_width = area.width / 2;
        let popup_height = area.height * 3 / 5;
        let popup_area = Rect {
            x: area.x + (area.width.saturating_sub(popup_width)) / 2,
            y: area.y + (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width.min(60),
            height: popup_height.min(30),
        };

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent))
            .title(" Select Namespace ")
            .title_style(Style::default().fg(t.accent).bold())
            .style(t.overlay);

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        let filter_display =
            if self.filter.is_empty() { "Type to filter...".to_string() } else { self.filter.to_string() };
        let filter_style = if self.filter.is_empty() { t.text_dim } else { Style::default().fg(t.fg) };
        let filter_line = Paragraph::new(format!(" > {filter_display}")).style(filter_style);
        frame.render_widget(filter_line, chunks[0]);

        let filtered = self.filtered_namespaces();
        let items: Vec<ListItem> = filtered
            .iter()
            .map(|ns| {
                let style = if *ns == "All Namespaces" {
                    Style::default().fg(t.accent).italic()
                } else {
                    Style::default().fg(t.fg)
                };
                ListItem::new(format!("  {ns}")).style(style)
            })
            .collect();

        let list = List::new(items).highlight_style(t.selection.add_modifier(Modifier::BOLD));

        let mut list_state =
            ListState::default().with_selected(Some(self.selected.min(filtered.len().saturating_sub(1))));
        frame.render_stateful_widget(list, chunks[1], &mut list_state);

        let hints = Paragraph::new(" Enter:select  Esc:cancel").style(t.text_dim);
        frame.render_widget(hints, chunks[2]);
    }
}
