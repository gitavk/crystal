use std::time::Duration;

use k8s_openapi::api::core::v1::Pod;
use kube::Api;

use kubetile_core::ForwardId;
use kubetile_tui::pane::ResourceKind;
use kubetile_tui::widgets::toast::ToastMessage;

use crate::command::InputMode;
use crate::event::AppEvent;
use crate::panes::PortForwardsPane;

use super::{App, PendingPortForward, PortForwardField};

impl App {
    pub(super) fn toggle_port_forward_for_selected(&mut self) {
        let Some((kind, pod, namespace)) = self.selected_resource_info() else {
            return;
        };
        if kind != ResourceKind::Pods {
            self.toasts.push(ToastMessage::info("Port forward is only available for Pods"));
            return;
        }

        let key = (namespace.clone(), pod.clone());
        if let Some(forward_id) = self.pod_forward_index.remove(&key) {
            if let Some(forward) = self.active_forwards.remove(&forward_id) {
                let app_tx = self.app_tx.clone();
                let pod_name = pod.clone();
                tokio::spawn(async move {
                    let _ = forward.stop().await;
                    let _ = app_tx
                        .send(AppEvent::Toast(ToastMessage::success(format!("Stopped port-forward for {pod_name}"))));
                });
                self.refresh_port_forwards_panes();
                return;
            }
        }

        let Some(client) = &self.kube_client else {
            self.toasts.push(ToastMessage::error("No cluster connection"));
            return;
        };
        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();

        tokio::spawn(async move {
            let suggested_remote = detect_remote_port(&kube_client, &pod, &namespace).await.unwrap_or(80);
            let _ = app_tx.send(AppEvent::PortForwardPromptReady { pod, namespace, suggested_remote });
        });
    }

    pub(super) fn open_port_forward_prompt(&mut self, pod: String, namespace: String, suggested_remote: u16) {
        self.pending_port_forward = Some(PendingPortForward {
            pod,
            namespace,
            local_input: "0".into(),
            remote_input: suggested_remote.to_string(),
            active_field: PortForwardField::Local,
        });
        self.dispatcher.set_mode(InputMode::PortForwardInput);
    }

    pub(super) fn confirm_port_forward(&mut self) {
        let Some(pending) = self.pending_port_forward.take() else {
            return;
        };

        let local_input = pending.local_input.trim();
        let remote_input = pending.remote_input.trim();

        let local_port = if local_input.is_empty() {
            0
        } else {
            match local_input.parse::<u16>() {
                Ok(port) => port,
                Err(_) => {
                    self.toasts.push(ToastMessage::error("Local port must be 0-65535"));
                    self.pending_port_forward = Some(pending);
                    return;
                }
            }
        };

        let remote_port = match remote_input.parse::<u16>() {
            Ok(0) | Err(_) => {
                self.toasts.push(ToastMessage::error("Remote port must be 1-65535"));
                self.pending_port_forward = Some(pending);
                return;
            }
            Ok(port) => port,
        };

        let pod = pending.pod;
        let namespace = pending.namespace;
        self.dispatcher.set_mode(InputMode::Normal);

        let Some(client) = &self.kube_client else {
            self.toasts.push(ToastMessage::error("No cluster connection"));
            return;
        };
        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();

        tokio::spawn(async move {
            match kubetile_core::PortForward::start(&kube_client, &pod, &namespace, local_port, remote_port).await {
                Ok(forward) => {
                    let _ = app_tx.send(AppEvent::PortForwardReady { forward });
                }
                Err(e) => {
                    let _ = app_tx
                        .send(AppEvent::Toast(ToastMessage::error(format!("Port-forward failed for {pod}: {e}"))));
                }
            }
        });
    }

    pub(super) fn attach_port_forward(&mut self, forward: kubetile_core::PortForward) {
        let pod = forward.pod_name().to_string();
        let ns = forward.namespace().to_string();
        let remote = forward.remote_port();
        let local = forward.local_port();
        let id = forward.id();
        self.pod_forward_index.insert((ns, pod.clone()), id);
        self.active_forwards.insert(id, forward);
        self.refresh_port_forwards_panes();
        self.toasts.push(ToastMessage::success(format!("Forwarding {pod}:{remote} -> 127.0.0.1:{local}")));
    }

    pub(super) fn stop_all_port_forwards(&mut self) {
        let forwards: Vec<kubetile_core::PortForward> = self.active_forwards.drain().map(|(_, f)| f).collect();
        self.pod_forward_index.clear();
        self.refresh_port_forwards_panes();
        for forward in forwards {
            tokio::spawn(async move {
                let _ = forward.stop().await;
            });
        }
    }

    pub(super) fn refresh_port_forwards_panes(&mut self) {
        let mut rows: Vec<(ForwardId, String, String, u16, u16, Duration)> = self
            .active_forwards
            .values()
            .map(|f| {
                (f.id(), f.pod_name().to_string(), f.namespace().to_string(), f.local_port(), f.remote_port(), f.age())
            })
            .collect();
        rows.sort_by(|a, b| a.5.cmp(&b.5).reverse());

        for pane in self.panes.values_mut() {
            if let Some(pf) = pane.as_any_mut().downcast_mut::<PortForwardsPane>() {
                pf.set_items(rows.clone());
            }
        }
    }

    pub(super) fn stop_selected_port_forward(&mut self) {
        let focused = self.tab_manager.active().focused_pane;
        let Some(pane) = self.panes.get(&focused) else { return };
        let Some(pf_pane) = pane.as_any().downcast_ref::<PortForwardsPane>() else {
            return;
        };
        let Some(forward_id) = pf_pane.selected_forward_id() else { return };
        let Some(forward) = self.active_forwards.remove(&forward_id) else {
            return;
        };

        let key = (forward.namespace().to_string(), forward.pod_name().to_string());
        self.pod_forward_index.remove(&key);
        self.refresh_port_forwards_panes();
        let pod_name = forward.pod_name().to_string();
        let app_tx = self.app_tx.clone();
        tokio::spawn(async move {
            let _ = forward.stop().await;
            let _ = app_tx.send(AppEvent::Toast(ToastMessage::success(format!("Stopped port-forward for {pod_name}"))));
        });
    }
}

async fn detect_remote_port(client: &kube::Client, pod_name: &str, namespace: &str) -> Option<u16> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let pod = pods.get(pod_name).await.ok()?;
    let spec = pod.spec?;

    let mut all_ports: Vec<(u16, Option<String>)> = Vec::new();
    for container in spec.containers {
        if let Some(ports) = container.ports {
            for p in ports {
                if p.container_port <= 0 {
                    continue;
                }
                if let Ok(port) = u16::try_from(p.container_port) {
                    all_ports.push((port, p.name));
                }
            }
        }
    }

    if all_ports.is_empty() {
        return None;
    }

    for (port, name) in &all_ports {
        if name.as_deref().is_some_and(|n| n.contains("http") || n.contains("web")) {
            return Some(*port);
        }
    }

    for preferred in [80u16, 8080, 8000, 3000, 5000] {
        if all_ports.iter().any(|(p, _)| *p == preferred) {
            return Some(preferred);
        }
    }

    Some(all_ports[0].0)
}
