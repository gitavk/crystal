use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme;

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
        // Set to a large value; render will clamp it
        self.scroll_offset = usize::MAX;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    pub fn render(
        &mut self,
        lines: &[LogLineRef],
        frame: &mut Frame,
        area: Rect,
        focused: bool,
        status_text: Option<&str>,
    ) {
        if area.height < 3 || area.width < 10 {
            return;
        }

        self.total_lines = lines.len();

        // Title bar
        let title_style = if focused {
            Style::new().fg(theme::HEADER_FG).bg(theme::HEADER_BG)
        } else {
            Style::new().fg(theme::TEXT_DIM).bg(theme::HEADER_BG)
        };
        let title = self.title();
        let title_bar = Paragraph::new(Line::from(vec![Span::styled(&title, title_style)]))
            .style(Style::new().bg(theme::HEADER_BG));
        let title_area = Rect { x: area.x, y: area.y, width: area.width, height: 1 };
        frame.render_widget(title_bar, title_area);

        // Status bar at bottom
        let status_area = Rect { x: area.x, y: area.y + area.height - 1, width: area.width, height: 1 };

        let content_area = Rect { x: area.x, y: area.y + 1, width: area.width, height: area.height.saturating_sub(2) };

        // Filter lines
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

        // Clamp scroll_offset
        if self.auto_scroll {
            self.scroll_offset = 0;
        }
        let max_offset = visible_count.saturating_sub(view_height);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }

        // Determine which lines to show (from bottom)
        let end = visible_count.saturating_sub(self.scroll_offset);
        let start = end.saturating_sub(view_height);
        let visible_lines = &filtered[start..end];

        // Determine if we need container column (multi-container)
        let has_multi_containers = has_multiple_containers(lines);
        let ts_width: u16 = if self.show_timestamps { 24 } else { 0 };
        let ctr_width: u16 = if has_multi_containers { 16 } else { 0 };

        // Render lines
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
                spans.push(Span::styled(ts_text, Style::new().fg(theme::TEXT_DIM)));
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

            let content_style = if log_line.is_stderr {
                Style::new().fg(theme::STATUS_PENDING)
            } else {
                Style::new().fg(theme::HEADER_FG)
            };

            // Highlight filter matches
            if let Some(ref f) = self.filter {
                if !f.is_empty() {
                    spans.extend(highlight_matches(&content, f, content_style));
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

        // Status bar
        let mut status_parts = Vec::new();

        if self.auto_scroll {
            status_parts.push(Span::styled(" FOLLOW ", Style::new().fg(theme::STATUS_RUNNING).bold()));
        } else {
            status_parts.push(Span::styled(" PAUSED ", Style::new().fg(theme::STATUS_PENDING).bold()));
        }

        status_parts.push(Span::styled(" | ", Style::new().fg(theme::TEXT_DIM)));

        if let Some(ref f) = self.filter {
            if !f.is_empty() {
                let filter_info = format!("Filter: \"{}\"  ({}/{} lines)", f, visible_count, self.total_lines);
                status_parts.push(Span::styled(filter_info, Style::new().fg(theme::ACCENT)));
                status_parts.push(Span::styled(" | ", Style::new().fg(theme::TEXT_DIM)));
            }
        }

        let line_info = format!("{} lines", self.total_lines);
        status_parts.push(Span::styled(line_info, Style::new().fg(theme::STATUS_FG)));

        if let Some(st) = status_text {
            status_parts.push(Span::styled(" | ", Style::new().fg(theme::TEXT_DIM)));
            status_parts.push(Span::styled(st.to_string(), Style::new().fg(theme::STATUS_PENDING)));
        }

        let status_bar = Paragraph::new(Line::from(status_parts)).style(Style::new().bg(theme::STATUS_BG));
        frame.render_widget(status_bar, status_area);

        if !focused {
            dim_area(frame.buffer_mut(), content_area);
        }
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

fn highlight_matches<'a>(text: &'a str, query: &str, base_style: Style) -> Vec<Span<'a>> {
    let mut spans = Vec::new();
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut last = 0;

    for (idx, _) in lower_text.match_indices(&lower_query) {
        if idx > last {
            spans.push(Span::styled(&text[last..idx], base_style));
        }
        let highlight_style = base_style.bg(theme::SELECTION_BG).bold();
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

fn dim_area(buf: &mut Buffer, area: Rect) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_fg(theme::TEXT_DIM);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lines(n: usize) -> Vec<LogLineRef> {
        (0..n)
            .map(|i| LogLineRef {
                timestamp: Some(format!("2024-01-15T10:30:{:02}.000Z", i % 60)),
                content: format!("log line {i}"),
                container: "main".into(),
                is_stderr: false,
            })
            .collect()
    }

    fn make_multi_container_lines() -> Vec<LogLineRef> {
        vec![
            LogLineRef {
                timestamp: Some("2024-01-15T10:30:00.000Z".into()),
                content: "from main".into(),
                container: "main".into(),
                is_stderr: false,
            },
            LogLineRef {
                timestamp: Some("2024-01-15T10:30:01.000Z".into()),
                content: "from sidecar".into(),
                container: "sidecar".into(),
                is_stderr: false,
            },
        ]
    }

    #[test]
    fn title_format() {
        let view = LogsView::new(1, "my-pod".into(), "default".into());
        assert_eq!(view.title(), "[logs:my-pod @ default]");
    }

    #[test]
    fn toggle_follow() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        assert!(view.auto_scroll());
        view.toggle_follow();
        assert!(!view.auto_scroll());
        view.toggle_follow();
        assert!(view.auto_scroll());
    }

    #[test]
    fn toggle_timestamps() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        assert!(view.show_timestamps());
        view.toggle_timestamps();
        assert!(!view.show_timestamps());
    }

    #[test]
    fn toggle_wrap() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        assert!(!view.wrap_lines());
        view.toggle_wrap();
        assert!(view.wrap_lines());
    }

    #[test]
    fn filter_hides_non_matching_lines() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        let lines = vec![
            LogLineRef {
                timestamp: None,
                content: "error: something failed".into(),
                container: "main".into(),
                is_stderr: false,
            },
            LogLineRef {
                timestamp: None,
                content: "info: all good".into(),
                container: "main".into(),
                is_stderr: false,
            },
            LogLineRef {
                timestamp: None,
                content: "error: another failure".into(),
                container: "main".into(),
                is_stderr: false,
            },
        ];

        view.set_filter(Some("error".into()));

        let backend = ratatui::backend::TestBackend::new(80, 25);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.render(&lines, frame, Rect::new(0, 0, 80, 25), true, None);
            })
            .unwrap();

        // After rendering with filter, check the status bar contains filter info
        let buf = terminal.backend().buffer().clone();
        let status_row = 24u16;
        let mut status_text = String::new();
        for x in 0..80u16 {
            status_text.push_str(buf[(x, status_row)].symbol());
        }
        assert!(status_text.contains("2/3 lines"), "status should show filtered count: {status_text}");
    }

    #[test]
    fn auto_scroll_follows_new_lines_at_bottom() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        assert!(view.auto_scroll());
        assert_eq!(view.scroll_offset(), 0);

        let lines = make_lines(100);
        let backend = ratatui::backend::TestBackend::new(80, 25);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.render(&lines, frame, Rect::new(0, 0, 80, 25), true, None);
            })
            .unwrap();

        assert_eq!(view.scroll_offset(), 0);
        assert!(view.auto_scroll());
    }

    #[test]
    fn scroll_up_pauses_auto_scroll() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        assert!(view.auto_scroll());
        view.scroll_up(5);
        assert!(!view.auto_scroll());
        assert_eq!(view.scroll_offset(), 5);
    }

    #[test]
    fn scroll_down_to_bottom_resumes_auto_scroll() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        view.scroll_up(10);
        assert!(!view.auto_scroll());
        view.scroll_down(10);
        assert!(view.auto_scroll());
        assert_eq!(view.scroll_offset(), 0);
    }

    #[test]
    fn scroll_to_top_sets_max_offset() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        view.scroll_to_top();
        assert!(!view.auto_scroll());
        assert_eq!(view.scroll_offset(), usize::MAX);
    }

    #[test]
    fn scroll_to_bottom_resets() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        view.scroll_up(42);
        view.scroll_to_bottom();
        assert!(view.auto_scroll());
        assert_eq!(view.scroll_offset(), 0);
    }

    #[test]
    fn scrolling_clamps_to_bounds() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        view.scroll_up(1000);

        let lines = make_lines(10);
        let backend = ratatui::backend::TestBackend::new(80, 25);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.render(&lines, frame, Rect::new(0, 0, 80, 12), true, None);
            })
            .unwrap();

        // view_height = 12 - 2 = 10, lines = 10, max_offset = 0
        assert_eq!(view.scroll_offset(), 0);
    }

    #[test]
    fn timestamp_toggle_affects_rendering() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        let lines = make_lines(1);

        let backend = ratatui::backend::TestBackend::new(80, 5);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        // With timestamps
        view.show_timestamps = true;
        terminal
            .draw(|frame| {
                view.render(&lines, frame, Rect::new(0, 0, 80, 5), true, None);
            })
            .unwrap();
        let buf_with_ts = terminal.backend().buffer().clone();

        // Without timestamps
        view.show_timestamps = false;
        terminal
            .draw(|frame| {
                view.render(&lines, frame, Rect::new(0, 0, 80, 5), true, None);
            })
            .unwrap();
        let buf_without_ts = terminal.backend().buffer().clone();

        // Content position should differ
        let content_start_with_ts = find_text_position(&buf_with_ts, 1, "log line 0");
        let content_start_without_ts = find_text_position(&buf_without_ts, 1, "log line 0");
        assert!(content_start_with_ts.unwrap_or(0) > content_start_without_ts.unwrap_or(0));
    }

    #[test]
    fn container_filter_shows_only_selected() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        let lines = make_multi_container_lines();

        view.set_container_filter(Some("sidecar".into()));

        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.render(&lines, frame, Rect::new(0, 0, 80, 10), true, None);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let mut content = String::new();
        for y in 1..9u16 {
            for x in 0..80u16 {
                content.push_str(buf[(x, y)].symbol());
            }
        }
        assert!(content.contains("from sidecar"));
        assert!(!content.contains("from main"));
    }

    #[test]
    fn tiny_area_does_not_panic() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        let lines = make_lines(5);

        let backend = ratatui::backend::TestBackend::new(80, 25);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.render(&lines, frame, Rect::new(0, 0, 5, 2), true, None);
            })
            .unwrap();
    }

    #[test]
    fn unfocused_dims_content() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        let lines = make_lines(3);

        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.render(&lines, frame, Rect::new(0, 0, 80, 10), false, None);
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let cell = &buf[(0, 1)];
        assert_eq!(cell.fg, theme::TEXT_DIM);
    }

    #[test]
    fn accessors_return_correct_values() {
        let view = LogsView::new(42, "my-pod".into(), "kube-system".into());
        assert_eq!(view.stream_id(), 42);
        assert_eq!(view.pod_name(), "my-pod");
        assert_eq!(view.namespace(), "kube-system");
    }

    #[test]
    fn has_multiple_containers_detects_multi() {
        let single = make_lines(3);
        assert!(!has_multiple_containers(&single));

        let multi = make_multi_container_lines();
        assert!(has_multiple_containers(&multi));

        let empty: Vec<LogLineRef> = Vec::new();
        assert!(!has_multiple_containers(&empty));
    }

    #[test]
    fn highlight_matches_finds_occurrences() {
        let style = Style::new();
        let spans = highlight_matches("hello error world error", "error", style);
        assert!(spans.len() >= 4); // "hello " + "error" + " world " + "error"
    }

    #[test]
    fn container_color_is_deterministic() {
        let c1 = container_color("main");
        let c2 = container_color("main");
        assert_eq!(c1, c2);
    }

    #[test]
    fn container_color_varies_by_name() {
        let c1 = container_color("main");
        let c2 = container_color("sidecar");
        // Different names should usually produce different colors (not guaranteed but very likely)
        assert_ne!(c1, c2);
    }

    #[test]
    fn set_filter_resets_scroll() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        view.scroll_up(10);
        view.set_filter(Some("test".into()));
        assert_eq!(view.scroll_offset(), 0);
    }

    #[test]
    fn set_container_filter_resets_scroll() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        view.scroll_up(10);
        view.set_container_filter(Some("main".into()));
        assert_eq!(view.scroll_offset(), 0);
    }

    #[test]
    fn status_bar_shows_reconnecting() {
        let mut view = LogsView::new(1, "pod".into(), "ns".into());
        let lines = make_lines(5);

        let backend = ratatui::backend::TestBackend::new(80, 10);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                view.render(&lines, frame, Rect::new(0, 0, 80, 10), true, Some("Reconnecting..."));
            })
            .unwrap();

        let buf = terminal.backend().buffer().clone();
        let status_row = 9u16;
        let mut status = String::new();
        for x in 0..80u16 {
            status.push_str(buf[(x, status_row)].symbol());
        }
        assert!(status.contains("Reconnecting..."));
    }

    fn find_text_position(buf: &Buffer, row: u16, text: &str) -> Option<u16> {
        let mut row_text = String::new();
        for x in 0..buf.area.width {
            row_text.push_str(buf[(x, row)].symbol());
        }
        row_text.find(text).map(|p| p as u16)
    }
}
