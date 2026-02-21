use std::env;
use std::fs;
use std::path::PathBuf;

use kubetile_tui::pane::{Pane, ResourceKind, ViewType};
use kubetile_tui::widgets::toast::ToastMessage;

use crate::command::InputMode;
use crate::event::AppEvent;
use crate::panes::{LogsPane, ResourceListPane};

use super::{App, PendingAction, PendingConfirmation};

impl App {
    pub(super) fn focused_supports_insert_mode(&self) -> bool {
        let focused = self.tab_manager.active().focused_pane;
        self.panes.get(&focused).is_some_and(|pane| matches!(pane.view_type(), ViewType::Exec(_) | ViewType::Terminal))
    }

    pub(super) fn selected_resource_info(&self) -> Option<(ResourceKind, String, String)> {
        let focused = self.tab_manager.active().focused_pane;
        let pane = self.panes.get(&focused)?;
        let rp = pane.as_any().downcast_ref::<ResourceListPane>()?;

        let kind = rp.kind()?.clone();

        let selected_idx = match rp.state.selected {
            Some(s) => {
                if rp.filtered_indices.is_empty() {
                    s
                } else {
                    *rp.filtered_indices.get(s)?
                }
            }
            None => return None,
        };

        let row = rp.state.items.get(selected_idx)?;
        let name = super::header_value(&rp.state.headers, row, "NAME", 0).unwrap_or_default();
        let namespace = super::header_value(&rp.state.headers, row, "NAMESPACE", usize::MAX)
            .unwrap_or_else(|| self.context_resolver.namespace().unwrap_or("default").to_string());

        Some((kind, name, namespace))
    }

    pub(super) fn initiate_delete(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let Some(pane) = self.panes.get(&focused) else { return };
        let Some(rp) = pane.as_any().downcast_ref::<ResourceListPane>() else { return };

        let kind = match rp.view_type() {
            ViewType::ResourceList(k) => k.clone(),
            _ => return,
        };

        let selected_idx = match rp.state.selected {
            Some(s) => {
                if rp.filtered_indices.is_empty() {
                    s
                } else {
                    match rp.filtered_indices.get(s) {
                        Some(&i) => i,
                        None => return,
                    }
                }
            }
            None => return,
        };

        let row = match rp.state.items.get(selected_idx) {
            Some(r) => r,
            None => return,
        };

        let name = super::header_value(&rp.state.headers, row, "NAME", 0).unwrap_or_default();
        let namespace = super::header_value(&rp.state.headers, row, "NAMESPACE", usize::MAX)
            .unwrap_or_else(|| self.context_resolver.namespace().unwrap_or("default").to_string());

        let message = format!("Delete {} {}\nin namespace {}?", kind.display_name(), name, namespace);

        self.pending_confirmation =
            Some(PendingConfirmation { message, action: PendingAction::Delete { kind, name, namespace } });
        self.dispatcher.set_mode(InputMode::ConfirmDialog);
    }

    pub(super) fn initiate_save_logs(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let Some(pane) = self.panes.get(&focused) else { return };
        let Some(logs) = pane.as_any().downcast_ref::<LogsPane>() else {
            self.toasts.push(ToastMessage::info("Save logs is only available in a Logs pane"));
            return;
        };

        let Some(downloads_dir) = home_downloads_dir() else {
            self.toasts.push(ToastMessage::error("HOME is not set; cannot resolve $HOME/Downloads"));
            return;
        };

        let context = self.context_resolver.context_name().unwrap_or("unknown-context");
        let namespace = logs.namespace();
        let pod = logs.pod_name();
        let timestamp = filename_timestamp_now();

        let filename = format!(
            "{}_{}_{}_{}.log",
            sanitize_filename_component(context),
            sanitize_filename_component(namespace),
            sanitize_filename_component(pod),
            timestamp
        );
        let path = downloads_dir.join(filename);

        let lines = logs.export_filtered_history();
        let filter = logs.filter_text().unwrap_or("");
        let exported_at = jiff::Timestamp::now().to_string();

        let mut content = String::new();
        content.push_str(&format!("# context: {context}\n"));
        content.push_str(&format!("# namespace: {namespace}\n"));
        content.push_str(&format!("# pod: {pod}\n"));
        content.push_str(&format!("# exported_at: {exported_at}\n"));
        if !filter.is_empty() {
            content.push_str(&format!("# filter: {filter}\n"));
        }
        content.push('\n');
        for line in lines {
            content.push_str(&line);
            content.push('\n');
        }

        let message = format!("Save logs to:\n{}?", path.display());
        self.pending_confirmation =
            Some(PendingConfirmation { message, action: PendingAction::SaveLogs { path, content } });
        self.dispatcher.set_mode(InputMode::ConfirmDialog);
    }

