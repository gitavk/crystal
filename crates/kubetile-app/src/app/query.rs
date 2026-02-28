use std::collections::HashMap;

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
        let new_id = if self.query_open_new_tab {
            let tab_name = format!("query:{}", config.pod);
            self.tab_manager.new_tab(&tab_name, ViewType::Query(config.pod.clone()));
            self.tab_manager.active().focused_pane
        } else {
            let focused = self.tab_manager.active().focused_pane;
            let view = ViewType::Query(config.pod.clone());
            let Some(id) = self.tab_manager.split_pane_with_ratio(focused, SplitDirection::Horizontal, view, 0.7)
            else {
                return;
            };
            id
        };
        self.panes.insert(new_id, Box::new(pane));
        self.set_focus(new_id);

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
        let mut schema_config: Option<QueryConfig> = None;
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                if qp.is_connecting() {
                    if let Some(version_str) = result.rows.first().and_then(|row| row.first()) {
                        qp.set_connected(extract_pg_version(version_str));
                        schema_config = Some(qp.config.clone());
                    } else {
                        qp.set_error("Connection test returned no data".to_string());
                    }
                } else {
                    let sql = qp.last_executed_sql().map(|s| s.to_string());
                    let config = qp.config.clone();
                    qp.set_result(result);
                    let (rows, bytes) = qp.size_hint();
                    if rows > 500 || bytes > 512_000 {
                        self.toasts.push(ToastMessage::info("Result is large — consider E to export"));
                    }
                    if let Some(sql) = sql {
                        let mut history =
                            kubetile_core::QueryHistory::load(&config.namespace, &config.pod, &config.database);
                        let _ = history.append(&sql);
                    }
                }
            }
        }
        if let Some(config) = schema_config {
            self.execute_schema_for_pane(pane_id, config);
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

    pub(super) fn open_query_history(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let config = match self.panes.get(&focused) {
            Some(pane) => pane.as_any().downcast_ref::<QueryPane>().map(|qp| qp.config.clone()),
            None => None,
        };
        let Some(config) = config else { return };
        let history = kubetile_core::QueryHistory::load(&config.namespace, &config.pod, &config.database);
        let entries: Vec<String> = history.entries.iter().map(|e| e.sql.clone()).collect();
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.open_history(entries);
            }
        }
        self.dispatcher.set_mode(InputMode::QueryHistory);
    }

    pub(super) fn close_query_history(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.close_history();
            }
        }
        self.dispatcher.set_mode(InputMode::QueryEditor);
    }

    pub(super) fn query_history_next(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.history_next();
            }
        }
    }

    pub(super) fn query_history_prev(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.history_prev();
            }
        }
    }

    pub(super) fn query_history_select(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let sql = self
            .panes
            .get(&focused)
            .and_then(|p| p.as_any().downcast_ref::<QueryPane>())
            .and_then(|qp| qp.history_selected_sql())
            .map(|s| s.to_string());
        let Some(sql) = sql else { return };
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.set_editor_content(&sql);
                qp.close_history();
            }
        }
        self.dispatcher.set_mode(InputMode::QueryEditor);
    }

    pub(super) fn query_history_delete(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let (idx, config) = match self.panes.get(&focused).and_then(|p| p.as_any().downcast_ref::<QueryPane>()) {
            Some(qp) => (qp.history_selected_index(), qp.config.clone()),
            None => return,
        };
        let mut history = kubetile_core::QueryHistory::load(&config.namespace, &config.pod, &config.database);
        let _ = history.delete(idx);
        let entries: Vec<String> = history.entries.iter().map(|e| e.sql.clone()).collect();
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.open_history(entries);
            }
        }
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
            qp.set_executing(&sql);
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

    // --- Save-name dialog ---

    pub(super) fn open_save_query_dialog(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.open_save_name();
            }
        }
        self.dispatcher.set_mode(InputMode::SaveQueryName);
    }

    pub(super) fn save_query_name_input(&mut self, c: char) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.save_name_input(c);
            }
        }
    }

    pub(super) fn save_query_name_backspace(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.save_name_backspace();
            }
        }
    }

    pub(super) fn confirm_save_query(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let (name, sql) = match self.panes.get(&focused).and_then(|p| p.as_any().downcast_ref::<QueryPane>()) {
            Some(qp) => {
                let name = qp.current_save_name().unwrap_or("").trim().to_string();
                let sql = qp.editor_content();
                (name, sql)
            }
            None => return,
        };
        if name.is_empty() {
            return;
        }
        let mut saved = kubetile_core::SavedQueries::load();
        let _ = saved.add(&name, &sql);
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.close_save_name();
            }
        }
        self.dispatcher.set_mode(InputMode::QueryEditor);
        self.toasts.push(ToastMessage::info(format!("Saved \"{name}\"")));
    }

    pub(super) fn cancel_save_query(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.close_save_name();
            }
        }
        self.dispatcher.set_mode(InputMode::QueryEditor);
    }

    // --- Saved-queries popup ---

    pub(super) fn open_saved_queries(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let saved = kubetile_core::SavedQueries::load();
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.open_saved_queries(saved.entries);
            }
        }
        self.dispatcher.set_mode(InputMode::SavedQueries);
    }

    pub(super) fn saved_queries_next(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.saved_queries_next();
            }
        }
    }

    pub(super) fn saved_queries_prev(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.saved_queries_prev();
            }
        }
    }

    /// Handles both "load into editor" (normal mode) and "confirm rename" (rename mode).
    pub(super) fn saved_queries_select(&mut self) {
        let focused = self.tab_manager.active().focused_pane;

        // Check if we're in rename mode first
        let is_renaming = self
            .panes
            .get(&focused)
            .and_then(|p| p.as_any().downcast_ref::<QueryPane>())
            .map(|qp| qp.saved_queries_is_renaming())
            .unwrap_or(false);

        if is_renaming {
            // Confirm rename: get (real_index, new_name)
            let data = self.panes.get(&focused).and_then(|p| p.as_any().downcast_ref::<QueryPane>()).and_then(|qp| {
                let (real_idx, _, _) = qp.saved_queries_selected()?;
                let new_name = qp.saved_queries_rename_input()?.trim().to_string();
                Some((real_idx, new_name))
            });
            if let Some((real_idx, new_name)) = data {
                if !new_name.is_empty() {
                    let mut saved = kubetile_core::SavedQueries::load();
                    let _ = saved.rename(real_idx, &new_name);
                    // Reload popup with updated entries
                    if let Some(pane) = self.panes.get_mut(&focused) {
                        if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                            qp.open_saved_queries(saved.entries);
                        }
                    }
                    self.toasts.push(ToastMessage::info(format!("Renamed to \"{new_name}\"")));
                }
            }
            return;
        }

        // Normal select: load SQL into editor
        let sql = self
            .panes
            .get(&focused)
            .and_then(|p| p.as_any().downcast_ref::<QueryPane>())
            .and_then(|qp| qp.saved_queries_selected())
            .map(|(_, _, sql)| sql.to_string());
        let Some(sql) = sql else { return };
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.set_editor_content(&sql);
                qp.close_saved_queries();
            }
        }
        self.dispatcher.set_mode(InputMode::QueryEditor);
    }

    pub(super) fn saved_queries_delete(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let real_idx = self
            .panes
            .get(&focused)
            .and_then(|p| p.as_any().downcast_ref::<QueryPane>())
            .and_then(|qp| qp.saved_queries_selected())
            .map(|(idx, _, _)| idx);
        let Some(real_idx) = real_idx else { return };
        let mut saved = kubetile_core::SavedQueries::load();
        let _ = saved.delete(real_idx);
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.open_saved_queries(saved.entries);
            }
        }
    }

    pub(super) fn saved_queries_start_rename(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.saved_queries_start_rename();
            }
        }
    }

    pub(super) fn saved_queries_input(&mut self, c: char) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.saved_queries_input(c);
            }
        }
    }

    pub(super) fn saved_queries_backspace(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.saved_queries_backspace();
            }
        }
    }

    pub(super) fn saved_queries_start_filter(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.saved_queries_start_filter();
            }
        }
    }

    pub(super) fn close_saved_queries(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let still_open = match self.panes.get_mut(&focused) {
            Some(pane) => match pane.as_any_mut().downcast_mut::<QueryPane>() {
                Some(qp) => qp.saved_queries_close_sub_mode(),
                None => false,
            },
            None => false,
        };
        if !still_open {
            self.dispatcher.set_mode(InputMode::QueryEditor);
        }
    }

    // --- Export dialog ---

    pub(super) fn open_export_dialog(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let config = match self.panes.get(&focused).and_then(|p| p.as_any().downcast_ref::<QueryPane>()) {
            Some(qp) => qp.config.clone(),
            None => return,
        };
        let now = jiff::Zoned::now();
        let ts = now.strftime("%Y%m%d_%H%M%S");
        let path = format!("~/kubetile_{}_{ts}.csv", config.pod);
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.open_export_dialog(path);
            }
        }
        self.dispatcher.set_mode(InputMode::ExportDialog);
    }

    pub(super) fn export_path_input(&mut self, c: char) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.export_path_input(c);
            }
        }
    }

    pub(super) fn export_path_backspace(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.export_path_backspace();
            }
        }
    }

    pub(super) fn confirm_export(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let (path_str, csv, row_count) =
            match self.panes.get(&focused).and_then(|p| p.as_any().downcast_ref::<QueryPane>()) {
                Some(qp) => {
                    let path = qp.current_export_path().unwrap_or("").to_string();
                    let csv = qp.all_rows_csv();
                    let n = qp.row_count();
                    (path, csv, n)
                }
                None => return,
            };
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.close_export_dialog();
            }
        }
        self.dispatcher.set_mode(InputMode::QueryBrowse);

        let full_path = expand_tilde(&path_str);
        if let Some(parent) = full_path.parent() {
            if !parent.as_os_str().is_empty() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    self.toasts.push(ToastMessage::error(format!("Export failed: {e}")));
                    return;
                }
            }
        }
        match std::fs::write(&full_path, csv) {
            Ok(()) => self.toasts.push(ToastMessage::info(format!("Exported {row_count} rows → {path_str}"))),
            Err(e) => self.toasts.push(ToastMessage::error(format!("Export failed: {e}"))),
        }
    }

    pub(super) fn cancel_export(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.close_export_dialog();
            }
        }
        self.dispatcher.set_mode(InputMode::QueryBrowse);
    }

    // --- Schema fetch ---

    fn execute_schema_for_pane(&self, pane_id: PaneId, config: QueryConfig) {
        let Some(client) = &self.kube_client else {
            return;
        };
        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();
        const SCHEMA_SQL: &str = "\
            SELECT table_schema, table_name, column_name, data_type \
            FROM information_schema.columns \
            WHERE table_schema NOT IN ('pg_catalog','information_schema') \
            ORDER BY table_name, ordinal_position";

        tokio::spawn(async move {
            if let Ok(result) = kubetile_core::query::execute_query(&kube_client, &config, SCHEMA_SQL).await {
                let _ = app_tx.send(crate::event::AppEvent::SchemaReady { pane_id, rows: result.rows });
            }
        });
    }

    pub(super) fn handle_schema_ready(&mut self, pane_id: PaneId, rows: Vec<Vec<String>>) {
        let mut tables: Vec<(String, String)> = Vec::new();
        let mut columns: HashMap<String, Vec<(String, String)>> = HashMap::new();

        for row in &rows {
            if row.len() < 4 {
                continue;
            }
            let schema = row[0].clone();
            let table = row[1].clone();
            let col_name = row[2].clone();
            let col_type = row[3].clone();

            if !tables.iter().any(|(n, _)| n == &table) {
                tables.push((table.clone(), schema));
            }
            columns.entry(table).or_default().push((col_name, col_type));
        }

        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.set_schema(tables, columns);
            }
        }
    }

    // --- Autocomplete ---

    pub(super) fn trigger_completion(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                if qp.trigger_completion() {
                    self.dispatcher.set_mode(InputMode::Completion);
                }
            }
        }
    }

    pub(super) fn complete_next(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.complete_next();
            }
        }
    }

    pub(super) fn complete_prev(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.complete_prev();
            }
        }
    }

    pub(super) fn complete_accept(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.complete_accept();
            }
        }
        self.dispatcher.set_mode(InputMode::QueryEditor);
    }

    pub(super) fn complete_dismiss(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.complete_dismiss();
            }
        }
        self.dispatcher.set_mode(InputMode::QueryEditor);
    }

    pub(super) fn complete_input(&mut self, c: char) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.editor_push(c);
                qp.update_completion();
                if !qp.completion_is_open() {
                    self.dispatcher.set_mode(InputMode::QueryEditor);
                }
            }
        }
    }

    pub(super) fn complete_backspace(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        if let Some(pane) = self.panes.get_mut(&focused) {
            if let Some(qp) = pane.as_any_mut().downcast_mut::<QueryPane>() {
                qp.editor_pop();
                qp.update_completion();
                if !qp.completion_is_open() {
                    self.dispatcher.set_mode(InputMode::QueryEditor);
                }
            }
        }
    }
}

fn expand_tilde(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(rest)
    } else if path == "~" {
        dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."))
    } else {
        std::path::PathBuf::from(path)
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
