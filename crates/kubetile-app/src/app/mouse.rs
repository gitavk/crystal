use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::Rect;

use kubetile_tui::pane::{PaneCommand, PaneId, ViewType};

use crate::command::InputMode;

use super::App;

impl App {
    /// Infer and enter the appropriate InputMode for whichever pane is currently
    /// focused.  Called after any mouse-driven focus change so the user can
    /// start interacting immediately without pressing an extra key.
    fn set_mode_for_focused_pane(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let mode = self
            .panes
            .get(&focused)
            .map(|pane| match pane.view_type() {
                ViewType::Query(_) => InputMode::QueryEditor,
                ViewType::Exec(_) | ViewType::Terminal => InputMode::Insert,
                _ => InputMode::Normal,
            })
            .unwrap_or(InputMode::Normal);
        self.dispatcher.set_mode(mode);
    }

    pub(super) fn update_mouse_geometry(&mut self, area: Rect) {
        // Mirror the three-zone split from render_root
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
            .split(area);

        let tab_area = chunks[0];
        let body_area = chunks[1];
        let status_area = chunks[2];

        self.mouse_tab_bar_row = tab_area.y;
        self.mouse_status_bar_row = status_area.y;

        // Pane rects — same logic as render_body
        let tab = self.tab_manager.active();
        if let Some(fs_id) = tab.fullscreen_pane {
            self.mouse_pane_rects = vec![(fs_id, body_area)];
        } else {
            self.mouse_pane_rects = tab.pane_tree.layout(body_area);
        }

        // Tab spans — replicate TabBarWidget rendering positions
        let tab_names = self.tab_manager.tab_names();
        let sep_w: usize = 3; // " │ "
        let labels: Vec<String> =
            tab_names.iter().enumerate().map(|(i, name)| format!("[{}] {}", i + 1, name)).collect();
        let widths: Vec<usize> = labels.iter().map(|l| l.len()).collect();
        let active = self.tab_manager.active_index();
        let scroll = tab_scroll_offset(&widths, sep_w, tab_area.width as usize, active);

        let mut spans: Vec<(usize, u16, u16)> = Vec::new();
        let mut x: u16 = tab_area.x;
        let mut first = true;
        for (i, _) in labels.iter().enumerate() {
            if i < scroll {
                continue;
            }
            if !first {
                x += sep_w as u16;
            }
            first = false;
            let w = widths[i] as u16;
            spans.push((i, x, x + w));
            x += w;
        }
        self.mouse_tab_spans = spans;
    }

    pub(super) fn handle_mouse(&mut self, event: MouseEvent) {
        let col = event.column;
        let row = event.row;
        let mode = self.dispatcher.mode();

        // Global modal overlays capture all keyboard input; don't let mouse
        // silently change focus behind them.
        if is_modal_mode(mode) {
            return;
        }

        match event.kind {
            // A1: Click-to-focus pane / A2: Click tab to switch
            MouseEventKind::Down(MouseButton::Left) => {
                if row == self.mouse_tab_bar_row {
                    // A2: Tab bar click
                    for &(tab_idx, x_start, x_end) in &self.mouse_tab_spans {
                        if col >= x_start && col < x_end {
                            let current_idx = self.tab_manager.active_index();
                            if tab_idx != current_idx {
                                self.switch_to_tab_index(tab_idx);
                                self.set_mode_for_focused_pane();
                            }
                            return;
                        }
                    }
                } else if row != self.mouse_status_bar_row {
                    // A1: Pane focus
                    if let Some(pane_id) = pane_at(&self.mouse_pane_rects, col, row) {
                        let current = self.tab_manager.active().focused_pane;
                        if pane_id != current {
                            self.tab_manager.active_mut().focused_pane = pane_id;
                            self.set_mode_for_focused_pane();
                        }
                    }
                }
            }

            // A4: Middle-click to close tab or pane
            MouseEventKind::Down(MouseButton::Middle) => {
                if row == self.mouse_tab_bar_row {
                    for &(tab_idx, x_start, x_end) in &self.mouse_tab_spans {
                        if col >= x_start && col < x_end {
                            self.switch_to_tab_index(tab_idx);
                            self.close_tab();
                            // after close, infer mode for whatever is now focused
                            self.set_mode_for_focused_pane();
                            return;
                        }
                    }
                } else if row != self.mouse_status_bar_row {
                    if let Some(pane_id) = pane_at(&self.mouse_pane_rects, col, row) {
                        self.tab_manager.active_mut().focused_pane = pane_id;
                        self.close_focused();
                        self.set_mode_for_focused_pane();
                    }
                }
            }

            // A3: Scroll wheel — dispatch to hovered pane regardless of focus
            MouseEventKind::ScrollUp => {
                if let Some(pane_id) = pane_at(&self.mouse_pane_rects, col, row) {
                    if let Some(pane) = self.panes.get_mut(&pane_id) {
                        pane.handle_command(&PaneCommand::ScrollUp);
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                if let Some(pane_id) = pane_at(&self.mouse_pane_rects, col, row) {
                    if let Some(pane) = self.panes.get_mut(&pane_id) {
                        pane.handle_command(&PaneCommand::ScrollDown);
                    }
                }
            }

            _ => {}
        }
    }
}

/// Modes that show a blocking overlay — mouse focus changes must be ignored.
fn is_modal_mode(mode: InputMode) -> bool {
    matches!(
        mode,
        InputMode::ConfirmDialog
            | InputMode::PortForwardInput
            | InputMode::QueryDialog
            | InputMode::NamespaceSelector
            | InputMode::ContextSelector
            | InputMode::ResourceSwitcher
    )
}

fn pane_at(rects: &[(PaneId, Rect)], col: u16, row: u16) -> Option<PaneId> {
    rects.iter().find(|(_, rect)| rect_contains(rect, col, row)).map(|(id, _)| *id)
}

fn rect_contains(rect: &Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

/// Mirror of TabBarWidget::compute_scroll — returns the first visible tab index.
fn tab_scroll_offset(widths: &[usize], sep_w: usize, max_w: usize, active: usize) -> usize {
    if widths.is_empty() {
        return 0;
    }
    let total: usize = widths.iter().sum::<usize>() + sep_w * widths.len().saturating_sub(1);
    if total <= max_w {
        return 0;
    }
    let mut scroll = 0;
    loop {
        let visible: usize = widths[scroll..].iter().sum::<usize>() + sep_w * widths[scroll..].len().saturating_sub(1);
        if visible <= max_w {
            break;
        }
        if scroll >= active {
            break;
        }
        scroll += 1;
    }
    scroll
}
