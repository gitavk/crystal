use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use kubetile_core::SavedQuery;

use super::QueryPane;

pub(super) struct QueryHistoryState {
    pub(super) entries: Vec<String>,
    pub(super) selected: usize,
}

pub(super) struct SavedQueriesState {
    pub(super) entries: Vec<SavedQuery>,
    pub(super) selected: usize,
    pub(super) filter_input: Option<String>,
    pub(super) rename_input: Option<String>,
}

fn saved_queries_filtered(sq: &SavedQueriesState) -> Vec<(usize, &SavedQuery)> {
    match &sq.filter_input {
        Some(f) if !f.is_empty() => {
            let f = f.to_lowercase();
            sq.entries.iter().enumerate().filter(|(_, e)| e.name.to_lowercase().contains(&f)).collect()
        }
        _ => sq.entries.iter().enumerate().collect(),
    }
}

impl QueryPane {
    // --- History ---

    pub fn open_history(&mut self, entries: Vec<String>) {
        self.history = Some(QueryHistoryState { entries, selected: 0 });
    }

    pub fn close_history(&mut self) {
        self.history = None;
    }

    pub fn history_next(&mut self) {
        if let Some(ref mut h) = self.history {
            if h.selected + 1 < h.entries.len() {
                h.selected += 1;
            }
        }
    }

    pub fn history_prev(&mut self) {
        if let Some(ref mut h) = self.history {
            h.selected = h.selected.saturating_sub(1);
        }
    }

    pub fn history_selected_sql(&self) -> Option<&str> {
        self.history.as_ref()?.entries.get(self.history.as_ref()?.selected).map(|s| s.as_str())
    }

    pub fn history_selected_index(&self) -> usize {
        self.history.as_ref().map(|h| h.selected).unwrap_or(0)
    }

    // --- Save-name dialog ---

    pub fn open_save_name(&mut self) {
        self.pending_save_name = Some(String::new());
    }

    pub fn close_save_name(&mut self) {
        self.pending_save_name = None;
    }

    pub fn save_name_input(&mut self, c: char) {
        if let Some(ref mut buf) = self.pending_save_name {
            buf.push(c);
        }
    }

    pub fn save_name_backspace(&mut self) {
        if let Some(ref mut buf) = self.pending_save_name {
            buf.pop();
        }
    }

    pub fn current_save_name(&self) -> Option<&str> {
        self.pending_save_name.as_deref()
    }

    // --- Saved-queries popup ---

    pub fn open_saved_queries(&mut self, entries: Vec<SavedQuery>) {
        self.saved_queries = Some(SavedQueriesState { entries, selected: 0, filter_input: None, rename_input: None });
    }

    pub fn close_saved_queries(&mut self) {
        self.saved_queries = None;
    }

    pub fn saved_queries_next(&mut self) {
        if let Some(ref mut sq) = self.saved_queries {
            let count = saved_queries_filtered(sq).len();
            if sq.selected + 1 < count {
                sq.selected += 1;
            }
        }
    }

    pub fn saved_queries_prev(&mut self) {
        if let Some(ref mut sq) = self.saved_queries {
            sq.selected = sq.selected.saturating_sub(1);
        }
    }

    pub fn saved_queries_start_filter(&mut self) {
        if let Some(ref mut sq) = self.saved_queries {
            sq.rename_input = None;
            sq.filter_input = Some(String::new());
            sq.selected = 0;
        }
    }

    pub fn saved_queries_start_rename(&mut self) {
        if let Some(ref mut sq) = self.saved_queries {
            let current_name =
                saved_queries_filtered(sq).get(sq.selected).map(|(_, e)| e.name.clone()).unwrap_or_default();
            sq.rename_input = Some(current_name);
        }
    }

    pub fn saved_queries_input(&mut self, c: char) {
        if let Some(ref mut sq) = self.saved_queries {
            if let Some(ref mut buf) = sq.rename_input {
                buf.push(c);
            } else if let Some(ref mut buf) = sq.filter_input {
                buf.push(c);
                sq.selected = 0;
            }
        }
    }

