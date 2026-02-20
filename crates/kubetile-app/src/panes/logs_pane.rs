use std::any::Any;
use std::cell::Cell;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crystal_tui::pane::{Pane, PaneCommand, ViewType};
use crystal_tui::theme::Theme;
use kubetile_core::{LogLine, LogStream, StreamStatus};

const MAX_LOG_LINES: usize = 5000;

#[derive(Clone)]
struct LogEntry {
    rendered: String,
    sort_ts: jiff::Timestamp,
    sequence: u64,
}

pub struct LogsPane {
    view_type: ViewType,
    pod_name: String,
    namespace: String,
    lines: Vec<LogEntry>,
    next_sequence: u64,
    scroll_offset: usize,
    horizontal_offset: usize,
    follow: bool,
    wrap: bool,
    filter_text: String,
    status: String,
    stream: Option<LogStream>,
    max_scroll_offset: Cell<usize>,
    max_horizontal_offset: Cell<usize>,
}

impl LogsPane {
    pub fn new(pod_name: String, namespace: String) -> Self {
        Self {
            view_type: ViewType::Logs(pod_name.clone()),
            pod_name,
            namespace,
            lines: Vec::new(),
            next_sequence: 0,
            scroll_offset: 0,
            horizontal_offset: 0,
            follow: true,
            wrap: true,
            filter_text: String::new(),
            status: "Connecting...".into(),
            stream: None,
            max_scroll_offset: Cell::new(0),
            max_horizontal_offset: Cell::new(0),
        }
    }

    pub fn attach_stream(&mut self, stream: LogStream) {
        self.stream = Some(stream);
        self.status = "Streaming".into();
    }

    pub fn append_snapshot(&mut self, lines: Vec<LogLine>) {
        self.push_lines(lines);
        if self.status == "Connecting..." {
            self.status = "Snapshot loaded".into();
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.stream = None;
        self.status = format!("Error: {error}");
    }

    pub fn pod_name(&self) -> &str {
        &self.pod_name
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn filter_text(&self) -> Option<&str> {
        if self.filter_text.is_empty() {
            None
        } else {
            Some(self.filter_text.as_str())
        }
    }

    pub fn export_filtered_history(&self) -> Vec<String> {
        self.filtered_lines().into_iter().map(|line| line.rendered.clone()).collect()
    }

    pub fn poll(&mut self) {
        let (new_lines, stream_status) = {
            let Some(stream) = self.stream.as_mut() else {
                return;
            };
            let new_lines = stream.next_lines();
            let stream_status = stream.status();
            (new_lines, stream_status)
        };

        if !new_lines.is_empty() {
            self.push_lines(new_lines);
        }

        self.status = match stream_status {
            StreamStatus::Streaming => "Streaming".into(),
            StreamStatus::Reconnecting { attempt } => format!("Reconnecting ({attempt})"),
            StreamStatus::Stopped => "Stopped".into(),
            StreamStatus::Error => "Error".into(),
        };
    }

    fn render_title(&self) -> String {
        format!("[logs:{} @ {}]", self.pod_name, self.namespace)
    }

    fn push_lines(&mut self, lines: Vec<LogLine>) {
        if lines.is_empty() {
            return;
        }

        for line in lines {
            let sequence = self.next_sequence;
            self.next_sequence = self.next_sequence.saturating_add(1);
            self.lines.push(LogEntry {
                rendered: format_log_line(&line),
                sort_ts: line.timestamp.unwrap_or_else(jiff::Timestamp::now),
                sequence,
            });
        }

        self.lines.sort_by(|a, b| a.sort_ts.cmp(&b.sort_ts).then_with(|| a.sequence.cmp(&b.sequence)));

        if self.lines.len() > MAX_LOG_LINES {
            let drop_count = self.lines.len().saturating_sub(MAX_LOG_LINES);
            self.lines.drain(0..drop_count);
        }
    }

    fn filtered_lines(&self) -> Vec<&LogEntry> {
        if self.filter_text.is_empty() {
            return self.lines.iter().collect();
        }

        let query = self.filter_text.to_lowercase();
        self.lines.iter().filter(|line| line.rendered.to_lowercase().contains(&query)).collect()
    }
}

impl Pane for LogsPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme) {
        let border_style = if focused { theme.border_active } else { theme.border };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", self.render_title()))
            .title_style(Style::default().fg(theme.accent).bold());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height == 0 {
            return;
        }

