use std::time::Duration;

use k8s_openapi::api::core::v1::PersistentVolumeClaim;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct PersistentVolumeClaimSummary {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub volume: String,
    pub capacity: String,
    pub access_modes: String,
    pub storage_class: String,
    pub age: Duration,
}

impl ResourceSummary for PersistentVolumeClaimSummary {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
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
            ("NAMESPACE", self.namespace.clone()),
            ("STATUS", self.status.clone()),
            ("VOLUME", self.volume.clone()),
            ("CAPACITY", self.capacity.clone()),
            ("ACCESS MODES", self.access_modes.clone()),
            ("STORAGECLASS", self.storage_class.clone()),
            ("AGE", format_duration(self.age)),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.status.clone(),
            self.volume.clone(),
            self.capacity.clone(),
            self.access_modes.clone(),
            self.storage_class.clone(),
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
                title: "Spec".into(),
                fields: vec![
                    ("Access Modes".into(), self.access_modes.clone()),
                    ("Storage Class".into(), self.storage_class.clone()),
                ],
            },
            DetailSection {
                title: "Status".into(),
                fields: vec![
                    ("Phase".into(), self.status.clone()),
                    ("Volume".into(), self.volume.clone()),
                    ("Capacity".into(), self.capacity.clone()),
                ],
            },
        ]
    }
}

fn format_access_modes(modes: &[String]) -> String {
    modes
        .iter()
        .map(|m| match m.as_str() {
            "ReadWriteOnce" => "RWO",
            "ReadOnlyMany" => "ROX",
            "ReadWriteMany" => "RWX",
            "ReadWriteOncePod" => "RWOP",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(",")
}

impl From<&PersistentVolumeClaim> for PersistentVolumeClaimSummary {
    fn from(pvc: &PersistentVolumeClaim) -> Self {
        let meta = &pvc.metadata;
        let name = meta.name.clone().unwrap_or_default();
        let namespace = meta.namespace.clone().unwrap_or_else(|| "default".into());

        let status = pvc.status.as_ref().and_then(|s| s.phase.as_deref()).unwrap_or("Pending").to_string();

        let volume = pvc.spec.as_ref().and_then(|s| s.volume_name.clone()).unwrap_or_default();

        let capacity = pvc
            .status
            .as_ref()
            .and_then(|s| s.capacity.as_ref())
            .and_then(|c| c.get("storage"))
            .map(|q| q.0.clone())
            .unwrap_or_default();

        let access_modes = pvc
            .status
            .as_ref()
            .and_then(|s| s.access_modes.as_ref())
            .map(|m| format_access_modes(m))
            .unwrap_or_default();

        let storage_class = pvc.spec.as_ref().and_then(|s| s.storage_class_name.clone()).unwrap_or_default();

        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, namespace, status, volume, capacity, access_modes, storage_class, age }
    }
}

impl From<PersistentVolumeClaim> for PersistentVolumeClaimSummary {
    fn from(p: PersistentVolumeClaim) -> Self {
        Self::from(&p)
    }
}
