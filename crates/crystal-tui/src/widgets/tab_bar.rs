use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme::Theme;

pub struct TabBarWidget<'a> {
    pub tabs: &'a [String],
    pub active: usize,
    pub theme: &'a Theme,
}

impl<'a> TabBarWidget<'a> {
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let t = self.theme;
        let header_bg = t.header.bg.unwrap_or(Color::Reset);
        let sep = " │ ";
        let sep_w = sep.len();

        let labels: Vec<String> =
            self.tabs.iter().enumerate().map(|(i, name)| format!("[{}] {}", i + 1, name)).collect();
        let widths: Vec<usize> = labels.iter().map(|l| l.len()).collect();

        let scroll = self.compute_scroll(&widths, sep_w, area.width as usize);

        let mut spans = Vec::new();
        let mut first = true;
        for (i, label) in labels.iter().enumerate() {
            if i < scroll {
                continue;
            }
            if !first {
                spans.push(Span::styled(sep, t.border.bg(header_bg)));
            }
            first = false;

            let style = if i == self.active {
                Style::default().fg(t.accent).bg(header_bg).add_modifier(Modifier::BOLD)
            } else {
                t.text_dim.bg(header_bg)
            };
            spans.push(Span::styled(label.clone(), style));
        }

        let line = Line::from(spans);
        let bar = Paragraph::new(line).style(Style::default().bg(header_bg));
        frame.render_widget(bar, area);
    }

    fn compute_scroll(&self, widths: &[usize], sep_w: usize, max_w: usize) -> usize {
        if widths.is_empty() {
            return 0;
        }

        let total: usize = widths.iter().sum::<usize>() + sep_w * widths.len().saturating_sub(1);
        if total <= max_w {
            return 0;
        }

        let mut scroll = 0;
        loop {
            let visible: usize =
                widths[scroll..].iter().sum::<usize>() + sep_w * widths[scroll..].len().saturating_sub(1);
            if visible <= max_w {
                break;
            }
            if scroll >= self.active {
                break;
            }
            scroll += 1;
        }
        scroll
    }
}

#[cfg(test)]
mod tests;
