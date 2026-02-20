use std::time::Duration;

use k8s_openapi::api::core::v1::Node;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct NodeSummary {
    pub name: String,
    pub status: String,
    pub roles: String,
    pub age: Duration,
    pub version: String,
}

impl ResourceSummary for NodeSummary {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> Option<&str> {
        None
    }

    fn status_display(&self) -> String {
        self.status.clone()
    }

    fn age(&self) -> Duration {
        self.age
    }

    fn columns(&self) -> Vec<(&str, String)> {
        vec![
            ("NAME", self.name.clone()),
            ("STATUS", self.status.clone()),
            ("ROLES", self.roles.clone()),
            ("AGE", format_duration(self.age)),
            ("VERSION", self.version.clone()),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.status.clone(),
            self.roles.clone(),
            format_duration(self.age),
            self.version.clone(),
        ]
    }

    fn detail_sections(&self) -> Vec<DetailSection> {
        vec![
            DetailSection {
                title: "Metadata".into(),
                fields: vec![
                    ("Name".into(), self.name.clone()),
                    ("Roles".into(), self.roles.clone()),
                    ("Age".into(), format_duration(self.age)),
                ],
            },
            DetailSection { title: "Status".into(), fields: vec![("Status".into(), self.status.clone())] },
            DetailSection { title: "Info".into(), fields: vec![("Version".into(), self.version.clone())] },
        ]
    }
}

impl From<&Node> for NodeSummary {
    fn from(node: &Node) -> Self {
        let meta = &node.metadata;
        let name = meta.name.clone().unwrap_or_default();

        let status = node
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref())
            .and_then(|conditions| {
                conditions.iter().find(|c| c.type_ == "Ready").map(|c| {
                    if c.status == "True" {
                        "Ready".to_string()
                    } else {
                        "NotReady".to_string()
                    }
                })
            })
            .unwrap_or_else(|| "Unknown".into());

        let roles = meta
            .labels
            .as_ref()
            .map(|labels| {
                let mut roles: Vec<&str> =
                    labels.keys().filter_map(|k| k.strip_prefix("node-role.kubernetes.io/")).collect();
                roles.sort();
                if roles.is_empty() {
                    "<none>".to_string()
                } else {
                    roles.join(",")
                }
            })
            .unwrap_or_else(|| "<none>".into());

        let version = node
            .status
            .as_ref()
            .and_then(|s| s.node_info.as_ref())
            .map(|info| info.kubelet_version.clone())
            .unwrap_or_default();

        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, status, roles, age, version }
    }
}

impl From<Node> for NodeSummary {
    fn from(n: Node) -> Self {
        Self::from(&n)
    }
}
