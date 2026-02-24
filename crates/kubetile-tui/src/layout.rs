use std::collections::HashMap;

use ratatui::prelude::*;

use crate::pane::{Pane, PaneId, PaneTree, ResourceKind};
use crate::theme::Theme;
use crate::widgets::confirm_dialog::ConfirmDialogWidget;
use crate::widgets::context_selector::ContextSelectorWidget;
use crate::widgets::namespace_selector::NamespaceSelectorWidget;
use crate::widgets::port_forward_dialog::PortForwardDialogWidget;
use crate::widgets::query_dialog::QueryDialogWidget;
use crate::widgets::resource_switcher::ResourceSwitcherWidget;
use crate::widgets::status_bar::StatusBarWidget;
use crate::widgets::tab_bar::TabBarWidget;
use crate::widgets::toast::{ToastMessage, ToastWidget};

pub struct NamespaceSelectorView<'a> {
    pub namespaces: &'a [String],
    pub filter: &'a str,
    pub selected: usize,
}

pub struct ContextSelectorView<'a> {
    pub contexts: &'a [String],
    pub filter: &'a str,
    pub selected: usize,
}

pub struct ResourceSwitcherView<'a> {
    pub input: &'a str,
    pub items: &'a [ResourceKind],
    pub selected: usize,
}

pub struct ConfirmDialogView<'a> {
    pub message: &'a str,
}

#[derive(Clone, Copy)]
pub enum PortForwardFieldView {
    Local,
    Remote,
}

#[derive(Clone, Copy)]
pub enum QueryDialogFieldView {
    Database,
    User,
    Password,
    Port,
}

pub struct PortForwardDialogView<'a> {
    pub pod: &'a str,
    pub namespace: &'a str,
    pub local_port: &'a str,
    pub remote_port: &'a str,
    pub active_field: PortForwardFieldView,
}

pub struct QueryDialogView<'a> {
    pub pod: &'a str,
    pub namespace: &'a str,
    pub database: &'a str,
    pub user: &'a str,
    pub password: &'a str,
    pub port: &'a str,
    pub active_field: QueryDialogFieldView,
}

pub struct RenderContext<'a> {
    pub cluster_name: Option<&'a str>,
    pub namespace: Option<&'a str>,
    pub namespace_selector: Option<NamespaceSelectorView<'a>>,
    pub context_selector: Option<ContextSelectorView<'a>>,
    pub resource_switcher: Option<ResourceSwitcherView<'a>>,
    pub confirm_dialog: Option<ConfirmDialogView<'a>>,
    pub port_forward_dialog: Option<PortForwardDialogView<'a>>,
    pub query_dialog: Option<QueryDialogView<'a>>,
    pub toasts: &'a [ToastMessage],
    pub pane_tree: &'a PaneTree,
    pub focused_pane: Option<PaneId>,
    pub fullscreen_pane: Option<PaneId>,
    pub panes: &'a HashMap<PaneId, Box<dyn Pane>>,
    pub tab_names: &'a [String],
    pub active_tab: usize,
    pub mode_name: &'a str,
    pub help_key: Option<&'a str>,
    pub namespace_key: Option<&'a str>,
    pub context_key: Option<&'a str>,
    pub close_pane_key: Option<&'a str>,
    pub new_tab_key: Option<&'a str>,
    pub quit_key: Option<&'a str>,
    pub theme: &'a Theme,
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
    let widget = TabBarWidget { tabs: ctx.tab_names, active: ctx.active_tab, theme: ctx.theme };
    widget.render(frame, area);
}

fn render_body(frame: &mut Frame, area: Rect, ctx: &RenderContext) {
    if let Some(fs_id) = ctx.fullscreen_pane {
        if let Some(pane) = ctx.panes.get(&fs_id) {
            pane.render(frame, area, true, ctx.theme);
        }
    } else {
        let pane_rects = ctx.pane_tree.layout(area);
        for (pane_id, pane_area) in &pane_rects {
            if let Some(pane) = ctx.panes.get(pane_id) {
                let focused = ctx.focused_pane == Some(*pane_id);
                pane.render(frame, *pane_area, focused, ctx.theme);
            }
        }
    }

    if let Some(ref ns) = ctx.namespace_selector {
        let widget = NamespaceSelectorWidget {
            namespaces: ns.namespaces,
            filter: ns.filter,
            selected: ns.selected,
            theme: ctx.theme,
        };
        widget.render(frame, area);
    }

    if let Some(ref cs) = ctx.context_selector {
        let widget =
            ContextSelectorWidget { contexts: cs.contexts, filter: cs.filter, selected: cs.selected, theme: ctx.theme };
        widget.render(frame, area);
    }

    if let Some(ref rs) = ctx.resource_switcher {
        let widget =
            ResourceSwitcherWidget { input: rs.input, items: rs.items, selected: rs.selected, theme: ctx.theme };
        widget.render(frame, area);
    }

    if let Some(ref cd) = ctx.confirm_dialog {
        let widget = ConfirmDialogWidget { message: cd.message, theme: ctx.theme };
        widget.render(frame, area);
    }

    if let Some(ref pf) = ctx.port_forward_dialog {
        let widget = PortForwardDialogWidget {
            pod: pf.pod,
            namespace: pf.namespace,
            local_port: pf.local_port,
            remote_port: pf.remote_port,
            active_field: pf.active_field,
            theme: ctx.theme,
        };
        widget.render(frame, area);
    }

    if let Some(ref qd) = ctx.query_dialog {
        let widget = QueryDialogWidget {
            pod: qd.pod,
            namespace: qd.namespace,
            database: qd.database,
            user: qd.user,
            password: qd.password,
            port: qd.port,
            active_field: qd.active_field,
            theme: ctx.theme,
        };
        widget.render(frame, area);
    }

    if !ctx.toasts.is_empty() {
        let widget = ToastWidget { toasts: ctx.toasts, theme: ctx.theme };
        widget.render(frame, area);
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, ctx: &RenderContext) {
    let widget = StatusBarWidget {
        mode: ctx.mode_name,
        context: ctx.cluster_name,
        help_key: ctx.help_key,
        namespace_key: ctx.namespace_key,
        context_key: ctx.context_key,
        close_pane_key: ctx.close_pane_key,
        new_tab_key: ctx.new_tab_key,
        quit_key: ctx.quit_key,
        theme: ctx.theme,
    };
    widget.render(frame, area);
}

#[cfg(test)]
mod tests;
