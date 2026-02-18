use std::any::Any;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crystal_tui::pane::{Pane, PaneCommand, ViewType};
use crystal_tui::theme::Theme;

pub struct HelpPane {
    scroll_offset: u16,
    global_shortcuts: Vec<(String, String)>,
    navigation_shortcuts: Vec<(String, String)>,
    browse_shortcuts: Vec<(String, String)>,
    tui_shortcuts: Vec<(String, String)>,
    interact_shortcuts: Vec<(String, String)>,
    mutate_shortcuts: Vec<(String, String)>,
}

impl HelpPane {
    pub fn new(
        global_shortcuts: Vec<(String, String)>,
        navigation_shortcuts: Vec<(String, String)>,
        browse_shortcuts: Vec<(String, String)>,
        tui_shortcuts: Vec<(String, String)>,
        interact_shortcuts: Vec<(String, String)>,
        mutate_shortcuts: Vec<(String, String)>,
    ) -> Self {
        Self {
            scroll_offset: 0,
            global_shortcuts,
            navigation_shortcuts,
            browse_shortcuts,
            tui_shortcuts,
            interact_shortcuts,
            mutate_shortcuts,
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
            // Try alt+N pattern first (e.g. "alt+1".."alt+9" → "alt+[1-9]")
            let mut numbers: Vec<u8> = Vec::new();
            let mut all_alt = true;
            for key in &unique {
                let lower = key.to_ascii_lowercase();
                if let Some(suffix) = lower.strip_prefix("alt+") {
                    if let Some(digit) =
                        suffix.chars().next().filter(|_| suffix.len() == 1).and_then(|c| c.to_digit(10))
                    {
                        numbers.push(digit as u8);
                        continue;
                    }
                }
                all_alt = false;
                break;
            }
            if all_alt && !numbers.is_empty() {
                numbers.sort_unstable();
                numbers.dedup();
                if numbers.len() >= 2
                    && numbers.last().copied() == numbers.first().copied().map(|v| v + numbers.len() as u8 - 1)
                {
                    return format!("Alt+[{}-{}]", numbers[0], numbers[numbers.len() - 1]);
                }
                return numbers.iter().map(|n| format!("Alt+{n}")).collect::<Vec<_>>().join(", ");
            }

            // Fallback: bare digits (e.g. "1".."9" → "1-9")
            let mut bare: Vec<u8> = Vec::new();
            for key in &unique {
                if let Some(digit) = key.chars().next().filter(|_| key.len() == 1).and_then(|c| c.to_digit(10)) {
                    bare.push(digit as u8);
                    continue;
                }
                return unique.join(", ");
            }
            bare.sort_unstable();
            bare.dedup();
            if bare.len() >= 2 && bare.last().copied() == bare.first().copied().map(|v| v + bare.len() as u8 - 1) {
                return format!("{}-{}", bare[0], bare[bare.len() - 1]);
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
            ("Interact", &self.interact_shortcuts),
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

    fn make_help() -> HelpPane {
        let global = vec![("Ctrl+Q".into(), "Quit".into()), ("?".into(), "Help".into())];
        let navigation = vec![("J".into(), "Down".into()), ("K".into(), "Up".into())];
        let browse =
            vec![("Y".into(), "View YAML".into()), ("D".into(), "Describe".into()), ("L".into(), "Logs".into())];
        let tui = vec![("Alt+V".into(), "Split V".into())];
        let interact = vec![("E".into(), "Exec".into()), ("P".into(), "Port Forward".into())];
        let mutate = vec![
            ("Ctrl+Alt+D".into(), "Delete".into()),
            ("Ctrl+Alt+S".into(), "Scale".into()),
            ("Ctrl+Alt+R".into(), "Restart".into()),
        ];
        HelpPane::new(global, navigation, browse, tui, interact, mutate)
    }

    #[test]
    fn help_pane_view_type() {
        let help = make_help();
        assert_eq!(help.view_type(), &ViewType::Help);
    }

    #[test]
    fn help_pane_shortcuts_from_dispatcher() {
        let help = make_help();
        assert!(help.browse_shortcuts.iter().any(|(_, d)| d == "View YAML"));
        assert!(help.browse_shortcuts.iter().any(|(_, d)| d == "Describe"));
        assert!(help.mutate_shortcuts.iter().any(|(_, d)| d == "Delete"));
    }

    #[test]
    fn compact_keys_alt_digits_contiguous_range() {
        // keys arrive title-cased from format_key_display ("Alt+1" style)
        let keys: Vec<String> = (1..=9).map(|n| format!("Alt+{n}")).collect();
        assert_eq!(HelpPane::compact_keys("Go to tab", &keys), "Alt+[1-9]");
    }

    #[test]
    fn compact_keys_alt_digits_partial_range() {
        let keys: Vec<String> = (1..=3).map(|n| format!("Alt+{n}")).collect();
        assert_eq!(HelpPane::compact_keys("Go to tab", &keys), "Alt+[1-3]");
    }

    #[test]
    fn compact_keys_alt_digits_non_contiguous() {
        let keys = vec!["Alt+1".to_string(), "Alt+3".to_string(), "Alt+5".to_string()];
        assert_eq!(HelpPane::compact_keys("Go to tab", &keys), "Alt+1, Alt+3, Alt+5");
    }

    #[test]
    fn compact_keys_bare_digits_legacy_range() {
        let keys: Vec<String> = (1..=9).map(|n| n.to_string()).collect();
        assert_eq!(HelpPane::compact_keys("Go to tab", &keys), "1-9");
    }
}
