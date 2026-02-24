use kubetile_core::QueryConfig;
use kubetile_tui::pane::ResourceKind;
use kubetile_tui::widgets::toast::ToastMessage;

use crate::command::InputMode;

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
            let _ = app_tx.send(crate::event::AppEvent::QueryPromptReady { config });
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
        let msg = format!("Config: db={} user={} port={}", pending.db_input, pending.user_input, pending.port_input);
        self.toasts.push(ToastMessage::info(msg));
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
