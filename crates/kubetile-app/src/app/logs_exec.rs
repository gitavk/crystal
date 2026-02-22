use k8s_openapi::api::core::v1::Pod;
use kube::Api;

use kubetile_core::resource::DetailSection;
use kubetile_tui::pane::{PaneId, ResourceKind, SplitDirection, ViewType};
use kubetile_tui::widgets::toast::ToastMessage;

use crate::event::AppEvent;
use crate::panes::logs_pane::HistoryRequest;
use crate::panes::{AppLogsPane, ExecPane, LogsPane, ResourceDetailPane, ResourceListPane, YamlPane};

use super::App;

impl App {
    pub(super) fn open_detail_pane(&mut self, kind: ResourceKind, name: String, namespace: String) {
        let sections = vec![DetailSection {
            title: "Metadata".into(),
            fields: vec![
                ("Name".into(), name.clone()),
                ("Namespace".into(), namespace.clone()),
                ("Kind".into(), kind.display_name().into()),
            ],
        }];

        let detail = ResourceDetailPane::new(kind.clone(), name.clone(), Some(namespace), sections);
        let focused = self.tab_manager.active().focused_pane;
        let view = ViewType::Detail(kind, name);
        if let Some(new_id) = self.tab_manager.split_pane(focused, SplitDirection::Horizontal, view) {
            self.panes.insert(new_id, Box::new(detail));
            self.set_focus(new_id);
        }
    }

    pub(super) fn open_yaml_pane(&mut self, pane_id: PaneId, kind: ResourceKind, name: String, content: String) {
        let yaml_pane = YamlPane::new(kind.clone(), name.clone(), content, &self.theme);
        let view = ViewType::Yaml(kind, name);
        if let Some(new_id) = self.tab_manager.split_pane(pane_id, SplitDirection::Horizontal, view) {
            self.panes.insert(new_id, Box::new(yaml_pane));
            self.set_focus(new_id);
        }
    }

    pub(super) fn open_logs_pane(&mut self) {
        let Some((kind, name, namespace)) = self.selected_resource_info() else {
            return;
        };
        if kind != ResourceKind::Pods {
            self.toasts.push(ToastMessage::info("Logs are only available for Pods"));
            return;
        }

        if let Some(existing_id) = self.find_logs_pane_in_active_tab(&name, &namespace) {
            self.set_focus(existing_id);
            return;
        }

        let pane_id = if let Some(existing_id) = self.find_any_logs_pane_in_active_tab() {
            self.panes.insert(existing_id, Box::new(LogsPane::new(name.clone(), namespace.clone())));
            self.set_focus(existing_id);
            existing_id
        } else {
            let focused = self.tab_manager.active().focused_pane;
            let pane = LogsPane::new(name.clone(), namespace.clone());
            let view = ViewType::Logs(name.clone());
            let ratio = self.calc_logs_split_ratio(focused);
            let Some(new_id) = self.tab_manager.split_pane_with_ratio(focused, SplitDirection::Horizontal, view, ratio)
            else {
                return;
            };
            self.panes.insert(new_id, Box::new(pane));
            self.set_focus(new_id);
            new_id
        };

        self.start_logs_stream_for_pane(pane_id, name, namespace);
    }

    fn find_logs_pane_in_active_tab(&self, pod_name: &str, namespace: &str) -> Option<PaneId> {
        self.tab_manager.active().pane_tree.leaf_ids().into_iter().find(|pane_id| {
            self.panes
                .get(pane_id)
                .and_then(|pane| pane.as_any().downcast_ref::<LogsPane>())
                .is_some_and(|logs| logs.pod_name() == pod_name && logs.namespace() == namespace)
        })
    }

    fn find_any_logs_pane_in_active_tab(&self) -> Option<PaneId> {
        self.tab_manager.active().pane_tree.leaf_ids().into_iter().find(|pane_id| {
            self.panes.get(pane_id).is_some_and(|pane| pane.as_any().downcast_ref::<LogsPane>().is_some())
        })
    }

