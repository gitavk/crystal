use kubetile_core::{QueryConfig, QueryResult};
use kubetile_tui::pane::{PaneId, ResourceKind, SplitDirection, ViewType};
use kubetile_tui::widgets::toast::ToastMessage;

use crate::command::InputMode;
use crate::event::AppEvent;
use crate::panes::QueryPane;

use super::{App, PendingQueryDialog, QueryDialogField};

impl App {
    pub(super) fn open_query_pane_for_selected(&mut self) {
        let Some((kind, pod, namespace)) = self.selected_resource_info() else {
            return;
        };
        if kind != ResourceKind::Pods {
            self.toasts.push(ToastMessage::info("Query is only available for Pods"));
            return;
        }

        let Some(client) = &self.kube_client else {
            self.toasts.push(ToastMessage::error("No cluster connection"));
            return;
        };
        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();

        tokio::spawn(async move {
            let config = kubetile_core::query::read_postgres_env(&kube_client, &pod, &namespace).await;
            let _ = app_tx.send(AppEvent::QueryPromptReady { config });
        });
    }

    pub(super) fn open_query_dialog(&mut self, config: QueryConfig) {
        self.pending_query_dialog = Some(PendingQueryDialog {
            pod: config.pod,
            namespace: config.namespace,
            db_input: config.database,
            user_input: config.user,
            password_input: config.password,
            port_input: config.port,
            active_field: QueryDialogField::Database,
        });
        self.dispatcher.set_mode(InputMode::QueryDialog);
    }

    pub(super) fn cancel_query_dialog(&mut self) {
        self.pending_query_dialog = None;
        self.dispatcher.set_mode(InputMode::Normal);
    }

    pub(super) fn confirm_query_dialog(&mut self) {
        let Some(pending) = self.pending_query_dialog.take() else {
            return;
        };
        self.dispatcher.set_mode(InputMode::Normal);

        let config = QueryConfig {
            pod: pending.pod,
            namespace: pending.namespace,
            database: pending.db_input,
            user: pending.user_input,
            password: pending.password_input,
            port: pending.port_input,
        };

        let pane = QueryPane::new(&config);
        let focused = self.tab_manager.active().focused_pane;
        let view = ViewType::Query(config.pod.clone());
        let Some(new_id) = self.tab_manager.split_pane_with_ratio(focused, SplitDirection::Horizontal, view, 0.7)
        else {
            return;
        };
        self.panes.insert(new_id, Box::new(pane));
        self.set_focus(new_id);
        self.dispatcher.set_mode(InputMode::QueryEditor);

        self.execute_query_for_pane(new_id, config);
    }

    fn execute_query_for_pane(&self, pane_id: PaneId, config: QueryConfig) {
        let Some(client) = &self.kube_client else {
            return;
        };
        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();

        tokio::spawn(async move {
            let event = match kubetile_core::query::execute_query(&kube_client, &config, "SELECT version()").await {
                Ok(result) => AppEvent::QueryReady { pane_id, result },
                Err(e) => AppEvent::QueryError { pane_id, error: e.to_string() },
            };
            let _ = app_tx.send(event);
        });
    }

    pub(super) fn handle_query_ready(&mut self, pane_id: PaneId, result: QueryResult) {
        let version = result
            .rows
            .first()
            .and_then(|row| row.first())
            .map(|s| extract_pg_version(s))
            .unwrap_or_else(|| "PostgreSQL".to_string());

        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.set_connected(version);
            }
        }
    }

    pub(super) fn handle_query_error(&mut self, pane_id: PaneId, error: String) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.set_error(error);
            }
        }
    }

    pub(super) fn query_dialog_input(&mut self, c: char) {
        let Some(ref mut pending) = self.pending_query_dialog else {
            return;
        };
        match pending.active_field {
            QueryDialogField::Database => pending.db_input.push(c),
            QueryDialogField::User => pending.user_input.push(c),
            QueryDialogField::Password => pending.password_input.push(c),
            QueryDialogField::Port => pending.port_input.push(c),
        }
    }

    pub(super) fn query_dialog_backspace(&mut self) {
        let Some(ref mut pending) = self.pending_query_dialog else {
            return;
        };
        match pending.active_field {
            QueryDialogField::Database => {
                pending.db_input.pop();
            }
            QueryDialogField::User => {
                pending.user_input.pop();
            }
            QueryDialogField::Password => {
                pending.password_input.pop();
            }
            QueryDialogField::Port => {
                pending.port_input.pop();
            }
        }
    }

    pub(super) fn query_dialog_next_field(&mut self) {
        if let Some(ref mut pending) = self.pending_query_dialog {
            pending.active_field = pending.active_field.next();
        }
    }
}

fn extract_pg_version(version_str: &str) -> String {
    // "PostgreSQL 15.2 on x86_64-pc-linux-gnu..." → "PostgreSQL 15.2"
    let mut parts = version_str.splitn(3, ' ');
    match (parts.next(), parts.next()) {
        (Some(name), Some(ver)) => format!("{name} {ver}"),
        _ => version_str.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_version_parses_pg_banner() {
        let banner = "PostgreSQL 15.2 on x86_64-pc-linux-gnu, compiled by gcc (Debian 12.2.0-14) 12.2.0, 64-bit";
        assert_eq!(extract_pg_version(banner), "PostgreSQL 15.2");
    }

    #[test]
    fn extract_version_handles_short_string() {
        assert_eq!(extract_pg_version("PostgreSQL"), "PostgreSQL");
    }
}
