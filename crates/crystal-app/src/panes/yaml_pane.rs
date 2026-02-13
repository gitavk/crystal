use std::any::Any;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crystal_tui::pane::{Pane, PaneCommand, ResourceKind, ViewType};
use crystal_tui::theme;

#[allow(dead_code)]
pub struct YamlPane {
    view_type: ViewType,
    resource_name: String,
    content: String,
    styled_lines: Vec<Line<'static>>,
    total_lines: usize,
    scroll_offset: usize,
    search_query: Option<String>,
    search_matches: Vec<usize>,
    current_match: usize,
    visible_height: u16,
}

#[allow(dead_code)]
impl YamlPane {
    pub fn new(kind: ResourceKind, name: String, yaml_content: String) -> Self {
        let styled_lines = Self::highlight_yaml(&yaml_content);
        let total_lines = styled_lines.len();
        Self {
            view_type: ViewType::Yaml(kind, name.clone()),
            resource_name: name,
            content: yaml_content,
            styled_lines,
            total_lines,
            scroll_offset: 0,
            search_query: None,
            search_matches: vec![],
            current_match: 0,
            visible_height: 0,
        }
    }

    pub fn highlight_yaml(content: &str) -> Vec<Line<'static>> {
        content
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let line_num = format!("{:>4} │ ", i + 1);
                let mut spans = vec![Span::styled(line_num, Style::default().fg(theme::TEXT_DIM))];

                let trimmed = line.trim_start();

                if trimmed.starts_with('#') {
                    spans.push(Span::styled(line.to_string(), Style::default().fg(theme::TEXT_DIM).italic()));
                } else if let Some((key_part, value_part)) = trimmed.split_once(':') {
                    let indent_len = line.len() - trimmed.len();
                    let indent = &line[..indent_len];

                    if !indent.is_empty() {
                        spans.push(Span::raw(indent.to_string()));
                    }

                    if let Some(stripped_key) = key_part.strip_prefix("- ") {
                        spans.push(Span::styled("- ", Style::default().fg(theme::TEXT_DIM)));
                        spans.push(Span::styled(stripped_key.to_string(), Style::default().fg(theme::ACCENT)));
                    } else {
                        spans.push(Span::styled(key_part.to_string(), Style::default().fg(theme::ACCENT)));
                    }

                    spans.push(Span::styled(":", Style::default().fg(theme::ACCENT)));

                    let value = value_part.trim();
                    if !value.is_empty() {
                        spans.push(Span::raw(" ".to_string()));
                        let value_style = Self::value_style(value);
                        spans.push(Span::styled(value.to_string(), value_style));
                    }
                } else if let Some(rest) = trimmed.strip_prefix("- ") {
                    let indent_len = line.len() - trimmed.len();
                    let indent = &line[..indent_len];
                    if !indent.is_empty() {
                        spans.push(Span::raw(indent.to_string()));
                    }
                    spans.push(Span::styled("- ", Style::default().fg(theme::TEXT_DIM)));
                    let value_style = Self::value_style(rest);
                    spans.push(Span::styled(rest.to_string(), value_style));
                } else if trimmed == "---" || trimmed == "..." {
                    spans.push(Span::styled(line.to_string(), Style::default().fg(theme::TEXT_DIM)));
                } else {
                    spans.push(Span::raw(line.to_string()));
                }

                Line::from(spans)
            })
            .collect()
    }

    fn value_style(value: &str) -> Style {
        let lower = value.to_lowercase();
        let is_keyword = lower == "true" || lower == "false" || lower == "null" || lower == "~";
        if is_keyword || value.parse::<f64>().is_ok() {
            Style::default().fg(theme::STATUS_RUNNING)
        } else {
            Style::default().fg(theme::HEADER_FG)
        }
    }

    fn update_search_matches(&mut self) {
        self.search_matches.clear();
        if let Some(query) = &self.search_query {
            if query.is_empty() {
                return;
            }
            let query_lower = query.to_lowercase();
            for (i, line) in self.content.lines().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    self.search_matches.push(i);
                }
            }
            self.current_match = 0;
        }
    }

    fn scroll_to_match(&mut self) {
        if let Some(&line_num) = self.search_matches.get(self.current_match) {
            let half_visible = self.visible_height as usize / 2;
            self.scroll_offset = line_num.saturating_sub(half_visible);
        }
    }

    fn prev_match(&mut self) {
        if !self.search_matches.is_empty() {
            if self.current_match == 0 {
                self.current_match = self.search_matches.len() - 1;
            } else {
                self.current_match -= 1;
            }
            self.scroll_to_match();
        }
    }
}