    fn calc_logs_split_ratio(&self, focused_pane: PaneId) -> f32 {
        let Ok((_, rows)) = crossterm::terminal::size() else {
            return 0.5;
        };
        let body_height = rows.saturating_sub(2) as usize;
        if body_height == 0 {
            return 0.5;
        }
        let item_count = self
            .panes
            .get(&focused_pane)
            .and_then(|p| p.as_any().downcast_ref::<ResourceListPane>())
            .map(|p| p.filtered_indices.len())
            .unwrap_or(body_height);
        // borders (2) + header row (1) = 3 overhead lines
        let needed_height = item_count + 3;
        if needed_height < body_height / 2 {
            (needed_height as f32 / body_height as f32).clamp(0.1, 0.5)
        } else {
            0.5
        }
    }

    pub(super) fn start_logs_stream_for_pane(&mut self, pane_id: PaneId, name: String, namespace: String) {
        let Some(client) = &self.kube_client else {
            self.attach_logs_error(pane_id, "No cluster connection".into());
            self.toasts.push(ToastMessage::error("No cluster connection"));
            return;
        };
        let kube_client = client.inner_client();
        let context = client.context().to_string();
        let app_tx = self.app_tx.clone();

        tokio::spawn(async move {
            let mut request = kubetile_core::LogRequest {
                context: Some(context),
                pod_name: name.clone(),
                namespace: namespace.clone(),
                container: None,
                follow: true,
                tail_lines: Some(0),
                since_seconds: None,
                previous: false,
                timestamps: true,
            };

            let pods: Api<Pod> = Api::namespaced(kube_client.clone(), &namespace);
            let mut snapshot_params = kube::api::LogParams {
                follow: false,
                previous: request.previous,
                timestamps: true,
                tail_lines: Some(1000),
                container: request.container.clone(),
                ..Default::default()
            };
            let mut snapshot_result = pods.logs(&name, &snapshot_params).await;
            if let Err(err) = &snapshot_result {
                let msg = err.to_string();
                if msg.contains("container") && msg.contains("must be specified") {
                    let detected_container = detect_container_name(&pods, &name, &msg).await;
                    if let Some(container_name) = detected_container {
                        snapshot_params.container = Some(container_name.clone());
                        request.container = Some(container_name);
                        snapshot_result = pods.logs(&name, &snapshot_params).await;
                    }
                }
            }
            if let Ok(snapshot) = snapshot_result {
                let container = request.container.clone().unwrap_or_default();
                let lines =
                    snapshot.lines().map(|raw| kubetile_core::parse_raw_log_line(raw, &container)).collect::<Vec<_>>();
                let _ =
                    app_tx.send(AppEvent::LogsSnapshotReady { pane_id, lines, container: request.container.clone() });
            } else if let Err(e) = snapshot_result {
                let _ = app_tx.send(AppEvent::LogsStreamError { pane_id, error: format!("snapshot failed: {e}") });
                return;
            }

            if let Ok(stream) = kubetile_core::LogStream::start(request).await {
                let _ = app_tx.send(AppEvent::LogsStreamReady { pane_id, stream });
            }
        });
    }

    pub(super) fn open_exec_pane(&mut self) {
        let Some((kind, name, namespace)) = self.selected_resource_info() else {
            return;
        };
        if kind != ResourceKind::Pods {
            self.toasts.push(ToastMessage::info("Exec is only available for Pods"));
            return;
        }

        let context = self.kube_client.as_ref().map(|c| c.context().to_string());

        let focused = self.tab_manager.active().focused_pane;
        let mut pane = ExecPane::new(name.clone(), "auto".into(), namespace.clone());

        match pane.spawn_kubectl(context.as_deref()) {
            Ok(()) => {
                let view = ViewType::Exec(name);
                let ratio = self.calc_logs_split_ratio(focused);
                let Some(new_id) =
                    self.tab_manager.split_pane_with_ratio(focused, SplitDirection::Horizontal, view, ratio)
                else {
                    return;
                };
                pane.start_output_forwarding(new_id, self.app_tx.clone());
                self.panes.insert(new_id, Box::new(pane));
                self.set_focus(new_id);
                self.dispatcher.set_mode(crate::command::InputMode::Insert);
            }
            Err(e) => {
                self.toasts.push(ToastMessage::error(format!("Failed to start exec: {e}")));
            }
        }
    }

