use std::collections::HashMap;

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::pane::{Pane, PaneId, PaneTree};
use crate::theme;
use crate::widgets::namespace_selector::NamespaceSelectorWidget;

pub struct NamespaceSelectorView<'a> {
    pub namespaces: &'a [String],
    pub filter: &'a str,
    pub selected: usize,
}

pub struct RenderContext<'a> {
    pub cluster_name: Option<&'a str>,
    pub namespace: Option<&'a str>,
    pub namespace_selector: Option<NamespaceSelectorView<'a>>,
    pub pane_tree: &'a PaneTree,
    pub focused_pane: Option<PaneId>,
    pub panes: &'a HashMap<PaneId, Box<dyn Pane>>,
}

pub fn render_root(frame: &mut Frame, ctx: &RenderContext) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    render_header(frame, chunks[0]);
    render_body(frame, chunks[1], ctx);
    render_status_bar(frame, chunks[2], ctx);
}

fn render_header(frame: &mut Frame, area: Rect) {
    let header =
        Paragraph::new(" crystal — kubernetes IDE").style(Style::default().fg(theme::HEADER_FG).bg(theme::HEADER_BG));
    frame.render_widget(header, area);
}

fn render_body(frame: &mut Frame, area: Rect, ctx: &RenderContext) {
    let pane_rects = ctx.pane_tree.layout(area);
    for (pane_id, pane_area) in &pane_rects {
        if let Some(pane) = ctx.panes.get(pane_id) {
            let focused = ctx.focused_pane == Some(*pane_id);
            pane.render(frame, *pane_area, focused);
        }
    }

    if let Some(ref ns) = ctx.namespace_selector {
        let widget = NamespaceSelectorWidget { namespaces: ns.namespaces, filter: ns.filter, selected: ns.selected };
        widget.render(frame, area);
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, ctx: &RenderContext) {
    let cluster = ctx.cluster_name.unwrap_or("no cluster");
    let ns = ctx.namespace.unwrap_or("n/a");
    let text = format!(" {cluster} | ns:{ns}  │  j/k:navigate  :::namespace  ?:help  Tab:focus  q:quit");
    let bar = Paragraph::new(text).style(Style::default().fg(theme::STATUS_FG).bg(theme::STATUS_BG));
    frame.render_widget(bar, area);
}
