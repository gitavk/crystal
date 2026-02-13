use std::fmt::Debug;

use anyhow::Result;
use k8s_openapi::api::apps::v1::{Deployment, StatefulSet};
use k8s_openapi::api::core::v1::Event;
use k8s_openapi::NamespaceResourceScope;
use kube::api::{Api, DeleteParams, ListParams, Patch, PatchParams};
use kube::{Client, Resource};
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Pods,
    Deployments,
    Services,
    StatefulSets,
    DaemonSets,
    Jobs,
    CronJobs,
    ConfigMaps,
    Secrets,
    Ingresses,
    Nodes,
    Namespaces,
    PersistentVolumes,
    PersistentVolumeClaims,
    Custom(String),
}

impl ResourceKind {
    pub fn short_name(&self) -> &str {
        match self {
            Self::Pods => "po",
            Self::Deployments => "deploy",
            Self::Services => "svc",
            Self::StatefulSets => "sts",
            Self::DaemonSets => "ds",
            Self::Jobs => "job",
            Self::CronJobs => "cj",
            Self::ConfigMaps => "cm",
            Self::Secrets => "secret",
            Self::Ingresses => "ing",
            Self::Nodes => "no",
            Self::Namespaces => "ns",
            Self::PersistentVolumes => "pv",
            Self::PersistentVolumeClaims => "pvc",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Pods => "Pods",
            Self::Deployments => "Deployments",
            Self::Services => "Services",
            Self::StatefulSets => "StatefulSets",
            Self::DaemonSets => "DaemonSets",
            Self::Jobs => "Jobs",
            Self::CronJobs => "CronJobs",
            Self::ConfigMaps => "ConfigMaps",
            Self::Secrets => "Secrets",
            Self::Ingresses => "Ingresses",
            Self::Nodes => "Nodes",
            Self::Namespaces => "Namespaces",
            Self::PersistentVolumes => "PersistentVolumes",
            Self::PersistentVolumeClaims => "PersistentVolumeClaims",
            Self::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceAction {
    Delete,
    ViewYaml,
    Describe,
    ViewLogs,
    Exec,
    Scale(i32),
    RestartRollout,
}

impl ResourceAction {
    pub fn available_for(kind: &ResourceKind) -> Vec<ResourceAction> {
        let mut actions = vec![
            ResourceAction::Delete,
            ResourceAction::ViewYaml,
            ResourceAction::Describe,
        ];
        match kind {
            ResourceKind::Pods => {
                actions.push(ResourceAction::ViewLogs);
                actions.push(ResourceAction::Exec);
            }
            ResourceKind::Deployments => {
                actions.push(ResourceAction::Scale(0));
                actions.push(ResourceAction::RestartRollout);
            }
            ResourceKind::StatefulSets => {
                actions.push(ResourceAction::Scale(0));
            }
            _ => {}
        }
        actions
    }
}

pub struct ActionExecutor {
    client: Client,
}

impl ActionExecutor {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn delete<K>(&self, name: &str, ns: &str) -> Result<()>
    where
        K: Resource<DynamicType = (), Scope = NamespaceResourceScope> + Clone + DeserializeOwned + Debug,
    {
        let api: Api<K> = Api::namespaced(self.client.clone(), ns);
        let dp = DeleteParams::default();
        api.delete(name, &dp).await?;
        Ok(())
    }

    pub async fn delete_cluster<K>(&self, name: &str) -> Result<()>
    where
        K: Resource<DynamicType = ()> + Clone + DeserializeOwned + Debug,
    {
        let api: Api<K> = Api::all(self.client.clone());
        let dp = DeleteParams::default();
        api.delete(name, &dp).await?;
        Ok(())
    }

    pub async fn scale(&self, kind: &ResourceKind, name: &str, ns: &str, replicas: i32) -> Result<()> {
        let patch = serde_json::json!({
            "spec": { "replicas": replicas }
        });
        let pp = PatchParams::apply("crystal");

        match kind {
            ResourceKind::Deployments => {
                let api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
                api.patch(name, &pp, &Patch::Merge(&patch)).await?;
            }
            ResourceKind::StatefulSets => {
                let api: Api<StatefulSet> = Api::namespaced(self.client.clone(), ns);
                api.patch(name, &pp, &Patch::Merge(&patch)).await?;
            }
            _ => anyhow::bail!("Scale not supported for {:?}", kind),
        }
        Ok(())
    }

    pub async fn restart_rollout(&self, name: &str, ns: &str) -> Result<()> {
        let now = jiff::Timestamp::now().to_string();
        let patch = serde_json::json!({
            "spec": {
                "template": {
                    "metadata": {
                        "annotations": {
                            "kubectl.kubernetes.io/restartedAt": now
                        }
                    }
                }
            }
        });
        let pp = PatchParams::apply("crystal");
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
        api.patch(name, &pp, &Patch::Merge(&patch)).await?;
        Ok(())
    }

    pub async fn get_yaml<K>(&self, name: &str, ns: &str) -> Result<String>
    where
        K: Resource<DynamicType = (), Scope = NamespaceResourceScope> + Clone + DeserializeOwned + Serialize + Debug,
    {
        let api: Api<K> = Api::namespaced(self.client.clone(), ns);
        let obj = api.get(name).await?;
        let yaml = serde_yaml::to_string(&obj)?;
        Ok(yaml)
    }

    pub async fn get_yaml_cluster<K>(&self, name: &str) -> Result<String>
    where
        K: Resource<DynamicType = ()> + Clone + DeserializeOwned + Serialize + Debug,
    {
        let api: Api<K> = Api::all(self.client.clone());
        let obj = api.get(name).await?;
        let yaml = serde_yaml::to_string(&obj)?;
        Ok(yaml)
    }

    pub async fn describe<K>(&self, name: &str, ns: &str) -> Result<String>
    where
        K: Resource<DynamicType = (), Scope = NamespaceResourceScope> + Clone + DeserializeOwned + Debug,
    {
        let api: Api<K> = Api::namespaced(self.client.clone(), ns);
        let obj = api.get(name).await?;

        let events_api: Api<Event> = Api::namespaced(self.client.clone(), ns);
        let lp = ListParams::default().fields(&format!("involvedObject.name={}", name));
        let events = events_api.list(&lp).await?;

        let mut output = String::new();
        output.push_str(&format!("Name: {}\n", name));
        output.push_str(&format!("Namespace: {}\n", ns));
        output.push_str(&format!("Resource: {:?}\n", obj));
        output.push_str("\n--- Events ---\n");

        let mut event_list: Vec<_> = events.items.into_iter().collect();
        event_list.sort_by(|a, b| {
            let a_time = a.last_timestamp.as_ref().map(|t| &t.0);
            let b_time = b.last_timestamp.as_ref().map(|t| &t.0);
            a_time.cmp(&b_time)
        });

        for event in &event_list {
            let kind = event.type_.as_deref().unwrap_or("Unknown");
            let reason = event.reason.as_deref().unwrap_or("");
            let message = event.message.as_deref().unwrap_or("");
            output.push_str(&format!("  {:<10} {:<20} {}\n", kind, reason, message));
        }

        if event_list.is_empty() {
            output.push_str("  <none>\n");
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_for_pods_includes_logs_and_exec() {
        let actions = ResourceAction::available_for(&ResourceKind::Pods);
        assert!(actions.contains(&ResourceAction::ViewLogs));
        assert!(actions.contains(&ResourceAction::Exec));
        assert!(actions.contains(&ResourceAction::Delete));
        assert!(actions.contains(&ResourceAction::ViewYaml));
        assert!(actions.contains(&ResourceAction::Describe));
    }

    #[test]
    fn available_for_deployments_includes_scale_and_restart() {
        let actions = ResourceAction::available_for(&ResourceKind::Deployments);
        assert!(actions.contains(&ResourceAction::Scale(0)));
        assert!(actions.contains(&ResourceAction::RestartRollout));
    }

    #[test]
    fn available_for_statefulsets_includes_scale() {
        let actions = ResourceAction::available_for(&ResourceKind::StatefulSets);
        assert!(actions.contains(&ResourceAction::Scale(0)));
        assert!(!actions.contains(&ResourceAction::RestartRollout));
    }

    #[test]
    fn available_for_configmaps_excludes_scale_logs_exec() {
        let actions = ResourceAction::available_for(&ResourceKind::ConfigMaps);
        assert!(!actions.contains(&ResourceAction::Scale(0)));
        assert!(!actions.contains(&ResourceAction::ViewLogs));
        assert!(!actions.contains(&ResourceAction::Exec));
        assert!(!actions.contains(&ResourceAction::RestartRollout));
    }

    #[test]
    fn base_actions_always_present() {
        let kinds = [
            ResourceKind::Pods,
            ResourceKind::Deployments,
            ResourceKind::Services,
            ResourceKind::ConfigMaps,
            ResourceKind::Nodes,
        ];
        for kind in &kinds {
            let actions = ResourceAction::available_for(kind);
            assert!(actions.contains(&ResourceAction::Delete), "Delete missing for {:?}", kind);
            assert!(actions.contains(&ResourceAction::ViewYaml), "ViewYaml missing for {:?}", kind);
            assert!(actions.contains(&ResourceAction::Describe), "Describe missing for {:?}", kind);
        }
    }
}
