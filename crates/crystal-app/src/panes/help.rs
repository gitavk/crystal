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
            ResourceKind::Pods => vec![("L", "Logs"), ("E", "Exec"), ("P", "Port Forward")],
            ResourceKind::Deployments => vec![("S", "Scale"), ("R", "Restart")],
            ResourceKind::StatefulSets => vec![("S", "Scale")],
            _ => vec![],
        }
    }

    fn normalize_shortcuts(entries: &[(String, String)]) -> Vec<(String, String)> {
        let mut order: Vec<String> = Vec::new();
        let mut grouped: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

        for (key, desc) in entries {
            if !grouped.contains_key(desc) {
                order.push(desc.clone());
            }
            grouped.entry(desc.clone()).or_default().push(key.clone());
        }

        let mut normalized = Vec::new();
        for desc in order {
            let keys = grouped.remove(&desc).unwrap_or_default();
            let display = Self::compact_keys(&desc, &keys);
            normalized.push((display, desc));
        }

        normalized
    }

    fn compact_keys(desc: &str, keys: &[String]) -> String {
        let mut unique: Vec<String> = Vec::new();
        for key in keys {
            if !unique.contains(key) {
                unique.push(key.clone());
            }
        }

        if desc == "Go to tab" {
            let mut numbers: Vec<u8> = Vec::new();
            for key in &unique {
                if key.len() == 1 {
                    if let Some(digit) = key.chars().next().and_then(|c| c.to_digit(10)) {
                        numbers.push(digit as u8);
                        continue;
                    }
                }
                return unique.join(", ");
            }

            numbers.sort_unstable();
            numbers.dedup();
            if numbers.len() >= 2
                && numbers.last().copied() == numbers.first().copied().map(|v| v + numbers.len() as u8 - 1)
            {
                return format!("{}-{}", numbers[0], numbers[numbers.len() - 1]);
            }
        }

        if unique.len() == 1 {
            unique[0].clone()
        } else {
            unique.join(", ")
        }
    }

    fn format_key(key: &str) -> String {
        key.to_ascii_uppercase()
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
        for (key, desc) in Self::normalize_shortcuts(&self.global_shortcuts) {
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<16}", Self::format_key(&key)), Style::default().fg(theme::HEADER_FG).bold()),
                Span::styled(desc, Style::default().fg(theme::STATUS_FG)),
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
            for (key, desc) in Self::normalize_shortcuts(&self.pane_shortcuts) {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:<16}", Self::format_key(&key)),
                        Style::default().fg(theme::HEADER_FG).bold(),
                    ),
                    Span::styled(desc, Style::default().fg(theme::STATUS_FG)),
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

            let normalized_resource = Self::normalize_shortcuts(&self.resource_shortcuts);
            let mut seen_desc: std::collections::HashSet<String> = std::collections::HashSet::new();
            for (_, desc) in &normalized_resource {
                seen_desc.insert(desc.clone());
            }

            for (key, desc) in normalized_resource {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:<16}", Self::format_key(&key)),
                        Style::default().fg(theme::HEADER_FG).bold(),
                    ),
                    Span::styled(desc.clone(), Style::default().fg(theme::STATUS_FG)),
                ]));
            }

            let specific = Self::resource_specific_entries(kind);
            for (key, desc) in specific {
                if seen_desc.contains(desc) {
                    continue;
                }
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {:<16}", Self::format_key(key)),
                        Style::default().fg(theme::HEADER_FG).bold(),
                    ),
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
        assert!(entries.iter().any(|(_, d)| *d == "Port Forward"));
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
