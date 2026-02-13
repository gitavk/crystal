use std::collections::HashMap;

use ratatui::prelude::*;

use crate::pane::{Pane, PaneId, PaneTree, ResourceKind};
use crate::widgets::namespace_selector::NamespaceSelectorWidget;
use crate::widgets::resource_switcher::ResourceSwitcherWidget;
use crate::widgets::status_bar::StatusBarWidget;
use crate::widgets::tab_bar::TabBarWidget;

pub struct NamespaceSelectorView<'a> {
    pub namespaces: &'a [String],
    pub filter: &'a str,
    pub selected: usize,
}

pub struct ResourceSwitcherView<'a> {
    pub input: &'a str,
    pub items: &'a [ResourceKind],
    pub selected: usize,
}

pub struct RenderContext<'a> {
    pub cluster_name: Option<&'a str>,
    pub namespace: Option<&'a str>,
    pub namespace_selector: Option<NamespaceSelectorView<'a>>,
    pub resource_switcher: Option<ResourceSwitcherView<'a>>,
    pub pane_tree: &'a PaneTree,
    pub focused_pane: Option<PaneId>,
    pub fullscreen_pane: Option<PaneId>,
    pub panes: &'a HashMap<PaneId, Box<dyn Pane>>,
    pub tab_names: &'a [String],
    pub active_tab: usize,
    pub mode_name: &'a str,
    pub mode_hints: &'a [(String, String)],
}

pub fn render_root(frame: &mut Frame, ctx: &RenderContext) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    render_tab_bar(frame, chunks[0], ctx);
    render_body(frame, chunks[1], ctx);
    render_status_bar(frame, chunks[2], ctx);
}

fn render_tab_bar(frame: &mut Frame, area: Rect, ctx: &RenderContext) {
    let widget = TabBarWidget { tabs: ctx.tab_names, active: ctx.active_tab };
    widget.render(frame, area);
}

fn render_body(frame: &mut Frame, area: Rect, ctx: &RenderContext) {
    if let Some(fs_id) = ctx.fullscreen_pane {
        if let Some(pane) = ctx.panes.get(&fs_id) {
            pane.render(frame, area, true);
        }
    } else {
        let pane_rects = ctx.pane_tree.layout(area);
        for (pane_id, pane_area) in &pane_rects {
            if let Some(pane) = ctx.panes.get(pane_id) {
                let focused = ctx.focused_pane == Some(*pane_id);
                pane.render(frame, *pane_area, focused);
            }
        }
    }

    if let Some(ref ns) = ctx.namespace_selector {
        let widget = NamespaceSelectorWidget { namespaces: ns.namespaces, filter: ns.filter, selected: ns.selected };
        widget.render(frame, area);
    }

    if let Some(ref rs) = ctx.resource_switcher {
        let widget = ResourceSwitcherWidget { input: rs.input, items: rs.items, selected: rs.selected };
        widget.render(frame, area);
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, ctx: &RenderContext) {
    let widget = StatusBarWidget {
        mode: ctx.mode_name,
        hints: ctx.mode_hints,
        cluster: ctx.cluster_name,
        namespace: ctx.namespace,
    };
    widget.render(frame, area);
}
