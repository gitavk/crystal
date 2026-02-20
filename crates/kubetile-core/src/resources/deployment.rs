use std::time::Duration;

use k8s_openapi::api::apps::v1::Deployment;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct DeploymentSummary {
    pub name: String,
    pub namespace: String,
    pub ready: String,
    pub up_to_date: i32,
    pub available: i32,
    pub age: Duration,
}

impl ResourceSummary for DeploymentSummary {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }

    fn status_display(&self) -> String {
        self.ready.clone()
    }

    fn age(&self) -> Duration {
        self.age
    }

    fn columns(&self) -> Vec<(&str, String)> {
        vec![
            ("NAME", self.name.clone()),
            ("NAMESPACE", self.namespace.clone()),
            ("READY", self.ready.clone()),
            ("UP-TO-DATE", self.up_to_date.to_string()),
            ("AVAILABLE", self.available.to_string()),
            ("AGE", format_duration(self.age)),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.ready.clone(),
            self.up_to_date.to_string(),
            self.available.to_string(),
            format_duration(self.age),
        ]
    }

    fn detail_sections(&self) -> Vec<DetailSection> {
        vec![
            DetailSection {
                title: "Metadata".into(),
                fields: vec![
                    ("Name".into(), self.name.clone()),
                    ("Namespace".into(), self.namespace.clone()),
                    ("Age".into(), format_duration(self.age)),
                ],
            },
            DetailSection {
                title: "Status".into(),
                fields: vec![
                    ("Ready".into(), self.ready.clone()),
                    ("Up-to-date".into(), self.up_to_date.to_string()),
                    ("Available".into(), self.available.to_string()),
                ],
            },
        ]
    }
}

impl From<&Deployment> for DeploymentSummary {
    fn from(deploy: &Deployment) -> Self {
        let meta = &deploy.metadata;
        let name = meta.name.clone().unwrap_or_default();
        let namespace = meta.namespace.clone().unwrap_or_else(|| "default".into());

        let status = deploy.status.as_ref();
        let replicas = status.and_then(|s| s.replicas).unwrap_or(0);
        let ready_replicas = status.and_then(|s| s.ready_replicas).unwrap_or(0);
        let up_to_date = status.and_then(|s| s.updated_replicas).unwrap_or(0);
        let available = status.and_then(|s| s.available_replicas).unwrap_or(0);

        let ready = format!("{ready_replicas}/{replicas}");
        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, namespace, ready, up_to_date, available, age }
    }
}

impl From<Deployment> for DeploymentSummary {
    fn from(d: Deployment) -> Self {
        Self::from(&d)
    }
}
