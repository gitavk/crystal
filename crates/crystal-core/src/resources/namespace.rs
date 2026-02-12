use std::time::Duration;

use k8s_openapi::api::core::v1::Namespace;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct NamespaceSummary {
    pub name: String,
    pub status: String,
    pub age: Duration,
}

impl ResourceSummary for NamespaceSummary {
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
        vec![("NAME", self.name.clone()), ("STATUS", self.status.clone()), ("AGE", format_duration(self.age))]
    }

    fn row(&self) -> Vec<String> {
        vec![self.name.clone(), self.status.clone(), format_duration(self.age)]
    }

    fn detail_sections(&self) -> Vec<DetailSection> {
        vec![
            DetailSection {
                title: "Metadata".into(),
                fields: vec![("Name".into(), self.name.clone()), ("Age".into(), format_duration(self.age))],
            },
            DetailSection { title: "Status".into(), fields: vec![("Phase".into(), self.status.clone())] },
        ]
    }
}

impl From<&Namespace> for NamespaceSummary {
    fn from(ns: &Namespace) -> Self {
        let meta = &ns.metadata;
        let name = meta.name.clone().unwrap_or_default();

        let status = ns.status.as_ref().and_then(|s| s.phase.as_deref()).unwrap_or("Active").to_string();

        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, status, age }
    }
}

impl From<Namespace> for NamespaceSummary {
    fn from(n: Namespace) -> Self {
        Self::from(&n)
    }
}