    pub(super) fn execute_confirmed_action(&mut self) {
        let confirmation = match self.pending_confirmation.take() {
            Some(c) => c,
            None => return,
        };
        self.dispatcher.set_mode(InputMode::Normal);

        match confirmation.action {
            PendingAction::Delete { kind, name, namespace } => {
                let Some(client) = &self.kube_client else {
                    self.toasts.push(ToastMessage::error("No cluster connection"));
                    return;
                };
                let kube_client = client.inner_client();
                let app_tx = self.app_tx.clone();
                let display_name = format!("{} {}", kind.short_name(), name);

                tokio::spawn(async move {
                    let executor = kubetile_core::ActionExecutor::new(kube_client);
                    let result = match kind {
                        ResourceKind::Pods => {
                            executor.delete::<k8s_openapi::api::core::v1::Pod>(&name, &namespace).await
                        }
                        ResourceKind::Deployments => {
                            executor.delete::<k8s_openapi::api::apps::v1::Deployment>(&name, &namespace).await
                        }
                        ResourceKind::Services => {
                            executor.delete::<k8s_openapi::api::core::v1::Service>(&name, &namespace).await
                        }
                        ResourceKind::StatefulSets => {
                            executor.delete::<k8s_openapi::api::apps::v1::StatefulSet>(&name, &namespace).await
                        }
                        ResourceKind::DaemonSets => {
                            executor.delete::<k8s_openapi::api::apps::v1::DaemonSet>(&name, &namespace).await
                        }
                        ResourceKind::Jobs => {
                            executor.delete::<k8s_openapi::api::batch::v1::Job>(&name, &namespace).await
                        }
                        ResourceKind::CronJobs => {
                            executor.delete::<k8s_openapi::api::batch::v1::CronJob>(&name, &namespace).await
                        }
                        ResourceKind::ConfigMaps => {
                            executor.delete::<k8s_openapi::api::core::v1::ConfigMap>(&name, &namespace).await
                        }
                        ResourceKind::Secrets => {
                            executor.delete::<k8s_openapi::api::core::v1::Secret>(&name, &namespace).await
                        }
                        ResourceKind::Ingresses => {
                            executor.delete::<k8s_openapi::api::networking::v1::Ingress>(&name, &namespace).await
                        }
                        ResourceKind::PersistentVolumeClaims => {
                            executor
                                .delete::<k8s_openapi::api::core::v1::PersistentVolumeClaim>(&name, &namespace)
                                .await
                        }
                        _ => Err(anyhow::anyhow!("Delete not supported for this resource type")),
                    };

                    let toast_event = match result {
                        Ok(()) => AppEvent::Toast(ToastMessage::success(format!("Deleted {display_name}"))),
                        Err(e) => AppEvent::Toast(ToastMessage::error(format!("Failed to delete {display_name}: {e}"))),
                    };
                    let _ = app_tx.send(toast_event);
                });
            }
            PendingAction::SaveLogs { path, content } => {
                if let Some(parent) = path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        self.toasts.push(ToastMessage::error(format!("Failed to create {}: {e}", parent.display())));
                        return;
                    }
                }

                match fs::write(&path, content) {
                    Ok(()) => self.toasts.push(ToastMessage::success(format!("Saved logs to {}", path.display()))),
                    Err(e) => self.toasts.push(ToastMessage::error(format!("Failed to save logs: {e}"))),
                }
            }
            PendingAction::MutateCommand(cmd) => {
                self.handle_command(cmd);
            }
        }
    }
}

fn home_downloads_dir() -> Option<PathBuf> {
    env::var_os("HOME").map(PathBuf::from).map(|home| home.join("Downloads"))
}

fn filename_timestamp_now() -> String {
    let iso = jiff::Timestamp::now().to_string();
    let mut out = String::with_capacity(15);
    for ch in iso.chars() {
        if ch.is_ascii_digit() {
            out.push(ch);
        } else if ch == 'T' && out.len() == 8 {
            out.push('-');
        }
        if out.len() >= 15 {
            break;
        }
    }
    if out.len() == 15 {
        out
    } else {
        "19700101-000000".to_string()
    }
}

fn sanitize_filename_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}
