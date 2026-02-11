use std::collections::HashMap;

use ratatui::prelude::{Frame, Rect};

use crate::pane::ViewType;

type RenderFn = fn(frame: &mut Frame, area: Rect, focused: bool);

/// Maps ViewType discriminant keys to render functions.
///
/// This indirection exists so that new view types (plugins, terminal)
/// can be added without modifying the render loop. Panes that implement
/// the Pane trait handle their own rendering; the registry provides
/// fallback renderers for pane types that haven't been instantiated yet.
pub struct ViewRegistry {
    renderers: HashMap<&'static str, RenderFn>,
}

impl ViewRegistry {
    pub fn new() -> Self {
        let mut renderers = HashMap::new();
        renderers.insert("Empty", render_empty as RenderFn);
        renderers.insert("Help", render_help_placeholder as RenderFn);
        Self { renderers }
    }

    pub fn register(&mut self, key: &'static str, render_fn: RenderFn) {
        self.renderers.insert(key, render_fn);
    }

    pub fn get(&self, view_type: &ViewType) -> Option<&RenderFn> {
        let key = view_type_key(view_type);
        self.renderers.get(key)
    }

    pub fn render_fallback(&self, view_type: &ViewType, frame: &mut Frame, area: Rect, focused: bool) {
        if let Some(render_fn) = self.get(view_type) {
            render_fn(frame, area, focused);
        } else {
            render_unknown(frame, area, focused);
        }
    }
}

impl Default for ViewRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn view_type_key(view_type: &ViewType) -> &'static str {
    match view_type {
        ViewType::ResourceList(_) => "ResourceList",
        ViewType::Detail(_, _) => "Detail",
        ViewType::Terminal => "Terminal",
        ViewType::Logs(_) => "Logs",
        ViewType::Exec(_) => "Exec",
        ViewType::Help => "Help",
        ViewType::Empty => "Empty",
        ViewType::Plugin(_) => "Plugin",
    }
}

fn render_empty(frame: &mut Frame, area: Rect, focused: bool) {
    use ratatui::prelude::*;
    use ratatui::widgets::{Block, Borders, Paragraph};

    let border_color = if focused { crate::theme::ACCENT } else { crate::theme::BORDER_COLOR };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Empty ")
        .title_style(Style::default().fg(crate::theme::TEXT_DIM));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let msg = Paragraph::new("Empty pane").style(Style::default().fg(crate::theme::TEXT_DIM));
    frame.render_widget(msg, inner);
}

fn render_help_placeholder(frame: &mut Frame, area: Rect, focused: bool) {
    use ratatui::prelude::*;
    use ratatui::widgets::{Block, Borders, Paragraph};

    let border_color = if focused { crate::theme::ACCENT } else { crate::theme::BORDER_COLOR };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Help ")
        .title_style(Style::default().fg(crate::theme::ACCENT).bold());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let msg = Paragraph::new("Press ? for help").style(Style::default().fg(crate::theme::TEXT_DIM));
    frame.render_widget(msg, inner);
}

fn render_unknown(frame: &mut Frame, area: Rect, focused: bool) {
    use ratatui::prelude::*;
    use ratatui::widgets::{Block, Borders, Paragraph};

    let border_color = if focused { crate::theme::ACCENT } else { crate::theme::BORDER_COLOR };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Unknown ")
        .title_style(Style::default().fg(crate::theme::TEXT_DIM));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let msg = Paragraph::new("Unknown view type").style(Style::default().fg(crate::theme::TEXT_DIM));
    frame.render_widget(msg, inner);
}

#[cfg(test)]
mod tests;
