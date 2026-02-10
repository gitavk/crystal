use std::any::Any;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crystal_tui::pane::{Pane, PaneCommand, ViewType};
use crystal_tui::theme;

pub struct HelpPane {
    context_view: Option<ViewType>,
    scroll_offset: u16,
}

impl HelpPane {
    pub fn new() -> Self {
        Self { context_view: None, scroll_offset: 0 }
    }

    fn global_shortcuts() -> Vec<(&'static str, &'static str)> {
        vec![
            ("q", "Quit"),
            ("?", "Toggle help"),
            ("Tab", "Focus next pane"),
            ("Shift+Tab", "Focus previous pane"),
            ("Alt+v", "Split vertical"),
            ("Alt+h", "Split horizontal"),
            ("Alt+w", "Close pane"),
            (":", "Namespace selector"),
        ]
    }

    fn pane_shortcuts(view: &ViewType) -> Vec<(&'static str, &'static str)> {
        match view {
            ViewType::ResourceList(_) => vec![
                ("j / Down", "Select next"),
                ("k / Up", "Select previous"),
                ("Enter", "Select / confirm"),
                ("Esc", "Back"),
            ],
            ViewType::Logs(_) => vec![("j / Down", "Scroll down"), ("k / Up", "Scroll up"), ("f", "Toggle follow")],
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
        for (key, desc) in Self::global_shortcuts() {
            lines.push(Line::from(vec![
                Span::styled(format!("  {key:<16}"), Style::default().fg(theme::HEADER_FG).bold()),
                Span::styled(desc, Style::default().fg(theme::STATUS_FG)),
            ]));
        }

        if let Some(ref view) = self.context_view {
            let pane_shortcuts = Self::pane_shortcuts(view);
            if !pane_shortcuts.is_empty() {
                let label = match view {
                    ViewType::ResourceList(_) => "Resource List",
                    ViewType::Logs(_) => "Logs",
                    ViewType::Terminal => "Terminal",
                    _ => "Pane",
                };
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("{label} Shortcuts"),
                    Style::default().fg(theme::ACCENT).bold(),
                )));
                lines.push(Line::from(""));
                for (key, desc) in pane_shortcuts {
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {key:<16}"), Style::default().fg(theme::HEADER_FG).bold()),
                        Span::styled(desc, Style::default().fg(theme::STATUS_FG)),
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
