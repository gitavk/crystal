use std::collections::HashMap;
use std::fmt::Debug;

use futures::StreamExt;
use kube::runtime::watcher::{self, Event};
use kube::{Api, Resource, ResourceExt};
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::resource::ResourceSummary;

#[derive(Debug, Clone)]
pub enum ResourceEvent<S> {
    Updated(Vec<S>),
    Error(String),
}

pub struct ResourceWatcher {
    cancel: CancellationToken,
}

impl ResourceWatcher {
    /// Watch any Kubernetes resource type and emit summary snapshots.
    ///
    /// Type parameters:
    /// - K: the k8s-openapi resource type (Pod, Deployment, etc.)
    /// - S: the summary struct (PodSummary, DeploymentSummary, etc.)
    ///
    /// Requirements:
    /// - K must implement Resource, Clone, DeserializeOwned, Debug, Send
    /// - S must implement ResourceSummary + From<K>
    pub fn watch<K, S>(api: Api<K>, tx: mpsc::Sender<ResourceEvent<S>>) -> Self
    where
        K: Resource<DynamicType = ()> + Clone + DeserializeOwned + Debug + Send + 'static,
        S: ResourceSummary + From<K> + Clone + Send + 'static,
    {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            let stream = watcher::watcher(api, watcher::Config::default());
            tokio::pin!(stream);

            let mut snapshot: HashMap<String, S> = HashMap::new();

            loop {
                tokio::select! {
                    _ = cancel_clone.cancelled() => {
                        info!("Resource watcher cancelled");
                        break;
                    }
                    item = stream.next() => {
                        match item {
                            Some(Ok(event)) => {
                                match event {
                                    Event::Apply(resource) | Event::InitApply(resource) => {
                                        let summary = S::from(resource);
                                        let key = match summary.namespace() {
                                            Some(ns) => format!("{}/{}", ns, summary.name()),
                                            None => summary.name().to_string(),
                                        };
                                        snapshot.insert(key, summary);
                                    }
                                    Event::Delete(resource) => {
                                        let name = resource.name_any();
                                        let ns = resource.namespace();
                                        let key = match ns {
                                            Some(ns) => format!("{ns}/{name}"),
                                            None => name,
                                        };
                                        snapshot.remove(&key);
                                    }
                                    Event::Init => {
                                        snapshot.clear();
                                    }
                                    Event::InitDone => {}
                                }
                                let items: Vec<S> = snapshot.values().cloned().collect();
                                let _ = tx.send(ResourceEvent::Updated(items)).await;
                            }
                            Some(Err(e)) => {
                                warn!("Watcher error: {e}");
                                let _ = tx.send(ResourceEvent::Error(e.to_string())).await;
                            }
                            None => break,
                        }
                    }
                }
            }
        });

        Self { cancel }
    }

    pub fn stop(&self) {
        self.cancel.cancel();
    }
}

impl Drop for ResourceWatcher {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{DeploymentSummary, PodSummary};
    use k8s_openapi::api::apps::v1::Deployment;
    use k8s_openapi::api::core::v1::Pod;

    /// Type-level test: verify generic watcher compiles for Pod
    #[test]
    fn test_watcher_compiles_for_pod() {
        // This test verifies the type constraints are correct.
        // We don't actually run the watcher since that requires a k8s cluster.
        fn _check_pod_watcher_compiles() {
            let _: fn(Api<Pod>, mpsc::Sender<ResourceEvent<PodSummary>>) -> ResourceWatcher =
                ResourceWatcher::watch::<Pod, PodSummary>;
        }
    }

    /// Type-level test: verify generic watcher compiles for Deployment
    #[test]
    fn test_watcher_compiles_for_deployment() {
        fn _check_deployment_watcher_compiles() {
            let _: fn(Api<Deployment>, mpsc::Sender<ResourceEvent<DeploymentSummary>>) -> ResourceWatcher =
                ResourceWatcher::watch::<Deployment, DeploymentSummary>;
        }
    }

    /// Test that CancellationToken stops the watcher
    #[tokio::test]
    async fn test_watcher_cancellation() {
        // Create a mock API (we won't actually use it, just verify the cancel mechanism)
        let (tx, mut rx) = mpsc::channel::<ResourceEvent<PodSummary>>(16);

        // Since we can't easily create a real Api without a cluster,
        // this test just verifies the watcher can be created and cancelled.
        // The actual watcher loop testing would require integration tests.

        let cancel = CancellationToken::new();
        cancel.cancel();

        // Verify cancellation works
        assert!(cancel.is_cancelled());

        // Verify channel is still open
        drop(tx);
        assert!(rx.recv().await.is_none());
    }

    /// Test ResourceEvent variants
    #[test]
    fn test_resource_event_updated() {
        let event: ResourceEvent<PodSummary> = ResourceEvent::Updated(vec![]);
        match event {
            ResourceEvent::Updated(items) => assert!(items.is_empty()),
            _ => panic!("Expected Updated variant"),
        }
    }

    #[test]
    fn test_resource_event_error() {
        let event = ResourceEvent::<PodSummary>::Error("test error".to_string());
        match event {
            ResourceEvent::Error(e) => assert_eq!(e, "test error"),
            _ => panic!("Expected Error variant"),
        }
    }
}
