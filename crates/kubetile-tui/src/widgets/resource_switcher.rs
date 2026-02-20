use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use crate::pane::ResourceKind;
use crate::theme::Theme;

pub struct ResourceSwitcherWidget<'a> {
    pub input: &'a str,
    pub items: &'a [ResourceKind],
    pub selected: usize,
    pub theme: &'a Theme,
}

impl<'a> ResourceSwitcherWidget<'a> {
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let t = self.theme;
        let overlay_bg = t.overlay.bg.unwrap_or(Color::Reset);
        let width: u16 = 40.min(area.width.saturating_sub(4));
        let height: u16 = ((self.items.len() + 3) as u16).min(20).min(area.height.saturating_sub(2));
        let popup = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };

        frame.render_widget(Clear, popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.accent))
            .title(" Switch Resource ")
            .title_style(Style::default().fg(t.accent).bold())
            .style(t.overlay);

        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(inner);

        let input_display = format!(":{}_", self.input);
        let input_line = Paragraph::new(input_display).style(Style::default().fg(t.fg).bg(overlay_bg));
        frame.render_widget(input_line, chunks[0]);

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, kind)| {
                let marker = if i == self.selected { "> " } else { "  " };
                let short = kind.short_name();
                let display = kind.display_name();
                let text = format!("{marker}{short:<8} {display}");
                let style =
                    if i == self.selected { Style::default().fg(t.accent).bold() } else { Style::default().fg(t.fg) };
                ListItem::new(text).style(style)
            })
            .collect();

        let list = List::new(items).highlight_style(t.selection.add_modifier(Modifier::BOLD));

        let mut list_state =
            ListState::default().with_selected(Some(self.selected.min(self.items.len().saturating_sub(1))));
        frame.render_stateful_widget(list, chunks[1], &mut list_state);
    }
}
