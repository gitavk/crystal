use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, StatefulSet};
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::api::core::v1::{
    ConfigMap, Namespace, Node, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service,
};
use k8s_openapi::api::networking::v1::Ingress;

use crossterm::event::{KeyEvent, KeyEventKind};
use kubetile_tui::pane::{PaneCommand, ResourceKind, SplitDirection, ViewType};
use kubetile_tui::widgets::toast::{ToastLevel, ToastMessage};

use crate::command::{Command, InputMode};
use crate::event::AppEvent;
use crate::panes::{LogsPane, ResourceListPane};
use crate::resource_switcher::ResourceSwitcher;

use super::App;

impl App {
    pub(super) fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Tick => {
                self.poll_runtime_panes();
                self.toasts.retain(|t| !t.is_expired());
            }
            AppEvent::Resize(_, _) => {}
            AppEvent::ResourceUpdate { pane_id, watcher_seq, headers, rows } => {
                if self.watcher_seq_by_pane.get(&pane_id).copied() == Some(watcher_seq) {
                    self.handle_resource_update(pane_id, headers, rows);
                }
            }
            AppEvent::ResourceError { pane_id, watcher_seq, error } => {
                if self.watcher_seq_by_pane.get(&pane_id).copied() == Some(watcher_seq) {
                    self.handle_resource_error(pane_id, error);
                }
            }
            AppEvent::Toast(toast) => {
                match toast.level {
                    ToastLevel::Success => tracing::info!("{}", toast.text),
                    ToastLevel::Error => tracing::error!("{}", toast.text),
                    ToastLevel::Info => tracing::info!("{}", toast.text),
                }
                self.toasts.push(toast);
            }
            AppEvent::YamlReady { pane_id, kind, name, content } => {
                self.open_yaml_pane(pane_id, kind, name, content);
            }
            AppEvent::LogsStreamReady { pane_id, stream } => {
                self.attach_logs_stream(pane_id, stream);
            }
            AppEvent::LogsSnapshotReady { pane_id, lines, container } => {
                self.attach_logs_snapshot(pane_id, lines, container);
            }
            AppEvent::LogsHistoryReady { pane_id, lines, tail_lines } => {
                if let Some(pane) = self.panes.get_mut(&pane_id) {
                    if let Some(logs_pane) = pane.as_any_mut().downcast_mut::<crate::panes::LogsPane>() {
                        logs_pane.prepend_history(lines, tail_lines);
                    }
                }
            }
            AppEvent::LogsStreamError { pane_id, error } => {
                self.attach_logs_error(pane_id, error);
            }
            AppEvent::PortForwardReady { forward } => {
                self.attach_port_forward(forward);
            }
            AppEvent::PortForwardPromptReady { pod, namespace, suggested_remote } => {
                self.open_port_forward_prompt(pod, namespace, suggested_remote);
            }
            AppEvent::ContextSwitchReady { client, namespaces } => {
                self.apply_context_switch(client, namespaces);
            }
            AppEvent::ContextSwitchError { context, error } => {
                self.toasts.push(ToastMessage::error(format!("Failed to switch context {context}: {error}")));
            }
            AppEvent::NamespacesUpdated { namespaces } => {
                self.namespaces = namespaces;
            }
            AppEvent::PtyOutput { pane_id, data } => {
                if let Some(pane) = self.panes.get_mut(&pane_id) {
                    if let Some(exec) = pane.as_any_mut().downcast_mut::<crate::panes::ExecPane>() {
                        exec.process_output(&data);
                    }
                }
            }
            AppEvent::ExecExited { pane_id } => {
                let was_focused = self.tab_manager.active().focused_pane == pane_id;
                self.close_pane(pane_id);
                if was_focused && self.dispatcher.mode() == InputMode::Insert {
                    self.dispatcher.set_mode(InputMode::Normal);
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if let Some((cmd, requires_confirm)) = self.dispatcher.dispatch(key) {
            if requires_confirm || matches!(cmd, Command::Quit) {
                self.pending_confirmation = Some(super::PendingConfirmation::from_command(cmd));
                self.dispatcher.set_mode(InputMode::ConfirmDialog);
            } else {
                self.handle_command(cmd);
            }
        }
    }

    pub(super) fn handle_command(&mut self, cmd: Command) {
        match cmd {
            Command::Quit => {
                self.stop_all_port_forwards();
                self.running = false;
            }
            Command::ShowHelp => self.toggle_help(),
            Command::ToggleAppLogsTab => self.toggle_app_logs_tab(),
            Command::TogglePortForwardsTab => self.toggle_port_forwards_tab(),
            Command::FocusNextPane => self.focus_next(),
            Command::FocusPrevPane => self.focus_prev(),
            Command::SplitVertical => self.split_focused(SplitDirection::Vertical),
            Command::SplitHorizontal => self.split_focused(SplitDirection::Horizontal),
            Command::ClosePane => self.close_focused(),
            Command::EnterMode(mode) => {
                if mode == InputMode::Insert && !self.focused_supports_insert_mode() {
                    return;
                }
                self.dispatcher.set_mode(mode);
                if mode == InputMode::NamespaceSelector {
                    self.namespace_filter.clear();
                    self.namespace_selected = 0;
                    self.refresh_namespaces();
                }
                if mode == InputMode::ContextSelector {
                    self.context_filter.clear();
                    self.context_selected = 0;
                    self.contexts = kubetile_core::KubeClient::list_contexts().unwrap_or_default();
                }
                if mode == InputMode::FilterInput {
                    self.filter_input_buffer.clear();
                    let focused = self.tab_manager.active().focused_pane;
                    if let Some(pane) = self.panes.get(&focused) {
                        if let Some(rp) = pane.as_any().downcast_ref::<ResourceListPane>() {
                            self.filter_input_buffer = rp.filter_text.clone();
                        }
                    }
                }
            }
            Command::ExitMode => self.dispatcher.set_mode(InputMode::Normal),
            Command::NamespaceConfirm => self.handle_namespace_confirm(),
            Command::NamespaceInput(c) => self.handle_namespace_input(c),
            Command::NamespaceBackspace => self.handle_namespace_backspace(),
            Command::ContextConfirm => self.handle_context_confirm(),
            Command::ContextInput(c) => self.handle_context_input(c),
            Command::ContextBackspace => self.handle_context_backspace(),
            Command::FocusDirection(dir) => self.focus_direction(dir),
            Command::NewTab => self.new_tab(),
            Command::CloseTab => self.close_tab(),
            Command::NextTab => self.switch_to_next_tab(),
            Command::PrevTab => self.switch_to_prev_tab(),
            Command::GoToTab(n) => {
                if n > 0 {
                    self.switch_to_tab_index(n - 1);
                }
            }
            Command::ToggleFullscreen => self.toggle_fullscreen(),
            Command::ResizeGrow => {
                let focused = self.tab_manager.active().focused_pane;
                self.tab_manager.active_mut().pane_tree.resize(focused, 0.05, true);
            }
            Command::ResizeShrink => {
                let focused = self.tab_manager.active().focused_pane;
                self.tab_manager.active_mut().pane_tree.resize(focused, 0.05, false);
            }
            Command::Pane(ref pane_cmd) if self.dispatcher.mode() == InputMode::NamespaceSelector => {
                self.handle_namespace_nav(pane_cmd);
            }
            Command::Pane(ref pane_cmd) if self.dispatcher.mode() == InputMode::ContextSelector => {
                self.handle_context_nav(pane_cmd);
            }
            Command::Pane(ref pane_cmd) if self.dispatcher.mode() == InputMode::ResourceSwitcher => {
                if let Some(ref mut sw) = self.resource_switcher {
                    match pane_cmd {
                        PaneCommand::SelectNext => sw.select_next(),
                        PaneCommand::SelectPrev => sw.select_prev(),
                        _ => {}
                    }
                }
            }
            Command::Pane(pane_cmd) => {
                let focused = self.tab_manager.active().focused_pane;
                match &pane_cmd {
                    PaneCommand::Select => {
                        if let Some((kind, name, ns)) = self.selected_resource_info() {
                            self.open_detail_pane(kind, name, ns);
                            return;
                        }
                    }
                    PaneCommand::Back => {
                        if let Some(pane) = self.panes.get(&focused) {
                            let is_detail_or_yaml =
                                matches!(pane.view_type(), ViewType::Detail(..) | ViewType::Yaml(..));
                            if is_detail_or_yaml {
                                self.close_pane(focused);
                                return;
                            }
                        }
                    }
                    _ => {}
                }
                if let Some(pane) = self.panes.get_mut(&focused) {
                    pane.handle_command(&pane_cmd);
                }
                if matches!(pane_cmd, PaneCommand::PageUp) {
                    if let Some(pane) = self.panes.get_mut(&focused) {
                        if let Some(lp) = pane.as_any_mut().downcast_mut::<LogsPane>() {
                            if lp.take_history_limit_notice() {
                                self.toasts.push(ToastMessage::info(
                                    "History buffer full (3000 lines). Use Ctrl+E to download the full log.",
                                ));
                            }
                        }
                    }
                }
            }

            Command::FilterInput(c) => {
                self.filter_input_buffer.push(c);
                let text = self.filter_input_buffer.clone();
                let focused = self.tab_manager.active().focused_pane;
                if let Some(pane) = self.panes.get_mut(&focused) {
                    pane.handle_command(&PaneCommand::Filter(text));
                }
            }
            Command::FilterBackspace => {
                self.filter_input_buffer.pop();
                if self.filter_input_buffer.is_empty() {
                    let focused = self.tab_manager.active().focused_pane;
                    if let Some(pane) = self.panes.get_mut(&focused) {
                        pane.handle_command(&PaneCommand::ClearFilter);
                    }
                } else {
                    let text = self.filter_input_buffer.clone();
                    let focused = self.tab_manager.active().focused_pane;
                    if let Some(pane) = self.panes.get_mut(&focused) {
                        pane.handle_command(&PaneCommand::Filter(text));
                    }
                }
            }
            Command::FilterCancel => {
                self.filter_input_buffer.clear();
                let focused = self.tab_manager.active().focused_pane;
                if let Some(pane) = self.panes.get_mut(&focused) {
                    pane.handle_command(&PaneCommand::ClearFilter);
                }
                self.dispatcher.set_mode(InputMode::Normal);
            }
            Command::PortForwardInput(c) => {
                if let Some(ref mut pending) = self.pending_port_forward {
                    let target = match pending.active_field {
                        super::PortForwardField::Local => &mut pending.local_input,
                        super::PortForwardField::Remote => &mut pending.remote_input,
                    };
                    if target == "0" {
                        target.clear();
                    }
                    target.push(c);
                }
            }
            Command::PortForwardBackspace => {
                if let Some(ref mut pending) = self.pending_port_forward {
                    let target = match pending.active_field {
                        super::PortForwardField::Local => &mut pending.local_input,
                        super::PortForwardField::Remote => &mut pending.remote_input,
                    };
                    target.pop();
                }
            }
            Command::PortForwardToggleField => {
                if let Some(ref mut pending) = self.pending_port_forward {
                    pending.active_field = pending.active_field.toggle();
                }
            }
            Command::PortForwardConfirm => {
                self.confirm_port_forward();
            }
            Command::PortForwardCancel => {
                self.pending_port_forward = None;
                self.dispatcher.set_mode(InputMode::Normal);
            }
            Command::SortByColumn => {
                let focused = self.tab_manager.active().focused_pane;
                if let Some(pane) = self.panes.get_mut(&focused) {
                    if let Some(rp) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                        let next_col = match rp.sort_column {
                            None => 0,
                            Some(c) => {
                                let num_cols = rp.state.headers.len();
                                if num_cols == 0 {
                                    0
                                } else {
                                    (c + 1) % num_cols
                                }
                            }
                        };
                        rp.sort_by_column(next_col);
                    }
                }
            }
            Command::ToggleAllNamespaces => {
                let focused = self.tab_manager.active().focused_pane;
                if let Some(pane) = self.panes.get_mut(&focused) {
                    if let Some(rp) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                        rp.all_namespaces = !rp.all_namespaces;
                        let kind = rp.kind().cloned();
                        let is_all = rp.all_namespaces;

                        if let Some(kind) = kind {
                            if kind.is_namespaced() {
                                if is_all {
                                    self.start_watcher_for_pane(focused, &kind, "");
                                } else {
                                    let ns = self.context_resolver.namespace().unwrap_or("default").to_string();
                                    self.start_watcher_for_pane(focused, &kind, &ns);
                                }
                                if let Some(pane) = self.panes.get_mut(&focused) {
                                    if let Some(rp) = pane.as_any_mut().downcast_mut::<ResourceListPane>() {
                                        let headers = rp.state.headers.clone();
                                        rp.state = crate::state::ResourceListState::new(headers);
                                        rp.filtered_indices.clear();
                                    }
                                }
                            }
                        }
                    }
                }
                self.update_active_tab_title();
            }

            Command::EnterResourceSwitcher => {
                self.resource_switcher = Some(ResourceSwitcher::new());
                self.dispatcher.set_mode(InputMode::ResourceSwitcher);
            }
            Command::ResourceSwitcherInput(ch) => {
                if let Some(ref mut sw) = self.resource_switcher {
                    sw.on_input(ch);
                }
            }
            Command::ResourceSwitcherBackspace => {
                if let Some(ref mut sw) = self.resource_switcher {
                    sw.on_backspace();
                }
            }
            Command::ResourceSwitcherConfirm => {
                let kind = self.resource_switcher.as_ref().and_then(|sw| sw.confirm());
                if let Some(kind) = kind {
                    self.switch_resource(kind);
                }
                self.resource_switcher = None;
                self.dispatcher.set_mode(InputMode::Normal);
            }
            Command::DenyAction => {
                self.resource_switcher = None;
                self.pending_confirmation = None;
                self.pending_port_forward = None;
                self.dispatcher.set_mode(InputMode::Normal);
            }

            Command::DeleteResource => {
                let focused = self.tab_manager.active().focused_pane;
                let is_port_forwards = self
                    .panes
                    .get(&focused)
                    .is_some_and(|p| matches!(p.view_type(), ViewType::Plugin(name) if name == "PortForwards"));
                if is_port_forwards {
                    self.stop_selected_port_forward();
                } else {
                    self.initiate_delete();
                }
            }
            Command::ConfirmAction => {
                self.execute_confirmed_action();
            }

            Command::ViewYaml => {
                if let Some((kind, name, ns)) = self.selected_resource_info() {
                    let Some(client) = &self.kube_client else {
                        self.toasts.push(ToastMessage::error("No cluster connection"));
                        return;
                    };
                    let kube_client = client.inner_client();
                    let app_tx = self.app_tx.clone();
                    let focused = self.tab_manager.active().focused_pane;
                    let kind_clone = kind.clone();
                    let name_clone = name.clone();

                    tokio::spawn(async move {
                        let executor = kubetile_core::ActionExecutor::new(kube_client);
                        let result = dispatch_get_yaml(&executor, &kind, &name, &ns).await;
                        let event = match result {
                            Ok(yaml) => AppEvent::YamlReady {
                                pane_id: focused,
                                kind: kind_clone,
                                name: name_clone,
                                content: yaml,
                            },
                            Err(e) => AppEvent::Toast(ToastMessage::error(format!("YAML fetch failed: {e}"))),
                        };
                        let _ = app_tx.send(event);
                    });
                }
            }

            Command::ViewDescribe => {
                if let Some((kind, name, ns)) = self.selected_resource_info() {
                    let Some(client) = &self.kube_client else {
                        self.toasts.push(ToastMessage::error("No cluster connection"));
                        return;
                    };
                    let kube_client = client.inner_client();
                    let app_tx = self.app_tx.clone();
                    let focused = self.tab_manager.active().focused_pane;
                    let kind_clone = kind.clone();
                    let name_clone = name.clone();

                    tokio::spawn(async move {
                        let executor = kubetile_core::ActionExecutor::new(kube_client);
                        let result = dispatch_describe(&executor, &kind, &name, &ns).await;
                        let event = match result {
                            Ok(text) => AppEvent::YamlReady {
                                pane_id: focused,
                                kind: kind_clone,
                                name: name_clone,
                                content: text,
                            },
                            Err(e) => AppEvent::Toast(ToastMessage::error(format!("Describe failed: {e}"))),
                        };
                        let _ = app_tx.send(event);
                    });
                }
            }
            Command::SaveLogsToFile => {
                self.initiate_save_logs();
            }
            Command::DownloadFullLogs => {
                self.initiate_download_full_logs();
            }

            Command::RestartRollout => {
                if let Some((kind, name, ns)) = self.selected_resource_info() {
                    if kind == ResourceKind::Deployments {
                        let Some(client) = &self.kube_client else {
                            self.toasts.push(ToastMessage::error("No cluster connection"));
                            return;
                        };
                        let kube_client = client.inner_client();
                        let app_tx = self.app_tx.clone();

                        tokio::spawn(async move {
                            let executor = kubetile_core::ActionExecutor::new(kube_client);
                            let toast = match executor.restart_rollout(&name, &ns).await {
                                Ok(()) => ToastMessage::success(format!("Restarted {name}")),
                                Err(e) => ToastMessage::error(format!("Restart failed: {e}")),
                            };
                            let _ = app_tx.send(AppEvent::Toast(toast));
                        });
                    } else {
                        self.toasts.push(ToastMessage::info("Restart rollout is only available for Deployments"));
                    }
                }
            }

            Command::ScaleResource => {
                self.toasts.push(ToastMessage::info("Scale not yet implemented"));
            }

            Command::ToggleDebugMode => {
                self.initiate_debug_toggle();
            }

            Command::ToggleRootDebugMode => {
                self.initiate_root_debug_toggle();
            }

            Command::ViewLogs => {
                self.open_logs_pane();
            }

            Command::ExecInto => {
                self.open_exec_pane();
            }
            Command::PortForward => {
                self.toggle_port_forward_for_selected();
            }

            Command::TerminalSpawn
            | Command::TerminalClose { .. }
            | Command::TerminalResize { .. }
            | Command::TerminalInput { .. }
            | Command::ExecStart { .. }
            | Command::ExecClose { .. }
            | Command::LogsStart { .. }
            | Command::LogsStop { .. }
            | Command::PortForwardStart { .. }
            | Command::PortForwardStop { .. } => {}
        }
    }
}

