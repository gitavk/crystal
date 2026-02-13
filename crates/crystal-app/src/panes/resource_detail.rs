use std::any::Any;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use crystal_core::resource::DetailSection;
use crystal_tui::pane::{Pane, PaneCommand, ResourceKind, ViewType};
use crystal_tui::theme;
use crystal_tui::widgets::breadcrumb::BreadcrumbWidget;

#[allow(dead_code)]
pub struct ResourceDetailPane {
    view_type: ViewType,
    kind: ResourceKind,
    name: String,
    namespace: Option<String>,
    sections: Vec<DetailSection>,
    scroll_offset: usize,
    selected_section: usize,
    visible_height: u16,
}

#[allow(dead_code)]
impl ResourceDetailPane {
    pub fn new(kind: ResourceKind, name: String, namespace: Option<String>, sections: Vec<DetailSection>) -> Self {
        Self {
            view_type: ViewType::Detail(kind.clone(), name.clone()),
            kind,
            name,
            namespace,
            sections,
            scroll_offset: 0,
            selected_section: 0,
            visible_height: 0,
        }
    }

    fn total_content_height(&self) -> usize {
        let mut height = 0;
        for section in &self.sections {
            height += 2; // top border + title line
            height += section.fields.len(); // one line per field
            height += 1; // bottom border
            height += 1; // spacing between sections
        }
        height
    }

    fn color_for_status_value(value: &str) -> Color {
        let lower = value.to_lowercase();
        match lower.as_str() {
            "running" | "active" | "ready" | "true" | "succeeded" | "bound" | "available" => theme::STATUS_RUNNING,
            "failed" | "error" | "crashloopbackoff" | "imagepullbackoff" | "false" | "evicted" => theme::STATUS_FAILED,
            "pending" | "waiting" | "terminating" | "containercreating" | "unknown" => theme::STATUS_PENDING,
            _ => theme::HEADER_FG,
        }
    }
}

