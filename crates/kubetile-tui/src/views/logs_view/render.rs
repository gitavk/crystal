use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme::Theme;

use super::{
    container_color, dim_area, has_multiple_containers, highlight_matches, truncate_str, LogLineRef, LogsView,
};

impl LogsView {
    pub fn render(
        &mut self,
        lines: &[LogLineRef],
        frame: &mut Frame,
        area: Rect,
        focused: bool,
        status_text: Option<&str>,
        theme: &Theme,
    ) {
        if area.height < 3 || area.width < 10 {
            return;
        }

        let t = theme;
        let header_bg = t.header.bg.unwrap_or(Color::Reset);
        let status_bg = t.status_bar.bg.unwrap_or(Color::Reset);
        let status_fg = t.status_bar.fg.unwrap_or(Color::Reset);
        let text_dim_color = t.text_dim.fg.unwrap_or(Color::Reset);

        self.total_lines = lines.len();

        let title_style = if focused { Style::new().fg(t.fg).bg(header_bg) } else { t.text_dim.bg(header_bg) };
        let title = self.title();
        let title_bar =
            Paragraph::new(Line::from(vec![Span::styled(&title, title_style)])).style(Style::new().bg(header_bg));
        let title_area = Rect { x: area.x, y: area.y, width: area.width, height: 1 };
        frame.render_widget(title_bar, title_area);

        let status_area = Rect { x: area.x, y: area.y + area.height - 1, width: area.width, height: 1 };
        let content_area = Rect { x: area.x, y: area.y + 1, width: area.width, height: area.height.saturating_sub(2) };

        let filtered: Vec<&LogLineRef> = lines
            .iter()
            .filter(|l| {
                if let Some(ref cf) = self.container_filter {
                    if l.container != *cf {
                        return false;
                    }
                }
                if let Some(ref f) = self.filter {
                    if !f.is_empty() {
                        return l.content.to_lowercase().contains(&f.to_lowercase());
                    }
                }
                true
            })
            .collect();

        let visible_count = filtered.len();
        let view_height = content_area.height as usize;

        if self.auto_scroll {
            self.scroll_offset = 0;
        }
        let max_offset = visible_count.saturating_sub(view_height);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }

        let end = visible_count.saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(view_height);
        let visible_lines = &filtered[start..end];

        let has_multi_containers = has_multiple_containers(lines);
        let ts_width: u16 = if self.show_timestamps { 24 } else { 0 };
        let ctr_width: u16 = if has_multi_containers { 16 } else { 0 };

        for (i, log_line) in visible_lines.iter().enumerate() {
            let y = content_area.y + i as u16;
            if y >= content_area.y + content_area.height {
                break;
            }

            let mut spans = Vec::new();
            let mut col_used: u16 = 0;

            if self.show_timestamps {
                let ts_text = match &log_line.timestamp {
                    Some(ts) => format!("{:<23} ", truncate_str(ts, 23)),
                    None => " ".repeat(24),
                };
                spans.push(Span::styled(ts_text, t.text_dim));
                col_used += ts_width;
            }

            if has_multi_containers {
                let ctr_text = format!("{:<15} ", truncate_str(&log_line.container, 15));
                let ctr_color = container_color(&log_line.container);
                spans.push(Span::styled(ctr_text, Style::new().fg(ctr_color)));
                col_used += ctr_width;
            }

            let content_width = (content_area.width.saturating_sub(col_used)) as usize;
            let content = if self.wrap_lines || log_line.content.len() <= content_width {
                log_line.content.clone()
            } else {
                log_line.content[..content_width].to_string()
            };

            let content_style = if log_line.is_stderr { t.status_pending } else { Style::new().fg(t.fg) };

            if let Some(ref f) = self.filter {
                if !f.is_empty() {
                    spans.extend(highlight_matches(&content, f, content_style, t));
                } else {
                    spans.push(Span::styled(content, content_style));
                }
            } else {
                spans.push(Span::styled(content, content_style));
            }

            let line_widget = Paragraph::new(Line::from(spans));
            let line_area = Rect { x: content_area.x, y, width: content_area.width, height: 1 };
            frame.render_widget(line_widget, line_area);
        }

        let mut status_parts = Vec::new();

        if self.auto_scroll {
            status_parts.push(Span::styled(" FOLLOW ", t.status_running.add_modifier(Modifier::BOLD)));
        } else {
            status_parts.push(Span::styled(" PAUSED ", t.status_pending.add_modifier(Modifier::BOLD)));
        }

        status_parts.push(Span::styled(" | ", t.text_dim));

        if let Some(ref f) = self.filter {
            if !f.is_empty() {
                let filter_info = format!("Filter: \"{}\"  ({}/{} lines)", f, visible_count, self.total_lines);
                status_parts.push(Span::styled(filter_info, Style::new().fg(t.accent)));
                status_parts.push(Span::styled(" | ", t.text_dim));
            }
        }

        let line_info = format!("{} lines", self.total_lines);
        status_parts.push(Span::styled(line_info, Style::new().fg(status_fg)));

        if let Some(st) = status_text {
            status_parts.push(Span::styled(" | ", t.text_dim));
            status_parts.push(Span::styled(st.to_string(), t.status_pending));
        }

        let status_bar = Paragraph::new(Line::from(status_parts)).style(Style::new().bg(status_bg));
        frame.render_widget(status_bar, status_area);

        if !focused {
            dim_area(frame.buffer_mut(), content_area, text_dim_color);
        }
    }
}