    pub fn saved_queries_backspace(&mut self) {
        if let Some(ref mut sq) = self.saved_queries {
            if let Some(ref mut buf) = sq.rename_input {
                buf.pop();
            } else if let Some(ref mut buf) = sq.filter_input {
                buf.pop();
                sq.selected = 0;
            }
        }
    }

    /// Returns false if neither sub-mode nor popup was open (caller should change mode to Normal).
    pub fn saved_queries_close_sub_mode(&mut self) -> bool {
        if let Some(ref mut sq) = self.saved_queries {
            if sq.rename_input.is_some() {
                sq.rename_input = None;
                return true;
            }
            if sq.filter_input.is_some() {
                sq.filter_input = None;
                sq.selected = 0;
                return true;
            }
            self.saved_queries = None;
            return false;
        }
        false
    }

    pub fn saved_queries_is_renaming(&self) -> bool {
        self.saved_queries.as_ref().is_some_and(|sq| sq.rename_input.is_some())
    }

    /// Returns `(real_index, name, sql)` for the currently selected entry.
    pub fn saved_queries_selected(&self) -> Option<(usize, &str, &str)> {
        let sq = self.saved_queries.as_ref()?;
        let filtered = saved_queries_filtered(sq);
        let (real_idx, entry) = filtered.get(sq.selected)?;
        Some((*real_idx, entry.name.as_str(), entry.sql.as_str()))
    }

    pub fn saved_queries_rename_input(&self) -> Option<&str> {
        self.saved_queries.as_ref()?.rename_input.as_deref()
    }

    // --- Export dialog ---

    pub fn open_export_dialog(&mut self, pre_filled: String) {
        self.export_dialog_path = Some(pre_filled);
    }

    pub fn close_export_dialog(&mut self) {
        self.export_dialog_path = None;
    }

    pub fn export_path_input(&mut self, c: char) {
        if let Some(ref mut buf) = self.export_dialog_path {
            buf.push(c);
        }
    }

    pub fn export_path_backspace(&mut self) {
        if let Some(ref mut buf) = self.export_dialog_path {
            buf.pop();
        }
    }

    pub fn current_export_path(&self) -> Option<&str> {
        self.export_dialog_path.as_deref()
    }
}

