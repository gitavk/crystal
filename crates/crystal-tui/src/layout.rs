use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};

use crate::theme;

pub fn render_root(frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    render_header(frame, chunks[0]);
    render_body(frame, chunks[1]);
    render_status_bar(frame, chunks[2]);
}

fn render_header(frame: &mut Frame, area: Rect) {
    let header =
        Paragraph::new(" crystal — kubernetes IDE").style(Style::default().fg(theme::HEADER_FG).bg(theme::HEADER_BG));
    frame.render_widget(header, area);
}

fn render_body(frame: &mut Frame, area: Rect) {
    let body = Block::default().style(Style::default().bg(theme::BODY_BG));
    frame.render_widget(body, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect) {
    let hints = Paragraph::new(" q: quit").style(Style::default().fg(theme::STATUS_FG).bg(theme::STATUS_BG));
    frame.render_widget(hints, area);
}
