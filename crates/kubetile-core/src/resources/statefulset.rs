use std::time::Duration;

use k8s_openapi::api::apps::v1::StatefulSet;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct StatefulSetSummary {
    pub name: String,
    pub namespace: String,
    pub ready: String,
    pub age: Duration,
}

impl ResourceSummary for StatefulSetSummary {
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
            ("AGE", format_duration(self.age)),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![self.name.clone(), self.ready.clone(), format_duration(self.age)]
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
            DetailSection { title: "Status".into(), fields: vec![("Ready".into(), self.ready.clone())] },
        ]
    }
}

impl From<&StatefulSet> for StatefulSetSummary {
    fn from(sts: &StatefulSet) -> Self {
        let meta = &sts.metadata;
        let name = meta.name.clone().unwrap_or_default();
        let namespace = meta.namespace.clone().unwrap_or_else(|| "default".into());

        let status = sts.status.as_ref();
        let replicas = status.map(|s| s.replicas).unwrap_or(0);
        let ready_replicas = status.and_then(|s| s.ready_replicas).unwrap_or(0);
        let ready = format!("{ready_replicas}/{replicas}");

        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, namespace, ready, age }
    }
}

impl From<StatefulSet> for StatefulSetSummary {
    fn from(s: StatefulSet) -> Self {
        Self::from(&s)
    }
}
