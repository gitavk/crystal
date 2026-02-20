use std::fmt;
use std::time::Duration;

use k8s_openapi::api::core::v1::Pod;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone, PartialEq)]
pub enum PodPhase {
    Running,
    Pending,
    Succeeded,
    Failed,
    Unknown,
}

impl fmt::Display for PodPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running => write!(f, "Running"),
            Self::Pending => write!(f, "Pending"),
            Self::Succeeded => write!(f, "Succeeded"),
            Self::Failed => write!(f, "Failed"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PodSummary {
    pub name: String,
    pub namespace: String,
    pub uid: Option<String>,
    pub status: PodPhase,
    pub ready: String,
    pub restarts: i32,
    pub age: Duration,
    pub node: Option<String>,
}

impl ResourceSummary for PodSummary {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }

    fn status_display(&self) -> String {
        self.status.to_string()
    }

    fn age(&self) -> Duration {
        self.age
    }

    fn columns(&self) -> Vec<(&str, String)> {
        vec![
            ("NAME", self.name.clone()),
            ("NAMESPACE", self.namespace.clone()),
            ("STATUS", self.status.to_string()),
            ("READY", self.ready.clone()),
            ("RESTARTS", self.restarts.to_string()),
            ("AGE", format_duration(self.age)),
            ("NODE", self.node.clone().unwrap_or_default()),
            ("UID", self.uid.clone().unwrap_or_default()),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.namespace.clone(),
            self.ready.clone(),
            self.status.to_string(),
            self.restarts.to_string(),
            format_duration(self.age),
            self.node.clone().unwrap_or_default(),
            self.uid.clone().unwrap_or_default(),
        ]
    }

    fn detail_sections(&self) -> Vec<DetailSection> {
        let mut metadata = vec![
            ("Name".into(), self.name.clone()),
            ("Namespace".into(), self.namespace.clone()),
            ("Status".into(), self.status.to_string()),
            ("Age".into(), format_duration(self.age)),
        ];
        if let Some(node) = &self.node {
            metadata.push(("Node".into(), node.clone()));
        }

        let status_section = vec![("Ready".into(), self.ready.clone()), ("Restarts".into(), self.restarts.to_string())];

        vec![
            DetailSection { title: "Metadata".into(), fields: metadata },
            DetailSection { title: "Status".into(), fields: status_section },
        ]
    }
}

impl From<&Pod> for PodSummary {
    fn from(pod: &Pod) -> Self {
        let metadata = &pod.metadata;
        let name = metadata.name.clone().unwrap_or_default();
        let namespace = metadata.namespace.clone().unwrap_or_else(|| "default".into());
        let uid = metadata.uid.clone();

        let status = pod
            .status
            .as_ref()
            .and_then(|s| s.phase.as_deref())
            .map(|p| match p {
                "Running" => PodPhase::Running,
                "Pending" => PodPhase::Pending,
                "Succeeded" => PodPhase::Succeeded,
                "Failed" => PodPhase::Failed,
                _ => PodPhase::Unknown,
            })
            .unwrap_or(PodPhase::Unknown);

        let container_statuses = pod.status.as_ref().and_then(|s| s.container_statuses.as_ref());

        let (ready_count, total_count) = container_statuses
            .map(|cs| {
                let total = cs.len();
                let ready = cs.iter().filter(|c| c.ready).count();
                (ready, total)
            })
            .unwrap_or((0, 0));
        let ready = format!("{ready_count}/{total_count}");

        let restarts = container_statuses.map(|cs| cs.iter().map(|c| c.restart_count).sum()).unwrap_or(0);

        let age = calculate_age(metadata.creation_timestamp.as_ref());

        let node = pod.spec.as_ref().and_then(|s| s.node_name.clone());

        Self { name, namespace, uid, status, ready, restarts, age, node }
    }
}

impl From<Pod> for PodSummary {
    fn from(pod: Pod) -> Self {
        Self::from(&pod)
    }
}
