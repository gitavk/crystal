use std::time::Duration;

use k8s_openapi::api::apps::v1::DaemonSet;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct DaemonSetSummary {
    pub name: String,
    pub namespace: String,
    pub desired: i32,
    pub current: i32,
    pub ready: i32,
    pub age: Duration,
}

impl ResourceSummary for DaemonSetSummary {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }

    fn status_display(&self) -> String {
        format!("{}/{}", self.ready, self.desired)
    }

    fn age(&self) -> Duration {
        self.age
    }

    fn columns(&self) -> Vec<(&str, String)> {
        vec![
            ("NAME", self.name.clone()),
            ("NAMESPACE", self.namespace.clone()),
            ("DESIRED", self.desired.to_string()),
            ("CURRENT", self.current.to_string()),
            ("READY", self.ready.to_string()),
            ("AGE", format_duration(self.age)),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.desired.to_string(),
            self.current.to_string(),
            self.ready.to_string(),
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
                    ("Desired".into(), self.desired.to_string()),
                    ("Current".into(), self.current.to_string()),
                    ("Ready".into(), self.ready.to_string()),
                ],
            },
        ]
    }
}

impl From<&DaemonSet> for DaemonSetSummary {
    fn from(ds: &DaemonSet) -> Self {
        let meta = &ds.metadata;
        let name = meta.name.clone().unwrap_or_default();
        let namespace = meta.namespace.clone().unwrap_or_else(|| "default".into());

        let status = ds.status.as_ref();
        let desired = status.map(|s| s.desired_number_scheduled).unwrap_or(0);
        let current = status.map(|s| s.current_number_scheduled).unwrap_or(0);
        let ready = status.map(|s| s.number_ready).unwrap_or(0);

        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, namespace, desired, current, ready, age }
    }
}

impl From<DaemonSet> for DaemonSetSummary {
    fn from(d: DaemonSet) -> Self {
        Self::from(&d)
    }
}