        let filtered = self.filtered_lines();
        let visible_height = inner.height.saturating_sub(1) as usize;
        let total = self.lines.len();
        let filtered_total = filtered.len();
        let max_offset = filtered_total.saturating_sub(visible_height);
        self.max_scroll_offset.set(max_offset);
        let offset = if self.follow { 0 } else { self.scroll_offset.min(max_offset) };
        let end = filtered_total.saturating_sub(offset);
        let start = end.saturating_sub(visible_height);
        let visible = &filtered[start..end];
        let viewport_width = inner.width as usize;
        let max_horizontal =
            visible.iter().map(|line| line.rendered.chars().count().saturating_sub(viewport_width)).max().unwrap_or(0);
        self.max_horizontal_offset.set(max_horizontal);
        let horizontal_offset = if self.wrap { 0 } else { self.horizontal_offset.min(max_horizontal) };

        let content = if visible.is_empty() {
            vec![Line::from(format!("Waiting for log lines... ({})", self.status))]
        } else {
            visible.iter().map(|l| Line::from(l.rendered.as_str())).collect()
        };
        let content_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: inner.height.saturating_sub(1) };
        let paragraph = if self.wrap {
            Paragraph::new(content).wrap(Wrap { trim: false })
        } else {
            Paragraph::new(content).scroll((0, horizontal_offset as u16))
        };
        frame.render_widget(paragraph, content_area);

