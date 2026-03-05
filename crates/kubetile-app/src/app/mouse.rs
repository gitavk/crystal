use std::time::{Duration, Instant};

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::Rect;

use kubetile_tui::pane::{PaneCommand, PaneId, ViewType};

use crate::command::InputMode;

use super::App;

pub(super) struct LastClick {
    pub col: u16,
    pub row: u16,
    pub at: Instant,
}

/// (pane_id, header_row_y, col_spans)
pub(super) type HeaderEntry = (PaneId, u16, Vec<(u16, u16)>);

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

        // B1: list-pane row geometry — populated after each render
        self.mouse_list_rows.clear();
        self.mouse_list_headers.clear();
        self.mouse_selection_targets.clear();
        for &(pane_id, _) in &self.mouse_pane_rects.clone() {
            if let Some(pane) = self.panes.get(&pane_id) {
                if let Some((data_rect, first_row)) = pane.list_row_geometry() {
                    self.mouse_list_rows.push((pane_id, data_rect, first_row));
                }
                if let Some((header_y, col_spans)) = pane.list_header_geometry() {
                    self.mouse_list_headers.push((pane_id, header_y, col_spans));
                }
                if let Some((data_rect, first_row)) = pane.text_selection_geometry() {
                    self.mouse_selection_targets.push((pane_id, data_rect, first_row));
                }
            }
        }
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
            // D1: Drag extends row-range selection
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(drag_pane_id) = self.mouse_drag_selection_pane {
                    let targets = self.mouse_selection_targets.clone();
                    for (pane_id, data_rect, first_row) in targets {
                        if pane_id != drag_pane_id || !rect_contains(&data_rect, col, row) {
                            continue;
                        }
                        let abs_row = (row - data_rect.y) as usize + first_row;
                        if let Some(pane) = self.panes.get_mut(&pane_id) {
                            pane.handle_command(&PaneCommand::SelectionExtendRow(abs_row));
                        }
                        break;
                    }
                }
            }

            // D1: Release finalises selection; show copy hint if anything selected
            MouseEventKind::Up(MouseButton::Left) => {
                let drag_pane_id = self.mouse_drag_selection_pane.take();
                if let Some(pane_id) = drag_pane_id {
                    if self.panes.get(&pane_id).is_some_and(|p| p.has_selection()) {
                        self.toasts.push(kubetile_tui::widgets::toast::ToastMessage::info(
                            "y  to copy selection  ·  Esc  to clear",
                        ));
                    }
                }
            }

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

                    // D1: Text-selection anchor — clear old drag pane if clicking elsewhere
                    if let Some(old_pane) = self.mouse_drag_selection_pane {
                        let still_in_target = self
                            .mouse_selection_targets
                            .iter()
                            .any(|(id, rect, _)| *id == old_pane && rect_contains(rect, col, row));
                        if !still_in_target {
                            if let Some(pane) = self.panes.get_mut(&old_pane) {
                                pane.handle_command(&PaneCommand::ClearSelection);
                            }
                            self.mouse_drag_selection_pane = None;
                        }
                    }
                    // D1: Start new drag selection if click lands in a selection target
                    let sel_targets = self.mouse_selection_targets.clone();
                    for (pane_id, data_rect, first_row) in sel_targets {
                        if !rect_contains(&data_rect, col, row) {
                            continue;
                        }
                        let abs_row = (row - data_rect.y) as usize + first_row;
                        self.tab_manager.active_mut().focused_pane = pane_id;
                        // Enter QueryBrowse when the selection target is a query result pane
                        if self.panes.get(&pane_id).is_some_and(|p| matches!(p.view_type(), ViewType::Query(_))) {
                            self.dispatcher.set_mode(InputMode::QueryBrowse);
                        }
                        if let Some(pane) = self.panes.get_mut(&pane_id) {
                            pane.handle_command(&PaneCommand::SelectionAnchorRow(abs_row));
                        }
                        self.mouse_drag_selection_pane = Some(pane_id);
                        return;
                    }

                    // C2: Column header click → sort
                    let list_headers = self.mouse_list_headers.clone();
                    for (pane_id, header_y, col_spans) in list_headers {
                        if row != header_y {
                            continue;
                        }
                        if pane_at(&self.mouse_pane_rects, col, row).is_none_or(|id| id != pane_id) {
                            continue;
                        }
                        self.tab_manager.active_mut().focused_pane = pane_id;
                        self.set_mode_for_focused_pane();
                        if let Some(col_idx) =
                            col_spans.iter().position(|&(x_start, x_end)| col >= x_start && col < x_end)
                        {
                            if let Some(pane) = self.panes.get_mut(&pane_id) {
                                pane.handle_command(&PaneCommand::SortByColumn(col_idx));
                            }
                        }
                        return;
                    }

                    // B1: Row click inside a list pane
                    let list_rows = self.mouse_list_rows.clone();
                    for (pane_id, data_rect, first_row) in list_rows {
                        if !rect_contains(&data_rect, col, row) {
                            continue;
                        }
                        self.tab_manager.active_mut().focused_pane = pane_id;
                        self.set_mode_for_focused_pane();

                        let display_row = (row - data_rect.y) as usize + first_row;

                        let now = Instant::now();
                        let is_double = self.mouse_last_click.as_ref().is_some_and(|lc| {
                            lc.col == col && lc.row == row && now.duration_since(lc.at) < Duration::from_millis(400)
                        });
                        self.mouse_last_click = Some(LastClick { col, row, at: now });

                        if let Some(pane) = self.panes.get_mut(&pane_id) {
                            pane.handle_command(&PaneCommand::SelectDisplayRow(display_row));
                        }
                        if is_double {
                            self.handle_command(crate::command::Command::Pane(PaneCommand::Select));
                        }
                        return;
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