pub(super) fn render_history_popup(
    frame: &mut Frame,
    area: Rect,
    h: &QueryHistoryState,
    theme: &kubetile_tui::theme::Theme,
) {
    let popup_w = (area.width.saturating_sub(4)).min(area.width * 9 / 10).max(20);
    let popup_h = (area.height.saturating_sub(2)).min(area.height * 4 / 5).max(6);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_w)) / 2,
        y: area.y + (area.height.saturating_sub(popup_h)) / 2,
        width: popup_w,
        height: popup_h,
    };
    frame.render_widget(Clear, popup);

    let count = h.entries.len();
    let title = format!(" Query History ({count}) ");
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(theme.accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(theme.overlay);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 2 {
        return;
    }

    let hint_y = inner.y + inner.height.saturating_sub(1);
    let hint_area = Rect { x: inner.x, y: hint_y, width: inner.width, height: 1 };
    let list_area = Rect { height: inner.height.saturating_sub(1), ..inner };

    frame.render_widget(
        Paragraph::new("j/k navigate  Enter select  d delete  Esc cancel").style(theme.text_dim),
        hint_area,
    );

    let list_w = (list_area.width * 2 / 5).max(10);
    let preview_w = list_area.width.saturating_sub(list_w + 1);
    let left_area = Rect { width: list_w, ..list_area };
    let divider_area = Rect { x: list_area.x + list_w, y: list_area.y, width: 1, height: list_area.height };
    let right_area = Rect { x: list_area.x + list_w + 1, y: list_area.y, width: preview_w, height: list_area.height };

    let divider_lines: Vec<Line> = std::iter::repeat_n(Line::from("│"), list_area.height as usize).collect();
    frame.render_widget(Paragraph::new(divider_lines).style(theme.text_dim), divider_area);

    let visible = list_area.height as usize;
    let scroll = if h.selected >= visible { h.selected + 1 - visible } else { 0 };
    let list_lines: Vec<Line> = h
        .entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible)
        .map(|(i, sql)| {
            let first_line = sql.lines().next().unwrap_or("").chars().take(list_w as usize - 3).collect::<String>();
            let prefix = if i == h.selected { "> " } else { "  " };
            let text = format!("{prefix}{first_line}");
            let style = if i == h.selected { Style::default().fg(theme.accent).bold() } else { Style::default() };
            Line::from(Span::styled(text, style))
        })
        .collect();
    frame.render_widget(Paragraph::new(list_lines), left_area);

    if let Some(sql) = h.entries.get(h.selected) {
        let preview_lines: Vec<Line> = sql
            .lines()
            .flat_map(|line| {
                if line.is_empty() {
                    vec![Line::from("")]
                } else {
                    line.chars()
                        .collect::<Vec<_>>()
                        .chunks(preview_w as usize)
                        .map(|chunk| Line::from(chunk.iter().collect::<String>()))
                        .collect()
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(preview_lines).style(Style::default().fg(theme.fg)), right_area);
    }
}

pub(super) fn render_save_name_popup(
    frame: &mut Frame,
    area: Rect,
    name_buf: &str,
    theme: &kubetile_tui::theme::Theme,
) {
    let popup_w = (area.width.saturating_sub(4)).clamp(30, 60);
    let popup_h = 5u16;
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_w)) / 2,
        y: area.y + (area.height.saturating_sub(popup_h)) / 2,
        width: popup_w,
        height: popup_h,
    };
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Save Query ")
        .title_style(Style::default().fg(theme.accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(theme.overlay);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 3 {
        return;
    }

    let input_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 };
    let hint_area = Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(1), width: inner.width, height: 1 };

    let display_name: String = name_buf.chars().take(inner.width.saturating_sub(8) as usize).collect();
    let label = format!("Name: {display_name}▌");
    frame.render_widget(Paragraph::new(label).style(Style::default().fg(theme.accent)), input_area);
    frame.render_widget(Paragraph::new("Enter confirm  Esc cancel").style(theme.text_dim), hint_area);
}