async fn dispatch_get_yaml(
    executor: &kubetile_core::ActionExecutor,
    kind: &ResourceKind,
    name: &str,
    ns: &str,
) -> anyhow::Result<String> {
    match kind {
        ResourceKind::Pods => executor.get_yaml::<Pod>(name, ns).await,
        ResourceKind::Deployments => executor.get_yaml::<Deployment>(name, ns).await,
        ResourceKind::Services => executor.get_yaml::<Service>(name, ns).await,
        ResourceKind::StatefulSets => executor.get_yaml::<StatefulSet>(name, ns).await,
        ResourceKind::DaemonSets => executor.get_yaml::<DaemonSet>(name, ns).await,
        ResourceKind::Jobs => executor.get_yaml::<Job>(name, ns).await,
        ResourceKind::CronJobs => executor.get_yaml::<CronJob>(name, ns).await,
        ResourceKind::ConfigMaps => executor.get_yaml::<ConfigMap>(name, ns).await,
        ResourceKind::Secrets => executor.get_yaml::<Secret>(name, ns).await,
        ResourceKind::Ingresses => executor.get_yaml::<Ingress>(name, ns).await,
        ResourceKind::PersistentVolumeClaims => executor.get_yaml::<PersistentVolumeClaim>(name, ns).await,
        ResourceKind::Nodes => executor.get_yaml_cluster::<Node>(name).await,
        ResourceKind::Namespaces => executor.get_yaml_cluster::<Namespace>(name).await,
        ResourceKind::PersistentVolumes => executor.get_yaml_cluster::<PersistentVolume>(name).await,
        ResourceKind::Custom(_) => Err(anyhow::anyhow!("YAML view not supported for custom resources")),
    }
}

