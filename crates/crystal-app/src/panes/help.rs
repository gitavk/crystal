use std::any::Any;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crystal_tui::pane::{Pane, PaneCommand, ResourceKind, ViewType};
use crystal_tui::theme;

pub struct HelpPane {
    context_view: Option<ViewType>,
    scroll_offset: u16,
    global_shortcuts: Vec<(String, String)>,
    pane_shortcuts: Vec<(String, String)>,
    resource_shortcuts: Vec<(String, String)>,
}

impl HelpPane {
    pub fn new(
        global_shortcuts: Vec<(String, String)>,
        pane_shortcuts: Vec<(String, String)>,
        resource_shortcuts: Vec<(String, String)>,
    ) -> Self {
        Self { context_view: None, scroll_offset: 0, global_shortcuts, pane_shortcuts, resource_shortcuts }
    }

    fn resource_specific_entries(kind: &ResourceKind) -> Vec<(&'static str, &'static str)> {
        match kind {
            ResourceKind::Pods => vec![("l", "Logs"), ("e", "Exec")],
            ResourceKind::Deployments => vec![("S", "Scale"), ("R", "Restart")],
            ResourceKind::StatefulSets => vec![("S", "Scale")],
            _ => vec![],
        }
    }
}

impl Pane for HelpPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_color = if focused { theme::ACCENT } else { theme::BORDER_COLOR };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(" Help ")
            .title_style(Style::default().fg(theme::ACCENT).bold());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled("Global Shortcuts", Style::default().fg(theme::ACCENT).bold())));
        lines.push(Line::from(""));
        for (key, desc) in &self.global_shortcuts {
            lines.push(Line::from(vec![
                Span::styled(format!("  {key:<16}"), Style::default().fg(theme::HEADER_FG).bold()),
                Span::styled(desc.as_str(), Style::default().fg(theme::STATUS_FG)),
            ]));
        }

        if self.context_view.is_some() && !self.pane_shortcuts.is_empty() {
            let label = match self.context_view.as_ref() {
                Some(ViewType::ResourceList(_)) => "Navigation",
                Some(ViewType::Logs(_)) => "Logs",
                Some(ViewType::Terminal) => "Terminal",
                _ => "Pane",
            };
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("{label} Shortcuts"),
                Style::default().fg(theme::ACCENT).bold(),
            )));
            lines.push(Line::from(""));
            for (key, desc) in &self.pane_shortcuts {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {key:<16}"), Style::default().fg(theme::HEADER_FG).bold()),
                    Span::styled(desc.as_str(), Style::default().fg(theme::STATUS_FG)),
                ]));
            }
        }

        if let Some(ViewType::ResourceList(ref kind)) = self.context_view {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("{} Actions", kind.display_name()),
                Style::default().fg(theme::ACCENT).bold(),
            )));
            lines.push(Line::from(""));

            for (key, desc) in &self.resource_shortcuts {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {key:<16}"), Style::default().fg(theme::HEADER_FG).bold()),
                    Span::styled(desc.as_str(), Style::default().fg(theme::STATUS_FG)),
                ]));
            }

            let specific = Self::resource_specific_entries(kind);
            for (key, desc) in specific {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {key:<16}"), Style::default().fg(theme::HEADER_FG).bold()),
                    Span::styled(desc, Style::default().fg(theme::STATUS_FG)),
                ]));
            }
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false }).scroll((self.scroll_offset, 0));
        frame.render_widget(paragraph, inner);
    }

    fn handle_command(&mut self, cmd: &PaneCommand) {
        match cmd {
            PaneCommand::ScrollUp | PaneCommand::SelectPrev => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            PaneCommand::ScrollDown | PaneCommand::SelectNext => {
                self.scroll_offset += 1;
            }
            _ => {}
        }
    }

    fn view_type(&self) -> &ViewType {
        &ViewType::Help
    }

    fn on_focus_change(&mut self, previous: Option<&ViewType>) {
        self.context_view = previous.cloned();
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

    fn make_help(context: Option<ViewType>) -> HelpPane {
        let global = vec![("q".into(), "Quit".into()), ("?".into(), "Help".into())];
        let pane = vec![("j".into(), "Down".into()), ("k".into(), "Up".into())];
        let resource =
            vec![("y".into(), "View YAML".into()), ("d".into(), "Describe".into()), ("Ctrl+d".into(), "Delete".into())];
        let mut help = HelpPane::new(global, pane, resource);
        help.context_view = context;
        help
    }

    #[test]
    fn pods_context_shows_logs_and_exec() {
        let help = make_help(Some(ViewType::ResourceList(ResourceKind::Pods)));
        let entries = HelpPane::resource_specific_entries(&ResourceKind::Pods);
        assert!(entries.iter().any(|(_, d)| *d == "Logs"));
        assert!(entries.iter().any(|(_, d)| *d == "Exec"));
        assert_eq!(help.view_type(), &ViewType::Help);
    }

    #[test]
    fn deployments_context_shows_scale_and_restart() {
        let entries = HelpPane::resource_specific_entries(&ResourceKind::Deployments);
        assert!(entries.iter().any(|(_, d)| *d == "Scale"));
        assert!(entries.iter().any(|(_, d)| *d == "Restart"));
    }

    #[test]
    fn configmaps_context_shows_no_extra_entries() {
        let entries = HelpPane::resource_specific_entries(&ResourceKind::ConfigMaps);
        assert!(entries.is_empty());
    }

    #[test]
    fn statefulsets_context_shows_scale_but_no_restart() {
        let entries = HelpPane::resource_specific_entries(&ResourceKind::StatefulSets);
        assert!(entries.iter().any(|(_, d)| *d == "Scale"));
        assert!(!entries.iter().any(|(_, d)| *d == "Restart"));
    }

    #[test]
    fn help_pane_resource_shortcuts_from_dispatcher() {
        let help = make_help(Some(ViewType::ResourceList(ResourceKind::Pods)));
        assert!(help.resource_shortcuts.iter().any(|(_, d)| d == "View YAML"));
        assert!(help.resource_shortcuts.iter().any(|(_, d)| d == "Describe"));
        assert!(help.resource_shortcuts.iter().any(|(_, d)| d == "Delete"));
    }
}