pub(super) fn render_saved_queries_popup(
    frame: &mut Frame,
    area: Rect,
    sq: &SavedQueriesState,
    theme: &kubetile_tui::theme::Theme,
) {
    let popup_w = (area.width.saturating_sub(4)).min(area.width * 9 / 10).max(20);
    let popup_h = (area.height.saturating_sub(2)).min(area.height * 4 / 5).max(6);
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_w)) / 2,
        y: area.y + (area.height.saturating_sub(popup_h)) / 2,
        width: popup_w,
        height: popup_h,
    };
    frame.render_widget(Clear, popup);

    let filtered = saved_queries_filtered(sq);
    let count = sq.entries.len();
    let title = format!(" Saved Queries ({count}) ");
    let block = Block::default()
        .title(title)
        .title_style(Style::default().fg(theme.accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(theme.overlay);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 2 {
        return;
    }

    let hint_y = inner.y + inner.height.saturating_sub(1);
    let hint_area = Rect { x: inner.x, y: hint_y, width: inner.width, height: 1 };

    let (content_h, filter_h) = if sq.filter_input.is_some() {
        (inner.height.saturating_sub(2), 1u16)
    } else {
        (inner.height.saturating_sub(1), 0u16)
    };
    let content_area = Rect { height: content_h, ..inner };

    let hint_text = if sq.rename_input.is_some() {
        "Enter confirm  Esc cancel"
    } else if sq.filter_input.is_some() {
        "j/k nav  Enter load  d del  e rename  Esc clear filter"
    } else {
        "j/k nav  Enter load  d del  e rename  / filter  Esc close"
    };
    frame.render_widget(Paragraph::new(hint_text).style(theme.text_dim), hint_area);

    if let Some(ref filter) = sq.filter_input {
        let filter_area = Rect { x: inner.x, y: hint_y.saturating_sub(1), width: inner.width, height: 1 };
        let display_filter: String = filter.chars().take(inner.width.saturating_sub(10) as usize).collect();
        frame.render_widget(
            Paragraph::new(format!("Filter: {display_filter}▌")).style(Style::default().fg(theme.accent)),
            filter_area,
        );
    }
    let _ = filter_h;

    if content_area.height == 0 {
        return;
    }

    let list_w = (content_area.width * 2 / 5).max(10);
    let preview_w = content_area.width.saturating_sub(list_w + 1);
    let left_area = Rect { width: list_w, ..content_area };
    let divider_area = Rect { x: content_area.x + list_w, y: content_area.y, width: 1, height: content_area.height };
    let right_area =
        Rect { x: content_area.x + list_w + 1, y: content_area.y, width: preview_w, height: content_area.height };

    let divider_lines: Vec<Line> = std::iter::repeat_n(Line::from("│"), content_area.height as usize).collect();
    frame.render_widget(Paragraph::new(divider_lines).style(theme.text_dim), divider_area);

    let visible = content_area.height as usize;
    let scroll = if sq.selected >= visible { sq.selected + 1 - visible } else { 0 };
    let list_lines: Vec<Line> = filtered
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible)
        .map(|(fi, (_, entry))| {
            let is_selected = fi == sq.selected;
            let max_name_w = (list_w as usize).saturating_sub(3);
            let display_name = if is_selected {
                if let Some(ref rename_buf) = sq.rename_input {
                    let n: String = rename_buf.chars().take(max_name_w.saturating_sub(1)).collect();
                    format!("{n}▌")
                } else {
                    entry.name.chars().take(max_name_w).collect()
                }
            } else {
                entry.name.chars().take(max_name_w).collect()
            };
            let prefix = if is_selected { "> " } else { "  " };
            let text = format!("{prefix}{display_name}");
            let style = if is_selected { Style::default().fg(theme.accent).bold() } else { Style::default() };
            Line::from(Span::styled(text, style))
        })
        .collect();
    frame.render_widget(Paragraph::new(list_lines), left_area);

    if let Some((_, entry)) = filtered.get(sq.selected) {
        let preview_lines: Vec<Line> = entry
            .sql
            .lines()
            .flat_map(|line| {
                if line.is_empty() {
                    vec![Line::from("")]
                } else {
                    line.chars()
                        .collect::<Vec<_>>()
                        .chunks(preview_w as usize)
                        .map(|chunk| Line::from(chunk.iter().collect::<String>()))
                        .collect()
                }
            })
            .collect();
        frame.render_widget(Paragraph::new(preview_lines).style(Style::default().fg(theme.fg)), right_area);
    } else if filtered.is_empty() {
        frame.render_widget(Paragraph::new("No matches").style(theme.text_dim), left_area);
    }
}

pub(super) fn render_export_dialog_popup(
    frame: &mut Frame,
    area: Rect,
    path_buf: &str,
    theme: &kubetile_tui::theme::Theme,
) {
    let popup_w = (area.width.saturating_sub(4)).clamp(30, 70);
    let popup_h = 5u16;
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(popup_w)) / 2,
        y: area.y + (area.height.saturating_sub(popup_h)) / 2,
        width: popup_w,
        height: popup_h,
    };
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Export to File ")
        .title_style(Style::default().fg(theme.accent).bold())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .style(theme.overlay);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if inner.height < 3 {
        return;
    }

    let input_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 };
    let hint_area = Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(1), width: inner.width, height: 1 };

    let prefix = "Path: ";
    let max_path_w = inner.width.saturating_sub(prefix.len() as u16) as usize;
    let display = if path_buf.len() > max_path_w {
        format!("…{}", &path_buf[path_buf.len().saturating_sub(max_path_w.saturating_sub(1))..])
    } else {
        path_buf.to_string()
    };
    let label = format!("{prefix}{display}");
    frame.render_widget(Paragraph::new(label).style(Style::default().fg(theme.accent)), input_area);
    frame.render_widget(Paragraph::new("Enter confirm  Esc cancel").style(theme.text_dim), hint_area);
}
