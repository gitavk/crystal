use std::fmt::Debug;

use anyhow::Result;
use k8s_openapi::api::apps::v1::{Deployment, ReplicaSet, StatefulSet};
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
        let mut actions = vec![ResourceAction::Delete, ResourceAction::ViewYaml, ResourceAction::Describe];
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
        let pp = PatchParams::apply("kubetile");

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

    pub async fn resolve_owner_deployment(&self, pod_name: &str, ns: &str) -> Result<String> {
        use k8s_openapi::api::core::v1::Pod;

        let pods: Api<Pod> = Api::namespaced(self.client.clone(), ns);
        let pod = pods.get(pod_name).await?;

        let rs_name = pod
            .metadata
            .owner_references
            .as_ref()
            .and_then(|refs| refs.iter().find(|r| r.kind == "ReplicaSet"))
            .map(|r| r.name.clone())
            .ok_or_else(|| {
                anyhow::anyhow!("Pod '{pod_name}' has no ReplicaSet owner — is it managed by a Deployment?")
            })?;

        let rs_api: Api<ReplicaSet> = Api::namespaced(self.client.clone(), ns);
        let rs = rs_api.get(&rs_name).await?;

        let deploy_name = rs
            .metadata
            .owner_references
            .as_ref()
            .and_then(|refs| refs.iter().find(|r| r.kind == "Deployment"))
            .map(|r| r.name.clone())
            .ok_or_else(|| anyhow::anyhow!("ReplicaSet '{rs_name}' has no Deployment owner"))?;

        Ok(deploy_name)
    }

    pub async fn enter_debug_mode(&self, name: &str, ns: &str) -> Result<()> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
        let deploy = api.get(name).await?;

        let containers = deploy
            .spec
            .as_ref()
            .and_then(|s| s.template.spec.as_ref())
            .map(|s| &s.containers)
            .ok_or_else(|| anyhow::anyhow!("No containers found in deployment"))?;

        let container = containers.first().ok_or_else(|| anyhow::anyhow!("Deployment has no containers"))?;

        // If already in root debug mode, the deployment is running `sleep infinity` as root.
        // Reuse the already-saved originals so we don't overwrite them with corrupted state.
        let annotations = deploy.metadata.annotations.as_ref();
        let orig_command = annotations
            .and_then(|a| a.get("debug.kubetile.io/original-command"))
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| serde_json::to_string(&container.command))?;
        let orig_args = annotations
            .and_then(|a| a.get("debug.kubetile.io/original-args"))
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| serde_json::to_string(&container.args))?;

        let patch = serde_json::json!({
            "metadata": {
                "annotations": {
                    "debug.kubetile.io/original-command": orig_command,
                    "debug.kubetile.io/original-args": orig_args,
                }
            },
            "spec": {
                "template": {
                    "metadata": {
                        "annotations": {
                            "debug.kubetile.io/debug-mode": "true",
                        }
                    },
                    "spec": {
                        "containers": [{
                            "name": container.name,
                            "command": ["sleep", "infinity"],
                            "args": []
                        }]
                    }
                }
            }
        });

        api.patch(name, &PatchParams::default(), &Patch::Strategic(&patch)).await?;
        Ok(())
    }

    pub async fn exit_debug_mode(&self, name: &str, ns: &str) -> Result<()> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
        let deploy = api.get(name).await?;

        let annotations = deploy.metadata.annotations.as_ref();
        let orig_command_str =
            annotations.and_then(|a| a.get("debug.kubetile.io/original-command")).map(|s| s.as_str()).unwrap_or("null");
        let orig_args_str =
            annotations.and_then(|a| a.get("debug.kubetile.io/original-args")).map(|s| s.as_str()).unwrap_or("null");

        let containers = deploy
            .spec
            .as_ref()
            .and_then(|s| s.template.spec.as_ref())
            .map(|s| &s.containers)
            .ok_or_else(|| anyhow::anyhow!("No containers found in deployment"))?;

        let container = containers.first().ok_or_else(|| anyhow::anyhow!("Deployment has no containers"))?;

        let orig_command: serde_json::Value = serde_json::from_str(orig_command_str).unwrap_or(serde_json::Value::Null);
        let orig_args: serde_json::Value = serde_json::from_str(orig_args_str).unwrap_or(serde_json::Value::Null);

        let patch = serde_json::json!({
            "metadata": {
                "annotations": {
                    "debug.kubetile.io/original-command": null,
                    "debug.kubetile.io/original-args": null,
                }
            },
            "spec": {
                "template": {
                    "metadata": {
                        "annotations": {
                            "debug.kubetile.io/debug-mode": null,
                        }
                    },
                    "spec": {
                        "containers": [{
                            "name": container.name,
                            "command": orig_command,
                            "args": orig_args,
                        }]
                    }
                }
            }
        });

        api.patch(name, &PatchParams::default(), &Patch::Strategic(&patch)).await?;
        Ok(())
    }

    pub async fn is_in_debug_mode(&self, name: &str, ns: &str) -> Result<bool> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
        let deploy = api.get(name).await?;
        let in_debug = deploy.metadata.annotations.as_ref().is_some_and(|a| {
            a.contains_key("debug.kubetile.io/original-command") && !a.contains_key("debug.kubetile.io/root-debug-mode")
        });
        Ok(in_debug)
    }

    pub async fn enter_root_debug_mode(&self, name: &str, ns: &str) -> Result<()> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
        let deploy = api.get(name).await?;

        let containers = deploy
            .spec
            .as_ref()
            .and_then(|s| s.template.spec.as_ref())
            .map(|s| &s.containers)
            .ok_or_else(|| anyhow::anyhow!("No containers found in deployment"))?;

        let container = containers.first().ok_or_else(|| anyhow::anyhow!("Deployment has no containers"))?;

        // If already in regular debug mode, the deployment is running `sleep infinity`.
        // Reuse the already-saved originals so we don't overwrite them with corrupted state.
        let annotations = deploy.metadata.annotations.as_ref();
        let orig_command = annotations
            .and_then(|a| a.get("debug.kubetile.io/original-command"))
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| serde_json::to_string(&container.command))?;
        let orig_args = annotations
            .and_then(|a| a.get("debug.kubetile.io/original-args"))
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| serde_json::to_string(&container.args))?;
        let orig_security_context = serde_json::to_string(&container.security_context)?;

        let patch = serde_json::json!({
            "metadata": {
                "annotations": {
                    "debug.kubetile.io/original-command": orig_command,
                    "debug.kubetile.io/original-args": orig_args,
                    "debug.kubetile.io/original-security-context": orig_security_context,
                    "debug.kubetile.io/root-debug-mode": "true",
                }
            },
            "spec": {
                "template": {
                    "metadata": {
                        "annotations": {
                            "debug.kubetile.io/debug-mode": "true",
                        }
                    },
                    "spec": {
                        "containers": [{
                            "name": container.name,
                            "command": ["sleep", "infinity"],
                            "args": [],
                            "securityContext": {
                                "runAsUser": 0
                            }
                        }]
                    }
                }
            }
        });

        api.patch(name, &PatchParams::default(), &Patch::Strategic(&patch)).await?;
        Ok(())
    }

    pub async fn exit_root_debug_mode(&self, name: &str, ns: &str) -> Result<()> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
        let deploy = api.get(name).await?;

        let annotations = deploy.metadata.annotations.as_ref();
        let orig_command_str =
            annotations.and_then(|a| a.get("debug.kubetile.io/original-command")).map(|s| s.as_str()).unwrap_or("null");
        let orig_args_str =
            annotations.and_then(|a| a.get("debug.kubetile.io/original-args")).map(|s| s.as_str()).unwrap_or("null");
        let orig_security_context_str = annotations
            .and_then(|a| a.get("debug.kubetile.io/original-security-context"))
            .map(|s| s.as_str())
            .unwrap_or("null");

        let containers = deploy
            .spec
            .as_ref()
            .and_then(|s| s.template.spec.as_ref())
            .map(|s| &s.containers)
            .ok_or_else(|| anyhow::anyhow!("No containers found in deployment"))?;

        let container = containers.first().ok_or_else(|| anyhow::anyhow!("Deployment has no containers"))?;

        let orig_command: serde_json::Value = serde_json::from_str(orig_command_str).unwrap_or(serde_json::Value::Null);
        let orig_args: serde_json::Value = serde_json::from_str(orig_args_str).unwrap_or(serde_json::Value::Null);
        let orig_security_context: serde_json::Value =
            serde_json::from_str(orig_security_context_str).unwrap_or(serde_json::Value::Null);

        let patch = serde_json::json!({
            "metadata": {
                "annotations": {
                    "debug.kubetile.io/original-command": null,
                    "debug.kubetile.io/original-args": null,
                    "debug.kubetile.io/original-security-context": null,
                    "debug.kubetile.io/root-debug-mode": null,
                }
            },
            "spec": {
                "template": {
                    "metadata": {
                        "annotations": {
                            "debug.kubetile.io/debug-mode": null,
                        }
                    },
                    "spec": {
                        "containers": [{
                            "name": container.name,
                            "command": orig_command,
                            "args": orig_args,
                            "securityContext": orig_security_context,
                        }]
                    }
                }
            }
        });

        api.patch(name, &PatchParams::default(), &Patch::Strategic(&patch)).await?;
        Ok(())
    }

    pub async fn is_in_root_debug_mode(&self, name: &str, ns: &str) -> Result<bool> {
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
        let deploy = api.get(name).await?;
        let in_root_debug =
            deploy.metadata.annotations.as_ref().is_some_and(|a| a.contains_key("debug.kubetile.io/root-debug-mode"));
        Ok(in_root_debug)
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
        let pp = PatchParams::apply("kubetile");
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
