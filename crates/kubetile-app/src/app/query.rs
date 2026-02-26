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

        self.execute_query_for_pane(new_id, config, "SELECT version()".to_string());
    }

    fn execute_query_for_pane(&self, pane_id: PaneId, config: QueryConfig, sql: String) {
        let Some(client) = &self.kube_client else {
            return;
        };
        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();

        tokio::spawn(async move {
            let event = match kubetile_core::query::execute_query(&kube_client, &config, &sql).await {
                Ok(result) => AppEvent::QueryReady { pane_id, result },
                Err(e) => AppEvent::QueryError { pane_id, error: e.to_string() },
            };
            let _ = app_tx.send(event);
        });
    }

    pub(super) fn handle_query_ready(&mut self, pane_id: PaneId, result: QueryResult) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                if qp.is_connecting() {
                    if let Some(version_str) = result.rows.first().and_then(|row| row.first()) {
                        qp.set_connected(extract_pg_version(version_str));
                    } else {
                        qp.set_error("Connection test returned no data".to_string());
                    }
                } else {
                    qp.set_result(result);
                }
            }
        }
    }

    pub(super) fn query_editor_input(&mut self, c: char) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.editor_push(c);
            }
        }
    }

    pub(super) fn query_editor_backspace(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.editor_pop();
            }
        }
    }

    pub(super) fn query_editor_newline(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.editor_newline();
            }
        }
    }

    pub(super) fn query_editor_cursor_up(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.cursor_up();
            }
        }
    }

    pub(super) fn query_editor_cursor_down(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.cursor_down();
            }
        }
    }

    pub(super) fn query_editor_cursor_left(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.cursor_left();
            }
        }
    }

    pub(super) fn query_editor_cursor_right(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.cursor_right();
            }
        }
    }

    pub(super) fn query_editor_indent(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.editor_indent();
            }
        }
    }

    pub(super) fn query_editor_deindent(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.editor_deindent();
            }
        }
    }

    pub(super) fn query_editor_home(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.editor_home();
            }
        }
    }

    pub(super) fn query_editor_end(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.editor_end();
            }
        }
    }

    pub(super) fn query_editor_scroll_up(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.result_page_down();
            }
        }
    }

    pub(super) fn query_editor_scroll_down(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.result_page_up();
            }
        }
    }

    pub(super) fn enter_query_browse(&mut self) {
        self.dispatcher.set_mode(InputMode::QueryBrowse);
    }

    pub(super) fn query_browse_next(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.scroll_up();
            }
        }
    }

    pub(super) fn query_browse_prev(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.scroll_down();
            }
        }
    }

    pub(super) fn query_browse_scroll_left(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.scroll_h_left();
            }
        }
    }

    pub(super) fn query_browse_scroll_right(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.scroll_h_right();
            }
        }
    }

    pub(super) fn query_copy_row(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let csv = self
            .panes
            .get(&focused)
            .and_then(|p| p.as_any().downcast_ref::<QueryPane>())
            .and_then(|qp| qp.selected_row_csv());
        match csv {
            None => self.toasts.push(ToastMessage::info("No row selected")),
            Some(csv) => match self.clipboard.as_mut() {
                None => self.toasts.push(ToastMessage::error("Clipboard unavailable")),
                Some(cb) => match cb.set_text(csv) {
                    Ok(_) => self.toasts.push(ToastMessage::info("Copied 1 row")),
                    Err(e) => self.toasts.push(ToastMessage::error(format!("Clipboard error: {e}"))),
                },
            },
        }
    }

    pub(super) fn query_copy_all(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let (csv, n) = match self.panes.get(&focused).and_then(|p| p.as_any().downcast_ref::<QueryPane>()) {
            None => return,
            Some(qp) => (qp.all_rows_csv(), qp.row_count()),
        };
        if csv.is_empty() {
            self.toasts.push(ToastMessage::info("No results to copy"));
            return;
        }
        match self.clipboard.as_mut() {
            None => self.toasts.push(ToastMessage::error("Clipboard unavailable")),
            Some(cb) => match cb.set_text(csv) {
                Ok(_) => self.toasts.push(ToastMessage::info(format!("Copied {n} rows"))),
                Err(e) => self.toasts.push(ToastMessage::error(format!("Clipboard error: {e}"))),
            },
        }
    }

    pub(super) fn execute_current_query(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let (sql, config) = {
            let Some(pane) = self.panes.get_mut(&focused) else {
                return;
            };
            let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() else {
                return;
            };
            let sql = qp.editor_content();
            let sql = sql.trim().to_string();
            if sql.is_empty() {
                return;
            }
            qp.set_executing();
            (sql, qp.config.clone())
        };
        self.execute_query_for_pane(focused, config, sql);
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
