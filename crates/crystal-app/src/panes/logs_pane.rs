use std::any::Any;
use std::cell::Cell;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crystal_core::{LogLine, LogStream, StreamStatus};
use crystal_tui::pane::{Pane, PaneCommand, ViewType};
use crystal_tui::theme::Theme;

const MAX_LOG_LINES: usize = 5000;

pub struct LogsPane {
    view_type: ViewType,
    pod_name: String,
    namespace: String,
    lines: Vec<String>,
    scroll_offset: usize,
    follow: bool,
    status: String,
    stream: Option<LogStream>,
    max_scroll_offset: Cell<usize>,
}

impl LogsPane {
    pub fn new(pod_name: String, namespace: String) -> Self {
        Self {
            view_type: ViewType::Logs(pod_name.clone()),
            pod_name,
            namespace,
            lines: Vec::new(),
            scroll_offset: 0,
            follow: true,
            status: "Connecting...".into(),
            stream: None,
            max_scroll_offset: Cell::new(0),
        }
    }

    pub fn attach_stream(&mut self, stream: LogStream) {
        self.stream = Some(stream);
        self.status = "Streaming".into();
    }

    pub fn append_snapshot(&mut self, lines: Vec<String>) {
        if !lines.is_empty() {
            self.lines.extend(lines.into_iter().map(|line| sanitize_log_text(&line)));
            self.lines.sort();
            self.lines.dedup();
            if self.lines.len() > MAX_LOG_LINES {
                let drop_count = self.lines.len().saturating_sub(MAX_LOG_LINES);
                self.lines.drain(0..drop_count);
            }
        }
        if self.status == "Connecting..." {
            self.status = "Snapshot loaded".into();
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.stream = None;
        self.status = format!("Error: {error}");
    }

    pub fn poll(&mut self) {
        let Some(stream) = self.stream.as_mut() else {
            return;
        };

        let new_lines = stream.next_lines();
        if !new_lines.is_empty() {
            for line in new_lines {
                self.lines.push(format_log_line(&line));
            }
            self.lines.sort();
            self.lines.dedup();
            if self.lines.len() > MAX_LOG_LINES {
                let drop_count = self.lines.len().saturating_sub(MAX_LOG_LINES);
                self.lines.drain(0..drop_count);
            }
        }

        self.status = match stream.status() {
            StreamStatus::Streaming => "Streaming".into(),
            StreamStatus::Reconnecting { attempt } => format!("Reconnecting ({attempt})"),
            StreamStatus::Stopped => "Stopped".into(),
            StreamStatus::Error => "Error".into(),
        };
    }

    fn render_title(&self) -> String {
        format!("[logs:{} @ {}]", self.pod_name, self.namespace)
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

        let visible_height = inner.height.saturating_sub(1) as usize;
        let total = self.lines.len();
        let max_offset = total.saturating_sub(visible_height);
        self.max_scroll_offset.set(max_offset);
        let offset = if self.follow { 0 } else { self.scroll_offset.min(max_offset) };
        let end = total.saturating_sub(offset);
        let start = end.saturating_sub(visible_height);
        let visible = &self.lines[start..end];

        let content = if visible.is_empty() {
            vec![Line::from(format!("Waiting for log lines... ({})", self.status))]
        } else {
            visible.iter().map(|l| Line::from(l.as_str())).collect()
        };
        let content_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: inner.height.saturating_sub(1) };
        frame.render_widget(Paragraph::new(content).wrap(Wrap { trim: false }), content_area);

        let mode_text = if self.follow { "FOLLOW" } else { "PAUSED" };
        let footer = format!("{mode_text} | {} lines | {}", self.lines.len(), self.status);
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
            PaneCommand::ToggleFollow => {
                self.follow = !self.follow;
                if self.follow {
                    self.scroll_offset = 0;
                }
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
    match &line.timestamp {
        Some(ts) => format!("{ts} {}", sanitize_log_text(&line.content)),
        None => sanitize_log_text(&line.content),
    }
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
    use super::sanitize_log_text;

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
}