async fn dispatch_describe(
    executor: &kubetile_core::ActionExecutor,
    kind: &ResourceKind,
    name: &str,
    ns: &str,
) -> anyhow::Result<String> {
    match kind {
        ResourceKind::Pods => executor.describe::<Pod>(name, ns).await,
        ResourceKind::Deployments => executor.describe::<Deployment>(name, ns).await,
        ResourceKind::Services => executor.describe::<Service>(name, ns).await,
        ResourceKind::StatefulSets => executor.describe::<StatefulSet>(name, ns).await,
        ResourceKind::DaemonSets => executor.describe::<DaemonSet>(name, ns).await,
        ResourceKind::Jobs => executor.describe::<Job>(name, ns).await,
        ResourceKind::CronJobs => executor.describe::<CronJob>(name, ns).await,
        ResourceKind::ConfigMaps => executor.describe::<ConfigMap>(name, ns).await,
        ResourceKind::Secrets => executor.describe::<Secret>(name, ns).await,
        ResourceKind::Ingresses => executor.describe::<Ingress>(name, ns).await,
        ResourceKind::PersistentVolumeClaims => executor.describe::<PersistentVolumeClaim>(name, ns).await,
        _ => Err(anyhow::anyhow!("Describe not supported for this resource type")),
    }
}
