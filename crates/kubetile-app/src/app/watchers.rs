use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, StatefulSet};
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::api::core::v1::{
    ConfigMap, Namespace, Node, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service,
};
use k8s_openapi::api::networking::v1::Ingress;
use kube::Api;
use tokio::sync::mpsc;

use kubetile_core::informer::{ResourceEvent, ResourceWatcher};
use kubetile_core::resource::ResourceSummary;
use kubetile_core::*;
use kubetile_tui::pane::{PaneId, ResourceKind};

use crate::event::AppEvent;

use super::App;

impl App {
    pub(super) fn start_watcher_for_pane(&mut self, pane_id: PaneId, kind: &ResourceKind, namespace: &str) {
        self.active_watchers.remove(&pane_id);
        let watcher_seq = self.watcher_seq_by_pane.get(&pane_id).copied().unwrap_or(0).wrapping_add(1);
        self.watcher_seq_by_pane.insert(pane_id, watcher_seq);

        let Some(client) = &self.kube_client else {
            return;
        };

        let kube_client = client.inner_client();
        let app_tx = self.app_tx.clone();

        fn spawn_bridge<S>(
            pane_id: PaneId,
            watcher_seq: u64,
            mut rx: mpsc::Receiver<ResourceEvent<S>>,
            app_tx: mpsc::UnboundedSender<AppEvent>,
        ) where
            S: ResourceSummary + 'static,
        {
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    let app_event = match event {
                        ResourceEvent::Updated(items) => {
                            let headers = if items.is_empty() {
                                vec![]
                            } else {
                                items[0].columns().into_iter().map(|(h, _)| h.to_string()).collect()
                            };
                            let rows = items.iter().map(|item| item.row()).collect();
                            AppEvent::ResourceUpdate { pane_id, watcher_seq, headers, rows }
                        }
                        ResourceEvent::Error(error) => AppEvent::ResourceError { pane_id, watcher_seq, error },
                    };
                    if app_tx.send(app_event).is_err() {
                        break;
                    }
                }
            });
        }

        let all_ns = namespace.is_empty();

        macro_rules! spawn_watcher {
            ($k8s_type:ty, $summary_type:ty) => {{
                let api: Api<$k8s_type> = if all_ns {
                    Api::all(kube_client.clone())
                } else {
                    Api::namespaced(kube_client.clone(), namespace)
                };
                let (tx, rx) = mpsc::channel(16);
                let watcher = ResourceWatcher::watch::<$k8s_type, $summary_type>(api, tx);
                self.active_watchers.insert(pane_id, watcher);
                spawn_bridge(pane_id, watcher_seq, rx, app_tx);
            }};
            (cluster $k8s_type:ty, $summary_type:ty) => {{
                let api: Api<$k8s_type> = Api::all(kube_client.clone());
                let (tx, rx) = mpsc::channel(16);
                let watcher = ResourceWatcher::watch::<$k8s_type, $summary_type>(api, tx);
                self.active_watchers.insert(pane_id, watcher);
                spawn_bridge(pane_id, watcher_seq, rx, app_tx);
            }};
        }

        match kind {
            ResourceKind::Pods => spawn_watcher!(Pod, PodSummary),
            ResourceKind::Deployments => spawn_watcher!(Deployment, DeploymentSummary),
            ResourceKind::Services => spawn_watcher!(Service, ServiceSummary),
            ResourceKind::StatefulSets => spawn_watcher!(StatefulSet, StatefulSetSummary),
            ResourceKind::DaemonSets => spawn_watcher!(DaemonSet, DaemonSetSummary),
            ResourceKind::Jobs => spawn_watcher!(Job, JobSummary),
            ResourceKind::CronJobs => spawn_watcher!(CronJob, CronJobSummary),
            ResourceKind::ConfigMaps => spawn_watcher!(ConfigMap, ConfigMapSummary),
            ResourceKind::Secrets => spawn_watcher!(Secret, SecretSummary),
            ResourceKind::Ingresses => spawn_watcher!(Ingress, IngressSummary),
            ResourceKind::Nodes => spawn_watcher!(cluster Node, NodeSummary),
            ResourceKind::Namespaces => spawn_watcher!(cluster Namespace, NamespaceSummary),
            ResourceKind::PersistentVolumes => spawn_watcher!(cluster PersistentVolume, PersistentVolumeSummary),
            ResourceKind::PersistentVolumeClaims => {
                spawn_watcher!(PersistentVolumeClaim, PersistentVolumeClaimSummary)
            }
            ResourceKind::Custom(_) => {
                tracing::warn!("Custom resource kinds are not yet supported");
            }
        }
    }
}