        let mode_text = if self.follow { "FOLLOW" } else { "PAUSED" };
        let wrap_mode = if self.wrap { "WRAP" } else { "NOWRAP" };
        let footer = format!("{mode_text} | {wrap_mode} | {} lines | {}", self.lines.len(), self.status);
        let footer = if self.filter_text.is_empty() {
            footer
        } else {
            format!(
                "{mode_text} | {wrap_mode} | filter:\"{}\" | {filtered_total}/{total} lines | {}",
                self.filter_text, self.status
            )
        };
        let footer_area =
            Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(1), width: inner.width, height: 1 };
        frame.render_widget(Paragraph::new(footer).style(theme.status_bar), footer_area);
    }

    fn handle_command(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::ScrollUp | PaneCommand::SelectPrev => {
                self.follow = false;
                self.scroll_offset = self.scroll_offset.saturating_add(1).min(self.max_scroll_offset.get());
            }
            PaneCommand::ScrollDown | PaneCommand::SelectNext => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                if self.scroll_offset == 0 {
                    self.follow = true;
                }
            }
            PaneCommand::ScrollLeft => {
                if !self.wrap {
                    self.horizontal_offset = self.horizontal_offset.saturating_sub(4);
                }
            }
            PaneCommand::ScrollRight => {
                if !self.wrap {
                    self.horizontal_offset =
                        self.horizontal_offset.saturating_add(4).min(self.max_horizontal_offset.get());
                }
            }
            PaneCommand::ToggleFollow => {
                self.follow = !self.follow;
                if self.follow {
                    self.scroll_offset = 0;
                }
            }
            PaneCommand::ToggleWrap => {
                self.wrap = !self.wrap;
                if self.wrap {
                    self.horizontal_offset = 0;
                } else {
                    self.horizontal_offset = self.horizontal_offset.min(self.max_horizontal_offset.get());
                }
            }
            PaneCommand::Filter(text) => {
                self.filter_text = text.clone();
                self.scroll_offset = 0;
            }
            PaneCommand::ClearFilter => {
                self.filter_text.clear();
                self.scroll_offset = 0;
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

fn format_log_line(line: &LogLine) -> String {
    sanitize_log_text(&line.content)
}

fn sanitize_log_text(input: &str) -> String {
    #[derive(Clone, Copy)]
    enum EscapeState {
        None,
        Esc,
        Csi,
        Osc,
        OscEsc,
    }

    let mut out = String::with_capacity(input.len());
    let mut state = EscapeState::None;

    for ch in input.chars() {
        match state {
            EscapeState::None => {
                if ch == '\u{1b}' {
                    state = EscapeState::Esc;
                    continue;
                }

                if ch == '\r' {
                    continue;
                }

                if ch.is_control() && ch != '\t' {
                    out.push(' ');
                    continue;
                }

                out.push(ch);
            }
            EscapeState::Esc => {
                state = match ch {
                    '[' => EscapeState::Csi,
                    ']' => EscapeState::Osc,
                    _ => EscapeState::None,
                };
            }
            EscapeState::Csi => {
                if ('@'..='~').contains(&ch) {
                    state = EscapeState::None;
                }
            }
            EscapeState::Osc => {
                if ch == '\u{7}' {
                    state = EscapeState::None;
                } else if ch == '\u{1b}' {
                    state = EscapeState::OscEsc;
                }
            }
            EscapeState::OscEsc => {
                state = if ch == '\\' { EscapeState::None } else { EscapeState::Osc };
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::{sanitize_log_text, LogsPane};
    use crystal_tui::pane::{Pane, PaneCommand};
    use kubetile_core::LogLine;

    #[test]
    fn sanitize_strips_ansi_sequences() {
        let input = "\u{1b}[31mERROR\u{1b}[0m message";
        assert_eq!(sanitize_log_text(input), "ERROR message");
    }

    #[test]
    fn sanitize_drops_carriage_returns_and_controls() {
        let input = "line1\r\nline2\u{0007}";
        assert_eq!(sanitize_log_text(input), "line1 line2 ");
    }

    #[test]
    fn append_snapshot_sorts_by_log_timestamp() {
        let mut pane = LogsPane::new("pod-a".into(), "default".into());
        let newer = "2024-01-01T00:00:02Z".parse().unwrap();
        let older = "2024-01-01T00:00:01Z".parse().unwrap();

        pane.append_snapshot(vec![
            LogLine { timestamp: Some(newer), content: "new".into(), container: "main".into(), is_stderr: false },
            LogLine { timestamp: Some(older), content: "old".into(), container: "main".into(), is_stderr: false },
        ]);

        assert!(pane.lines[0].rendered.contains("old"));
        assert!(pane.lines[1].rendered.contains("new"));
    }

    #[test]
    fn append_snapshot_preserves_arrival_order_when_timestamps_missing() {
        let mut pane = LogsPane::new("pod-a".into(), "default".into());
        pane.append_snapshot(vec![
            LogLine { timestamp: None, content: "first".into(), container: "main".into(), is_stderr: false },
            LogLine { timestamp: None, content: "second".into(), container: "main".into(), is_stderr: false },
        ]);

        assert!(pane.lines[0].rendered.contains("first"));
        assert!(pane.lines[1].rendered.contains("second"));
    }

    #[test]
    fn filter_matches_log_content_case_insensitive() {
        let mut pane = LogsPane::new("pod-a".into(), "default".into());
        pane.append_snapshot(vec![
            LogLine { timestamp: None, content: "Error connecting".into(), container: "main".into(), is_stderr: false },
            LogLine { timestamp: None, content: "ready".into(), container: "main".into(), is_stderr: false },
        ]);

        pane.handle_command(&PaneCommand::Filter("ERROR".into()));
        let filtered = pane.filtered_lines();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].rendered, "Error connecting");
    }

    #[test]
    fn clear_filter_restores_all_lines() {
        let mut pane = LogsPane::new("pod-a".into(), "default".into());
        pane.append_snapshot(vec![
            LogLine { timestamp: None, content: "alpha".into(), container: "main".into(), is_stderr: false },
            LogLine { timestamp: None, content: "beta".into(), container: "main".into(), is_stderr: false },
        ]);

        pane.handle_command(&PaneCommand::Filter("alpha".into()));
        assert_eq!(pane.filtered_lines().len(), 1);

        pane.handle_command(&PaneCommand::ClearFilter);
        assert_eq!(pane.filtered_lines().len(), 2);
    }
}
