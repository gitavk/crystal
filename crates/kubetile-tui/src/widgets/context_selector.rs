use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use crate::theme::Theme;

pub struct ContextSelectorWidget<'a> {
    pub contexts: &'a [String],
    pub filter: &'a str,
    pub selected: usize,
    pub theme: &'a Theme,
}

impl<'a> ContextSelectorWidget<'a> {
    pub fn filtered_contexts(&self) -> Vec<&'a str> {
        let filter_lower = self.filter.to_lowercase();
        self.contexts
            .iter()
            .filter(|ctx| filter_lower.is_empty() || ctx.to_lowercase().contains(&filter_lower))
            .map(|ctx| ctx.as_str())
            .collect()
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
            .title(" Select Context ")
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

        let filtered = self.filtered_contexts();
        let items: Vec<ListItem> =
            filtered.iter().map(|ctx| ListItem::new(format!("  {ctx}")).style(Style::default().fg(t.fg))).collect();

        let list = List::new(items).highlight_style(t.selection.add_modifier(Modifier::BOLD));
        let mut list_state =
            ListState::default().with_selected(Some(self.selected.min(filtered.len().saturating_sub(1))));
        frame.render_stateful_widget(list, chunks[1], &mut list_state);

        let hints = Paragraph::new(" Enter:select  Esc:cancel").style(t.text_dim);
        frame.render_widget(hints, chunks[2]);
    }
}
