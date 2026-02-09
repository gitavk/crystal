use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme;
use crate::widgets::namespace_selector::NamespaceSelectorWidget;
use crate::widgets::resource_list::ResourceListWidget;

pub struct ResourceListView<'a> {
    pub title: &'a str,
    pub headers: &'a [String],
    pub items: &'a [Vec<String>],
    pub selected: Option<usize>,
    pub scroll_offset: usize,
    pub loading: bool,
    pub error: Option<&'a str>,
}

pub struct NamespaceSelectorView<'a> {
    pub namespaces: &'a [String],
    pub filter: &'a str,
    pub selected: usize,
}

pub struct RenderContext<'a> {
    pub cluster_name: Option<&'a str>,
    pub namespace: Option<&'a str>,
    pub resource_list: Option<ResourceListView<'a>>,
    pub namespace_selector: Option<NamespaceSelectorView<'a>>,
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
    if let Some(ref list) = ctx.resource_list {
        let widget = ResourceListWidget {
            title: list.title,
            headers: list.headers,
            items: list.items,
            selected: list.selected,
            scroll_offset: list.scroll_offset,
            loading: list.loading,
            error: list.error,
        };
        widget.render(frame, area);
    }

    if let Some(ref ns) = ctx.namespace_selector {
        let widget = NamespaceSelectorWidget { namespaces: ns.namespaces, filter: ns.filter, selected: ns.selected };
        widget.render(frame, area);
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, ctx: &RenderContext) {
    let cluster = ctx.cluster_name.unwrap_or("no cluster");
    let ns = ctx.namespace.unwrap_or("n/a");
    let text = format!(" {cluster} | ns:{ns}  │  j/k:navigate  :::namespace  1:pods  q:quit");
    let bar = Paragraph::new(text).style(Style::default().fg(theme::STATUS_FG).bg(theme::STATUS_BG));
    frame.render_widget(bar, area);
}