impl Pane for YamlPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_color = if focused { theme::ACCENT } else { theme::BORDER_COLOR };

        let title = format!(" YAML: {} ", self.resource_name);
        let line_count = format!(" {} lines ", self.total_lines);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(title)
            .title_style(Style::default().fg(theme::ACCENT).bold())
            .title(Line::styled(line_count, Style::default().fg(theme::TEXT_DIM)).alignment(Alignment::Right));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        // Reserve 1 line for search bar if search is active
        let has_search = self.search_query.is_some();
        let content_height = if has_search { inner.height.saturating_sub(1) } else { inner.height };
        let content_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: content_height };

        // Build display lines with search highlighting
        let max_scroll = self.total_lines.saturating_sub(content_height as usize);
        let scroll = self.scroll_offset.min(max_scroll);

        let display_lines: Vec<Line> = self
            .styled_lines
            .iter()
            .enumerate()
            .skip(scroll)
            .take(content_height as usize)
            .map(|(line_idx, line)| {
                let is_match = self.search_matches.contains(&line_idx);
                let is_current = self.search_matches.get(self.current_match).is_some_and(|&m| m == line_idx);
                if is_current {
                    line.clone().style(Style::default().bg(theme::SELECTION_BG))
                } else if is_match {
                    line.clone().style(Style::default().bg(Color::Rgb(49, 50, 68)))
                } else {
                    line.clone()
                }
            })
            .collect();

        let paragraph = Paragraph::new(display_lines);
        frame.render_widget(paragraph, content_area);

        // Scrollbar
        if self.total_lines > content_height as usize {
            let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                content_area,
                &mut scrollbar_state,
            );
        }

        // Search bar
        if let Some(query) = &self.search_query {
            let search_area = Rect { x: inner.x, y: inner.y + content_height, width: inner.width, height: 1 };
            let match_info = if self.search_matches.is_empty() {
                "no matches".to_string()
            } else {
                format!("[{}/{}]", self.current_match + 1, self.search_matches.len())
            };
            let search_line = Line::from(vec![
                Span::styled("/", Style::default().fg(theme::ACCENT)),
                Span::styled(query.clone(), Style::default().fg(theme::HEADER_FG)),
                Span::raw(" ".repeat(
                    search_area.width.saturating_sub(query.len() as u16 + 1 + match_info.len() as u16) as usize,
                )),
                Span::styled(match_info, Style::default().fg(theme::TEXT_DIM)),
            ]);
            frame.render_widget(Paragraph::new(vec![search_line]), search_area);
        }
    }

    fn handle_command(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::ScrollUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            PaneCommand::ScrollDown => {
                if self.scroll_offset < self.total_lines.saturating_sub(1) {
                    self.scroll_offset += 1;
                }
            }
            PaneCommand::SearchInput(ch) => {
                self.search_query.get_or_insert_with(String::new).push(*ch);
                self.update_search_matches();
            }
            PaneCommand::SearchConfirm => {
                if !self.search_matches.is_empty() {
                    self.current_match = (self.current_match + 1) % self.search_matches.len();
                    self.scroll_to_match();
                }
            }
            PaneCommand::SearchClear => {
                self.search_query = None;
                self.search_matches.clear();
                self.current_match = 0;
            }
            PaneCommand::SelectPrev => {
                self.prev_match();
            }
            _ => {}
        }
    }

    fn view_type(&self) -> &ViewType {
        &self.view_type
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_YAML: &str = "\
apiVersion: v1
kind: Pod
metadata:
  name: nginx-abc123
  namespace: default
  labels:
    app: nginx
spec:
  containers:
    - name: nginx
      image: nginx:1.25
      ports:
        - containerPort: 80
status:
  phase: Running
  # this is a comment
  ready: true
  restartCount: 0";

    #[test]
    fn highlight_yaml_keys_get_accent_style() {
        let lines = YamlPane::highlight_yaml(SAMPLE_YAML);
        assert!(!lines.is_empty());
        // First line: "apiVersion: v1" - key should be ACCENT
        let first_line = &lines[0];
        let has_accent_key = first_line
            .spans
            .iter()
            .any(|span| span.content.contains("apiVersion") && span.style.fg == Some(theme::ACCENT));
        assert!(has_accent_key, "Key should be styled with ACCENT color");
    }

    #[test]
    fn highlight_yaml_comments_get_dim_italic() {
        let lines = YamlPane::highlight_yaml(SAMPLE_YAML);
        let comment_line = lines.iter().find(|l| l.spans.iter().any(|s| s.content.contains("# this is a comment")));
        assert!(comment_line.is_some(), "Comment line should exist");
        let comment_span = comment_line.unwrap().spans.iter().find(|s| s.content.contains("#")).unwrap();
        assert_eq!(comment_span.style.fg, Some(theme::TEXT_DIM));
        assert!(comment_span.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn highlight_yaml_booleans_get_status_running_color() {
        let lines = YamlPane::highlight_yaml("enabled: true");
        let has_green_true =
            lines[0].spans.iter().any(|s| s.content == "true" && s.style.fg == Some(theme::STATUS_RUNNING));
        assert!(has_green_true, "Boolean 'true' should be STATUS_RUNNING color");
    }

    #[test]
    fn highlight_yaml_numbers_get_status_running_color() {
        let lines = YamlPane::highlight_yaml("replicas: 3");
        let has_green_num =
            lines[0].spans.iter().any(|s| s.content == "3" && s.style.fg == Some(theme::STATUS_RUNNING));
        assert!(has_green_num, "Number should be STATUS_RUNNING color");
    }

    #[test]
    fn highlight_yaml_line_numbers_present() {
        let lines = YamlPane::highlight_yaml("key: value\nkey2: value2");
        assert_eq!(lines.len(), 2);
        assert!(lines[0].spans[0].content.contains("1"));
        assert!(lines[1].spans[0].content.contains("2"));
    }

    #[test]
    fn search_finds_correct_lines() {
        let mut pane = YamlPane::new(ResourceKind::Pods, "test".into(), SAMPLE_YAML.into());
        pane.visible_height = 20;
        for ch in "nginx".chars() {
            pane.handle_command(&PaneCommand::SearchInput(ch));
        }
        assert!(pane.search_matches.len() >= 2, "Should find 'nginx' on multiple lines");
        // Lines with nginx: name: nginx-abc123, app: nginx, - name: nginx, image: nginx:1.25
        for &line_idx in &pane.search_matches {
            let line_content: String = SAMPLE_YAML.lines().nth(line_idx).unwrap().to_string();
            assert!(
                line_content.to_lowercase().contains("nginx"),
                "Matched line should contain 'nginx': {line_content}"
            );
        }
    }

    #[test]
    fn search_next_wraps_around() {
        let mut pane = YamlPane::new(ResourceKind::Pods, "test".into(), SAMPLE_YAML.into());
        pane.visible_height = 20;
        for ch in "nginx".chars() {
            pane.handle_command(&PaneCommand::SearchInput(ch));
        }
        let match_count = pane.search_matches.len();
        assert!(match_count >= 2);

        // Navigate to end and wrap
        for _ in 0..match_count {
            pane.handle_command(&PaneCommand::SearchConfirm);
        }
        // After match_count confirms starting from 0, we should wrap back to 0
        // SearchConfirm increments first, so after match_count confirms we're at match_count % match_count = 0
        assert_eq!(pane.current_match, 0);
    }

    #[test]
    fn search_clear_resets_state() {
        let mut pane = YamlPane::new(ResourceKind::Pods, "test".into(), SAMPLE_YAML.into());
        for ch in "nginx".chars() {
            pane.handle_command(&PaneCommand::SearchInput(ch));
        }
        assert!(!pane.search_matches.is_empty());
        pane.handle_command(&PaneCommand::SearchClear);
        assert!(pane.search_query.is_none());
        assert!(pane.search_matches.is_empty());
    }

    #[test]
    fn scroll_clamps_to_bounds() {
        let mut pane = YamlPane::new(ResourceKind::Pods, "test".into(), "a: 1\nb: 2".into());
        // Scroll down many times
        for _ in 0..100 {
            pane.handle_command(&PaneCommand::ScrollDown);
        }
        // scroll_offset should be clamped to total_lines - 1
        assert!(pane.scroll_offset <= pane.total_lines);

        // Scroll up many times
        for _ in 0..200 {
            pane.handle_command(&PaneCommand::ScrollUp);
        }
        assert_eq!(pane.scroll_offset, 0);
    }

    #[test]
    fn view_type_is_yaml() {
        let pane = YamlPane::new(ResourceKind::Pods, "test".into(), "".into());
        assert_eq!(*pane.view_type(), ViewType::Yaml(ResourceKind::Pods, "test".into()));
    }

    #[test]
    fn list_markers_styled_dim() {
        let lines = YamlPane::highlight_yaml("  - item1");
        let has_dim_marker = lines[0].spans.iter().any(|s| s.content == "- " && s.style.fg == Some(theme::TEXT_DIM));
        assert!(has_dim_marker, "List marker '- ' should be TEXT_DIM");
    }
}