    pub(super) fn attach_logs_stream(&mut self, pane_id: PaneId, stream: kubetile_core::LogStream) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(logs_pane) = pane.as_any_mut().downcast_mut::<LogsPane>() {
                logs_pane.attach_stream(stream);
            }
        }
    }

    pub(super) fn attach_logs_snapshot(
        &mut self,
        pane_id: PaneId,
        lines: Vec<kubetile_core::LogLine>,
        container: Option<String>,
    ) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(logs_pane) = pane.as_any_mut().downcast_mut::<LogsPane>() {
                logs_pane.set_container(container);
                logs_pane.append_snapshot(lines);
            }
        }
    }

    pub(super) fn fetch_logs_history(&mut self, pane_id: PaneId, request: HistoryRequest) {
        let Some(client) = &self.kube_client else {
            return;
        };
        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();
        let tail_lines = request.tail_lines;

        tokio::spawn(async move {
            let pods: Api<Pod> = Api::namespaced(kube_client, &request.namespace);
            let params = kube::api::LogParams {
                follow: false,
                timestamps: true,
                tail_lines: Some(tail_lines as i64),
                container: request.container.clone(),
                ..Default::default()
            };
            let Ok(snapshot) = pods.logs(&request.pod_name, &params).await else {
                return;
            };
            let container = request.container.unwrap_or_default();
            let lines =
                snapshot.lines().map(|raw| kubetile_core::parse_raw_log_line(raw, &container)).collect::<Vec<_>>();
            let _ = app_tx.send(AppEvent::LogsHistoryReady { pane_id, lines, tail_lines });
        });
    }

    pub(super) fn attach_logs_error(&mut self, pane_id: PaneId, error: String) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(logs_pane) = pane.as_any_mut().downcast_mut::<LogsPane>() {
                logs_pane.set_error(error);
            }
        }
    }

    pub(super) fn poll_runtime_panes(&mut self) {
        let mut history_requests: Vec<(PaneId, HistoryRequest)> = Vec::new();
        for (&pane_id, pane) in self.panes.iter_mut() {
            if let Some(logs_pane) = pane.as_any_mut().downcast_mut::<LogsPane>() {
                logs_pane.poll();
                if let Some(req) = logs_pane.take_history_request() {
                    history_requests.push((pane_id, req));
                }
            }
            if let Some(app_logs_pane) = pane.as_any_mut().downcast_mut::<AppLogsPane>() {
                app_logs_pane.poll();
            }
        }
        for (pane_id, req) in history_requests {
            self.fetch_logs_history(pane_id, req);
        }
    }
}

async fn detect_container_name(pods: &Api<Pod>, pod_name: &str, error_msg: &str) -> Option<String> {
    if let Some(name) = first_container_from_logs_error(error_msg) {
        return Some(name);
    }
    pods.get(pod_name)
        .await
        .ok()
        .and_then(|pod| pod.spec.as_ref().and_then(|s| s.containers.first()).map(|c| c.name.clone()))
}

fn first_container_from_logs_error(error_msg: &str) -> Option<String> {
    let marker = "choose one of:";
    let (_, right) = error_msg.split_once(marker)?;
    let candidates = if let (Some(start), Some(end_rel)) = (right.find('['), right.find(']')) {
        &right[start + 1..end_rel]
    } else {
        right
    };
    candidates
        .split([',', ' ', '\n', '\t'])
        .map(str::trim)
        .find(|s| !s.is_empty() && *s != "or")
        .map(|s| s.trim_matches('"').to_string())
}
