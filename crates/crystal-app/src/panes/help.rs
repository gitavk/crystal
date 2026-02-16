use std::any::Any;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crystal_tui::pane::{Pane, PaneCommand, ResourceKind, ViewType};
use crystal_tui::theme::Theme;

pub struct HelpPane {
    context_view: Option<ViewType>,
    scroll_offset: u16,
    global_shortcuts: Vec<(String, String)>,
    navigation_shortcuts: Vec<(String, String)>,
    browse_shortcuts: Vec<(String, String)>,
    tui_shortcuts: Vec<(String, String)>,
    mutate_shortcuts: Vec<(String, String)>,
}

impl HelpPane {
    pub fn new(
        global_shortcuts: Vec<(String, String)>,
        navigation_shortcuts: Vec<(String, String)>,
        browse_shortcuts: Vec<(String, String)>,
        tui_shortcuts: Vec<(String, String)>,
        mutate_shortcuts: Vec<(String, String)>,
    ) -> Self {
        Self {
            context_view: None,
            scroll_offset: 0,
            global_shortcuts,
            navigation_shortcuts,
            browse_shortcuts,
            tui_shortcuts,
            mutate_shortcuts,
        }
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
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme) {
        let border_style = if focused { theme.border_active } else { theme.border };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(" Help ")
            .title_style(Style::default().fg(theme.accent).bold());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line> = Vec::new();

        let sections: &[(&str, &[(String, String)])] = &[
            ("Global (Ctrl+)", &self.global_shortcuts),
            ("Navigation", &self.navigation_shortcuts),
            ("Browse", &self.browse_shortcuts),
            ("TUI (Alt+)", &self.tui_shortcuts),
            ("Mutate (Ctrl+Alt+)", &self.mutate_shortcuts),
        ];

        for (i, (title, shortcuts)) in sections.iter().enumerate() {
            if shortcuts.is_empty() {
                continue;
            }
            if i > 0 {
                lines.push(Line::from(""));
            }
            lines
                .push(Line::from(Span::styled(format!("{title} Shortcuts"), Style::default().fg(theme.accent).bold())));
            lines.push(Line::from(""));
            for (key, desc) in Self::normalize_shortcuts(shortcuts) {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {:<16}", Self::format_key(&key)), Style::default().fg(theme.fg).bold()),
                    Span::styled(desc, theme.text_dim),
                ]));
            }
        }

        if let Some(ViewType::ResourceList(ref kind)) = self.context_view {
            let specific = Self::resource_specific_entries(kind);
            if !specific.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("{} Actions", kind.display_name()),
                    Style::default().fg(theme.accent).bold(),
                )));
                lines.push(Line::from(""));
                for (key, desc) in specific {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {:<16}", Self::format_key(key)), Style::default().fg(theme.fg).bold()),
                        Span::styled(desc, theme.text_dim),
                    ]));
                }
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
        let global = vec![("Ctrl+Q".into(), "Quit".into()), ("?".into(), "Help".into())];
        let navigation = vec![("J".into(), "Down".into()), ("K".into(), "Up".into())];
        let browse = vec![("Y".into(), "View YAML".into()), ("D".into(), "Describe".into())];
        let tui = vec![("Alt+V".into(), "Split V".into())];
        let mutate = vec![("Ctrl+Alt+D".into(), "Delete".into())];
        let mut help = HelpPane::new(global, navigation, browse, tui, mutate);
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
    fn help_pane_shortcuts_from_dispatcher() {
        let help = make_help(Some(ViewType::ResourceList(ResourceKind::Pods)));
        assert!(help.browse_shortcuts.iter().any(|(_, d)| d == "View YAML"));
        assert!(help.browse_shortcuts.iter().any(|(_, d)| d == "Describe"));
        assert!(help.mutate_shortcuts.iter().any(|(_, d)| d == "Delete"));
    }
}
