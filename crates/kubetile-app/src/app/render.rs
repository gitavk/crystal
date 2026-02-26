use kubetile_tui::layout::{
    ConfirmDialogView, ContextSelectorView, NamespaceSelectorView, PortForwardDialogView, PortForwardFieldView,
    QueryDialogFieldView, QueryDialogView, RenderContext, ResourceSwitcherView,
};
use kubetile_tui::pane::{ResourceKind, ViewType};

use crate::command::InputMode;
use crate::panes::ResourceListPane;

use super::{App, PortForwardField, QueryDialogField};

impl App {
    pub(super) fn mode_name(&self) -> &'static str {
        match self.dispatcher.mode() {
            InputMode::Normal => "Normal",
            InputMode::NamespaceSelector => "Namespace",
            InputMode::ContextSelector => "Context",
            InputMode::Pane => "Pane",
            InputMode::Tab => "Tab",
            InputMode::Search => "Search",
            InputMode::Command => "Command",
            InputMode::Insert => "Insert",
            InputMode::ResourceSwitcher => "Resource",
            InputMode::ConfirmDialog => "Confirm",
            InputMode::FilterInput => "Filter",
            InputMode::PortForwardInput => "PortForward",
            InputMode::QueryDialog => "QueryDialog",
            InputMode::QueryEditor => "QueryEditor",
            InputMode::QueryBrowse => "QueryBrowse",
            InputMode::QueryHistory => "QueryHistory",
        }
    }

    pub(super) fn build_render_context(&self) -> (RenderContext<'_>, Vec<String>, [Option<String>; 6]) {
        let namespace_selector = if self.dispatcher.mode() == InputMode::NamespaceSelector {
            Some(NamespaceSelectorView {
                namespaces: &self.namespaces,
                filter: &self.namespace_filter,
                selected: self.namespace_selected,
            })
        } else {
            None
        };
        let context_selector = if self.dispatcher.mode() == InputMode::ContextSelector {
            Some(ContextSelectorView {
                contexts: &self.contexts,
                filter: &self.context_filter,
                selected: self.context_selected,
            })
        } else {
            None
        };

        let resource_switcher = self.resource_switcher.as_ref().map(|sw| ResourceSwitcherView {
            input: sw.input(),
            items: sw.filtered(),
            selected: sw.selected(),
        });

        let confirm_dialog = self.pending_confirmation.as_ref().map(|pc| ConfirmDialogView { message: &pc.message });
        let query_dialog = self.pending_query_dialog.as_ref().map(|qd| QueryDialogView {
            pod: &qd.pod,
            namespace: &qd.namespace,
            database: &qd.db_input,
            user: &qd.user_input,
            password: &qd.password_input,
            port: &qd.port_input,
            active_field: match qd.active_field {
                QueryDialogField::Database => QueryDialogFieldView::Database,
                QueryDialogField::User => QueryDialogFieldView::User,
                QueryDialogField::Password => QueryDialogFieldView::Password,
                QueryDialogField::Port => QueryDialogFieldView::Port,
            },
        });
        let port_forward_dialog = self.pending_port_forward.as_ref().map(|pf| PortForwardDialogView {
            pod: &pf.pod,
            namespace: &pf.namespace,
            local_port: &pf.local_input,
            remote_port: &pf.remote_input,
            active_field: match pf.active_field {
                PortForwardField::Local => PortForwardFieldView::Local,
                PortForwardField::Remote => PortForwardFieldView::Remote,
            },
        });

        let tab_names = self.tab_manager.tab_names();
        let keys = [
            self.dispatcher.key_for("help"),
            self.dispatcher.key_for("namespace_selector"),
            self.dispatcher.key_for("context_selector"),
            self.dispatcher.key_for("close_pane"),
            self.dispatcher.key_for("new_tab"),
            self.dispatcher.key_for("quit"),
        ];

        let tab = self.tab_manager.active();
        let (pane_tree, focused_pane, fullscreen_pane) = (&tab.pane_tree, tab.focused_pane, tab.fullscreen_pane);

        let ctx = RenderContext {
            cluster_name: self.context_resolver.context_name(),
            namespace: self.context_resolver.namespace(),
            namespace_selector,
            context_selector,
            resource_switcher,
            confirm_dialog,
            port_forward_dialog,
            query_dialog,
            toasts: &self.toasts,
            pane_tree,
            focused_pane: Some(focused_pane),
            fullscreen_pane,
            panes: &self.panes,
            tab_names: &[],
            active_tab: self.tab_manager.active_index(),
            mode_name: self.mode_name(),
            help_key: None,
            namespace_key: None,
            context_key: None,
            close_pane_key: None,
            new_tab_key: None,
            quit_key: None,
            theme: &self.theme,
        };

        (ctx, tab_names, keys)
    }

    pub(super) fn update_active_tab_title(&mut self) {
        let tab_id = self.tab_manager.active().id;
        let ns = self.active_namespace_label();
        let alias = self.active_view_alias();
        let title = format!("{ns}|{alias}");
        self.tab_manager.rename_tab(tab_id, &title);
    }

    fn active_namespace_label(&self) -> String {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get(&focused) {
            if let Some(rp) = pane.as_any().downcast_ref::<ResourceListPane>() {
                if rp.all_namespaces {
                    return "*".into();
                }
            }
        }
        let ns = self.context_resolver.namespace().unwrap_or("n/a");
        if ns.len() > 25 {
            format!("{}…", &ns[..24])
        } else {
            ns.to_string()
        }
    }

    fn active_view_alias(&self) -> String {
        let focused = self.tab_manager.active().focused_pane;
        let Some(pane) = self.panes.get(&focused) else { return "UNK".into() };
        match pane.view_type() {
            ViewType::ResourceList(kind) => resource_alias(kind),
            ViewType::Detail(kind, _) => resource_alias(kind),
            ViewType::Yaml(kind, _) => resource_alias(kind),
            ViewType::Logs(_) => "LOG".into(),
            ViewType::Exec(_) => "EXE".into(),
            ViewType::Terminal => "TER".into(),
            ViewType::Help => "HLP".into(),
            ViewType::Empty => "EMP".into(),
            ViewType::Plugin(name) if name == "AppLogs" => "ALG".into(),
            ViewType::Plugin(_) => "PLG".into(),
            ViewType::Query(_) => "SQL".into(),
        }
    }
}

fn resource_alias(kind: &ResourceKind) -> String {
    match kind {
        ResourceKind::Pods => "POD".into(),
        ResourceKind::Deployments => "DEP".into(),
        ResourceKind::Services => "SVC".into(),
        ResourceKind::StatefulSets => "STS".into(),
        ResourceKind::DaemonSets => "DMS".into(),
        ResourceKind::Jobs => "JOB".into(),
        ResourceKind::CronJobs => "CRN".into(),
        ResourceKind::ConfigMaps => "CFG".into(),
        ResourceKind::Secrets => "SEC".into(),
        ResourceKind::Ingresses => "ING".into(),
        ResourceKind::Nodes => "NOD".into(),
        ResourceKind::Namespaces => "NSP".into(),
        ResourceKind::PersistentVolumes => "PVS".into(),
        ResourceKind::PersistentVolumeClaims => "PVC".into(),
        ResourceKind::Custom(name) => {
            let up = name.to_uppercase();
            up.chars().take(3).collect()
        }
    }
}
