use std::collections::HashMap;

use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::runtime::watcher::{self, Event};
use kube::Api;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::resources::PodSummary;

#[derive(Debug, Clone)]
pub enum ResourceEvent<S> {
    Updated(Vec<S>),
    Error(String),
}

pub struct ResourceWatcher {
    cancel: CancellationToken,
}

impl ResourceWatcher {
    pub fn watch_pods(api: Api<Pod>, tx: mpsc::Sender<ResourceEvent<PodSummary>>) -> Self {
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        tokio::spawn(async move {
            let stream = watcher::watcher(api, watcher::Config::default());
            tokio::pin!(stream);

            let mut pods: HashMap<String, PodSummary> = HashMap::new();

            loop {
                tokio::select! {
                    _ = cancel_clone.cancelled() => {
                        info!("Pod watcher cancelled");
                        break;
                    }
                    item = stream.next() => {
                        match item {
                            Some(Ok(event)) => {
                                match event {
                                    Event::Apply(pod) | Event::InitApply(pod) => {
                                        let summary = PodSummary::from(&pod);
                                        let key = format!("{}/{}", summary.namespace, summary.name);
                                        pods.insert(key, summary);
                                    }
                                    Event::Delete(pod) => {
                                        let name = pod.metadata.name.as_deref().unwrap_or_default();
                                        let ns = pod.metadata.namespace.as_deref().unwrap_or("default");
                                        let key = format!("{ns}/{name}");
                                        pods.remove(&key);
                                    }
                                    Event::Init => {
                                        pods.clear();
                                    }
                                    Event::InitDone => {}
                                }
                                let snapshot: Vec<PodSummary> = pods.values().cloned().collect();
                                let _ = tx.send(ResourceEvent::Updated(snapshot)).await;
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
