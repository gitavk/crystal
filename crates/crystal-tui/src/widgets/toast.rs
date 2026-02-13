use std::time::{Duration, Instant};

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::theme;

#[derive(Clone, Debug)]
pub enum ToastLevel {
    Success,
    Error,
    Info,
}

#[derive(Clone, Debug)]
pub struct ToastMessage {
    pub text: String,
    pub level: ToastLevel,
    pub created_at: Instant,
    pub ttl: Duration,
}

impl ToastMessage {
    pub fn success(text: impl Into<String>) -> Self {
        Self { text: text.into(), level: ToastLevel::Success, created_at: Instant::now(), ttl: Duration::from_secs(3) }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self { text: text.into(), level: ToastLevel::Error, created_at: Instant::now(), ttl: Duration::from_secs(5) }
    }

    pub fn info(text: impl Into<String>) -> Self {
        Self { text: text.into(), level: ToastLevel::Info, created_at: Instant::now(), ttl: Duration::from_secs(3) }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.ttl
    }
}

pub struct ToastWidget<'a> {
    pub toasts: &'a [ToastMessage],
}

impl<'a> ToastWidget<'a> {
    pub fn render(self, frame: &mut Frame, area: Rect) {
        let max_visible = 3;
        let visible: Vec<_> = self.toasts.iter().rev().take(max_visible).collect();

        let mut y_offset = area.y + area.height;

        for toast in &visible {
            let text_width = toast.text.len() as u16 + 4;
            let width = text_width.max(20).min(area.width.saturating_sub(2));
            let height = 3_u16;

            y_offset = y_offset.saturating_sub(height);

            let toast_area = Rect { x: area.x + area.width.saturating_sub(width + 1), y: y_offset, width, height };

            if toast_area.y < area.y {
                break;
            }

            let (border_color, prefix) = match toast.level {
                ToastLevel::Success => (theme::STATUS_RUNNING, "ok "),
                ToastLevel::Error => (theme::STATUS_FAILED, "err "),
                ToastLevel::Info => (theme::ACCENT, ""),
            };

            frame.render_widget(Clear, toast_area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme::OVERLAY_BG));

            let inner = block.inner(toast_area);
            frame.render_widget(block, toast_area);

            let text = Paragraph::new(format!("{prefix}{}", toast.text)).style(Style::default().fg(theme::HEADER_FG));
            frame.render_widget(text, inner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_toast_has_3s_ttl() {
        let toast = ToastMessage::success("done");
        assert_eq!(toast.ttl, Duration::from_secs(3));
        assert!(matches!(toast.level, ToastLevel::Success));
    }

    #[test]
    fn error_toast_has_5s_ttl() {
        let toast = ToastMessage::error("failed");
        assert_eq!(toast.ttl, Duration::from_secs(5));
        assert!(matches!(toast.level, ToastLevel::Error));
    }

    #[test]
    fn is_expired_returns_false_when_fresh() {
        let toast = ToastMessage::success("test");
        assert!(!toast.is_expired());
    }

    #[test]
    fn is_expired_returns_true_after_ttl() {
        let toast = ToastMessage {
            text: "old".into(),
            level: ToastLevel::Info,
            created_at: Instant::now() - Duration::from_secs(10),
            ttl: Duration::from_secs(3),
        };
        assert!(toast.is_expired());
    }

    #[test]
    fn cleanup_retains_unexpired_only() {
        let mut toasts = vec![
            ToastMessage::success("fresh"),
            ToastMessage {
                text: "stale".into(),
                level: ToastLevel::Error,
                created_at: Instant::now() - Duration::from_secs(10),
                ttl: Duration::from_secs(5),
            },
        ];
        toasts.retain(|t| !t.is_expired());
        assert_eq!(toasts.len(), 1);
        assert_eq!(toasts[0].text, "fresh");
    }
}