impl Pane for ResourceDetailPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_color = if focused { theme::ACCENT } else { theme::BORDER_COLOR };
        let outer_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(border_color));
        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        if inner.height < 2 || inner.width < 4 {
            return;
        }

        // Breadcrumb at top
        let breadcrumb_area = Rect { x: inner.x + 1, y: inner.y, width: inner.width.saturating_sub(2), height: 1 };
        let kind_name = self.kind.display_name();
        let segments: Vec<&str> = vec![kind_name, &self.name];
        frame.render_widget(BreadcrumbWidget { segments: &segments }, breadcrumb_area);

        // Content area below breadcrumb
        let content_area =
            Rect { x: inner.x, y: inner.y + 1, width: inner.width, height: inner.height.saturating_sub(1) };

        let mut lines: Vec<Line> = Vec::new();
        for (idx, section) in self.sections.iter().enumerate() {
            let is_selected = idx == self.selected_section;
            let title_style = if is_selected {
                Style::default().fg(theme::ACCENT).bold()
            } else {
                Style::default().fg(theme::TABLE_HEADER_FG).bold()
            };
            let border_char_style =
                if is_selected { Style::default().fg(theme::ACCENT) } else { Style::default().fg(theme::BORDER_COLOR) };

            // Section header
            let title_line = Line::from(vec![
                Span::styled("┌─ ", border_char_style),
                Span::styled(&section.title, title_style),
                Span::styled(
                    " ─".to_string()
                        + &"─".repeat(content_area.width.saturating_sub(section.title.len() as u16 + 6) as usize)
                        + "┐",
                    border_char_style,
                ),
            ]);
            lines.push(title_line);

            for (key, value) in &section.fields {
                let value_color = Self::color_for_status_value(value);
                lines.push(Line::from(vec![
                    Span::styled("│ ", border_char_style),
                    Span::styled(format!("{key:<14}"), Style::default().fg(theme::TEXT_DIM)),
                    Span::styled(value, Style::default().fg(value_color)),
                ]));
            }

            let bottom_line = Line::from(vec![Span::styled(
                "└".to_string() + &"─".repeat(content_area.width.saturating_sub(2) as usize) + "┘",
                border_char_style,
            )]);
            lines.push(bottom_line);
            lines.push(Line::from(""));
        }

        let total_lines = lines.len();
        let scroll = self.scroll_offset.min(total_lines.saturating_sub(content_area.height as usize));
        let paragraph = Paragraph::new(lines).scroll((scroll as u16, 0));
        frame.render_widget(paragraph, content_area);

        // Scrollbar
        if total_lines > content_area.height as usize {
            let mut scrollbar_state =
                ScrollbarState::new(total_lines.saturating_sub(content_area.height as usize)).position(scroll);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                content_area,
                &mut scrollbar_state,
            );
        }
    }

    fn handle_command(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::ScrollUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            PaneCommand::ScrollDown => {
                self.scroll_offset += 1;
            }
            PaneCommand::SelectNext => {
                if self.selected_section < self.sections.len().saturating_sub(1) {
                    self.selected_section += 1;
                }
            }
            PaneCommand::SelectPrev => {
                self.selected_section = self.selected_section.saturating_sub(1);
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

    fn sample_sections() -> Vec<DetailSection> {
        vec![
            DetailSection {
                title: "Metadata".into(),
                fields: vec![("Name".into(), "nginx-abc123".into()), ("Namespace".into(), "default".into())],
            },
            DetailSection {
                title: "Status".into(),
                fields: vec![("Phase".into(), "Running".into()), ("Ready".into(), "1/1".into())],
            },
            DetailSection {
                title: "Containers".into(),
                fields: vec![("Image".into(), "nginx:1.25".into()), ("Ready".into(), "true".into())],
            },
        ]
    }

    #[test]
    fn scroll_down_increments_offset() {
        let mut pane = ResourceDetailPane::new(ResourceKind::Pods, "test".into(), None, sample_sections());
        assert_eq!(pane.scroll_offset, 0);
        pane.handle_command(&PaneCommand::ScrollDown);
        assert_eq!(pane.scroll_offset, 1);
        pane.handle_command(&PaneCommand::ScrollDown);
        assert_eq!(pane.scroll_offset, 2);
    }

    #[test]
    fn scroll_up_decrements_offset() {
        let mut pane = ResourceDetailPane::new(ResourceKind::Pods, "test".into(), None, sample_sections());
        pane.scroll_offset = 5;
        pane.handle_command(&PaneCommand::ScrollUp);
        assert_eq!(pane.scroll_offset, 4);
    }

    #[test]
    fn scroll_up_clamps_at_zero() {
        let mut pane = ResourceDetailPane::new(ResourceKind::Pods, "test".into(), None, sample_sections());
        pane.handle_command(&PaneCommand::ScrollUp);
        assert_eq!(pane.scroll_offset, 0);
    }

    #[test]
    fn select_next_advances_section() {
        let mut pane = ResourceDetailPane::new(ResourceKind::Pods, "test".into(), None, sample_sections());
        assert_eq!(pane.selected_section, 0);
        pane.handle_command(&PaneCommand::SelectNext);
        assert_eq!(pane.selected_section, 1);
        pane.handle_command(&PaneCommand::SelectNext);
        assert_eq!(pane.selected_section, 2);
    }

    #[test]
    fn select_next_clamps_at_last_section() {
        let mut pane = ResourceDetailPane::new(ResourceKind::Pods, "test".into(), None, sample_sections());
        pane.handle_command(&PaneCommand::SelectNext);
        pane.handle_command(&PaneCommand::SelectNext);
        pane.handle_command(&PaneCommand::SelectNext);
        assert_eq!(pane.selected_section, 2);
    }

    #[test]
    fn select_prev_decrements_section() {
        let mut pane = ResourceDetailPane::new(ResourceKind::Pods, "test".into(), None, sample_sections());
        pane.selected_section = 2;
        pane.handle_command(&PaneCommand::SelectPrev);
        assert_eq!(pane.selected_section, 1);
    }

    #[test]
    fn select_prev_clamps_at_zero() {
        let mut pane = ResourceDetailPane::new(ResourceKind::Pods, "test".into(), None, sample_sections());
        pane.handle_command(&PaneCommand::SelectPrev);
        assert_eq!(pane.selected_section, 0);
    }

    #[test]
    fn renders_all_sections() {
        let pane =
            ResourceDetailPane::new(ResourceKind::Pods, "nginx".into(), Some("default".into()), sample_sections());
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(60, 30)).unwrap();
        terminal
            .draw(|frame| {
                pane.render(frame, Rect::new(0, 0, 60, 30), true);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = (0..30)
            .map(|y| {
                (0..60).map(|x| buf.cell((x, y)).unwrap().symbol().chars().next().unwrap_or(' ')).collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(content.contains("Metadata"));
        assert!(content.contains("Status"));
        assert!(content.contains("Containers"));
        assert!(content.contains("Pods"));
        assert!(content.contains("nginx"));
    }

    #[test]
    fn status_value_colors() {
        assert_eq!(ResourceDetailPane::color_for_status_value("Running"), theme::STATUS_RUNNING);
        assert_eq!(ResourceDetailPane::color_for_status_value("Failed"), theme::STATUS_FAILED);
        assert_eq!(ResourceDetailPane::color_for_status_value("Pending"), theme::STATUS_PENDING);
    }

    #[test]
    fn view_type_is_detail() {
        let pane = ResourceDetailPane::new(ResourceKind::Pods, "test".into(), None, vec![]);
        assert_eq!(*pane.view_type(), ViewType::Detail(ResourceKind::Pods, "test".into()));
    }
}
