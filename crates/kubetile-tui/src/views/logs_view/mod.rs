use ratatui::prelude::*;

use crate::theme::Theme;

mod render;

#[derive(Debug, Clone)]
pub struct LogLineRef {
    pub timestamp: Option<String>,
    pub content: String,
    pub container: String,
    pub is_stderr: bool,
}

pub struct LogsView {
    scroll_offset: usize,
    auto_scroll: bool,
    filter: Option<String>,
    show_timestamps: bool,
    wrap_lines: bool,
    container_filter: Option<String>,
    pod_name: String,
    namespace: String,
    total_lines: usize,
    stream_id: u64,
}

impl LogsView {
    pub fn new(stream_id: u64, pod_name: String, namespace: String) -> Self {
        Self {
            scroll_offset: 0,
            auto_scroll: true,
            filter: None,
            show_timestamps: true,
            wrap_lines: false,
            container_filter: None,
            pod_name,
            namespace,
            total_lines: 0,
            stream_id,
        }
    }

    pub fn stream_id(&self) -> u64 {
        self.stream_id
    }

    pub fn pod_name(&self) -> &str {
        &self.pod_name
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn auto_scroll(&self) -> bool {
        self.auto_scroll
    }

    pub fn filter(&self) -> Option<&str> {
        self.filter.as_deref()
    }

    pub fn show_timestamps(&self) -> bool {
        self.show_timestamps
    }

    pub fn wrap_lines(&self) -> bool {
        self.wrap_lines
    }

    pub fn container_filter(&self) -> Option<&str> {
        self.container_filter.as_deref()
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn title(&self) -> String {
        format!("[logs:{} @ {}]", self.pod_name, self.namespace)
    }

    pub fn toggle_follow(&mut self) {
        self.auto_scroll = !self.auto_scroll;
    }

    pub fn toggle_timestamps(&mut self) {
        self.show_timestamps = !self.show_timestamps;
    }

    pub fn toggle_wrap(&mut self) {
        self.wrap_lines = !self.wrap_lines;
    }

    pub fn set_filter(&mut self, filter: Option<String>) {
        self.filter = filter;
        self.scroll_offset = 0;
    }

    pub fn set_container_filter(&mut self, container: Option<String>) {
        self.container_filter = container;
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(lines);
        self.auto_scroll = false;
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        if self.scroll_offset == 0 {
            self.auto_scroll = true;
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = usize::MAX;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }
}

fn has_multiple_containers(lines: &[LogLineRef]) -> bool {
    if lines.is_empty() {
        return false;
    }
    let first = &lines[0].container;
    lines.iter().any(|l| l.container != *first)
}

fn container_color(container: &str) -> Color {
    let hash: u32 = container.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let colors = [
        Color::Rgb(137, 180, 250), // blue
        Color::Rgb(166, 227, 161), // green
        Color::Rgb(249, 226, 175), // yellow
        Color::Rgb(203, 166, 247), // mauve
        Color::Rgb(148, 226, 213), // teal
        Color::Rgb(250, 179, 135), // peach
    ];
    colors[(hash as usize) % colors.len()]
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        s[..max].to_string()
    }
}

fn highlight_matches<'a>(text: &'a str, query: &str, base_style: Style, theme: &Theme) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut last = 0;

    for (idx, _) in lower_text.match_indices(&lower_query) {
        if idx > last {
            spans.push(Span::styled(&text[last..idx], base_style));
        }
        let highlight_style = base_style.patch(theme.selection).add_modifier(Modifier::BOLD);
        spans.push(Span::styled(&text[idx..idx + query.len()], highlight_style));
        last = idx + query.len();
    }

    if last < text.len() {
        spans.push(Span::styled(&text[last..], base_style));
    }

    if spans.is_empty() {
        spans.push(Span::styled(text, base_style));
    }

    spans
}

fn dim_area(buf: &mut Buffer, area: Rect, dim_color: Color) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_fg(dim_color);
            }
        }
    }
}

#[cfg(test)]
mod tests;
