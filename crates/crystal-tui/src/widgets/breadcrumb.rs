use ratatui::prelude::*;

use crate::theme::Theme;

pub struct BreadcrumbWidget<'a> {
    pub segments: &'a [&'a str],
    pub theme: &'a Theme,
}

impl<'a> BreadcrumbWidget<'a> {
    pub fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 || self.segments.is_empty() {
            return;
        }

        let t = self.theme;
        let mut spans: Vec<Span> = Vec::new();
        let last_idx = self.segments.len() - 1;
        for (i, segment) in self.segments.iter().enumerate() {
            let style = if i == last_idx { Style::default().fg(t.accent).bold() } else { Style::default().fg(t.fg) };
            spans.push(Span::styled(*segment, style));
            if i < last_idx {
                spans.push(Span::styled(" > ", t.text_dim));
            }
        }

        let line = Line::from(spans);
        buf.set_line(area.x, area.y, &line, area.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_theme() -> Theme {
        Theme::default()
    }

    #[test]
    fn breadcrumb_renders_segments_with_separator() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 30, 1));
        let theme = default_theme();
        let widget = BreadcrumbWidget { segments: &["Pods", "nginx-abc123"], theme: &theme };
        widget.render(Rect::new(0, 0, 30, 1), &mut buf);
        let content: String = (0..30).map(|x| buf.cell((x, 0)).unwrap().symbol().chars().next().unwrap()).collect();
        assert!(content.contains("Pods"));
        assert!(content.contains(">"));
        assert!(content.contains("nginx-abc123"));
    }

    #[test]
    fn breadcrumb_renders_single_segment() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        let theme = default_theme();
        let widget = BreadcrumbWidget { segments: &["Pods"], theme: &theme };
        widget.render(Rect::new(0, 0, 20, 1), &mut buf);
        let content: String = (0..20).map(|x| buf.cell((x, 0)).unwrap().symbol().chars().next().unwrap()).collect();
        assert!(content.contains("Pods"));
        assert!(!content.contains(">"));
    }

    #[test]
    fn breadcrumb_renders_three_segments() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 40, 1));
        let theme = default_theme();
        let widget = BreadcrumbWidget { segments: &["A", "B", "C"], theme: &theme };
        widget.render(Rect::new(0, 0, 40, 1), &mut buf);
        let content: String = (0..40).map(|x| buf.cell((x, 0)).unwrap().symbol().chars().next().unwrap()).collect();
        assert!(content.contains("A"));
        assert!(content.contains("B"));
        assert!(content.contains("C"));
    }

    #[test]
    fn breadcrumb_empty_segments_renders_nothing() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1));
        let theme = default_theme();
        let widget = BreadcrumbWidget { segments: &[], theme: &theme };
        widget.render(Rect::new(0, 0, 20, 1), &mut buf);
        let content: String = (0..20).map(|x| buf.cell((x, 0)).unwrap().symbol().chars().next().unwrap()).collect();
        assert_eq!(content.trim(), "");
    }
}
