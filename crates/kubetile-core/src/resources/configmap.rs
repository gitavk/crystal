use std::time::Duration;

use k8s_openapi::api::core::v1::ConfigMap;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct ConfigMapSummary {
    pub name: String,
    pub namespace: String,
    pub data_count: usize,
    pub age: Duration,
}

impl ResourceSummary for ConfigMapSummary {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }

    fn status_display(&self) -> String {
        format!("{} keys", self.data_count)
    }

    fn age(&self) -> Duration {
        self.age
    }

    fn columns(&self) -> Vec<(&str, String)> {
        vec![
            ("NAME", self.name.clone()),
            ("NAMESPACE", self.namespace.clone()),
            ("DATA", self.data_count.to_string()),
            ("AGE", format_duration(self.age)),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![self.name.clone(), self.data_count.to_string(), format_duration(self.age)]
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
            DetailSection { title: "Data".into(), fields: vec![("Keys".into(), self.data_count.to_string())] },
        ]
    }
}

impl From<&ConfigMap> for ConfigMapSummary {
    fn from(cm: &ConfigMap) -> Self {
        let meta = &cm.metadata;
        let name = meta.name.clone().unwrap_or_default();
        let namespace = meta.namespace.clone().unwrap_or_else(|| "default".into());
        let data_count = cm.data.as_ref().map(|d| d.len()).unwrap_or(0);
        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, namespace, data_count, age }
    }
}

impl From<ConfigMap> for ConfigMapSummary {
    fn from(c: ConfigMap) -> Self {
        Self::from(&c)
    }
}
