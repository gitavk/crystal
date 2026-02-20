use std::time::Duration;

use k8s_openapi::api::core::v1::PersistentVolume;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct PersistentVolumeSummary {
    pub name: String,
    pub capacity: String,
    pub access_modes: String,
    pub reclaim_policy: String,
    pub status: String,
    pub claim: String,
    pub storage_class: String,
    pub age: Duration,
}

impl ResourceSummary for PersistentVolumeSummary {
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
            ("CAPACITY", self.capacity.clone()),
            ("ACCESS MODES", self.access_modes.clone()),
            ("RECLAIM POLICY", self.reclaim_policy.clone()),
            ("STATUS", self.status.clone()),
            ("CLAIM", self.claim.clone()),
            ("STORAGECLASS", self.storage_class.clone()),
            ("AGE", format_duration(self.age)),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.capacity.clone(),
            self.access_modes.clone(),
            self.reclaim_policy.clone(),
            self.status.clone(),
            self.claim.clone(),
            self.storage_class.clone(),
            format_duration(self.age),
        ]
    }

    fn detail_sections(&self) -> Vec<DetailSection> {
        vec![
            DetailSection {
                title: "Metadata".into(),
                fields: vec![("Name".into(), self.name.clone()), ("Age".into(), format_duration(self.age))],
            },
            DetailSection {
                title: "Spec".into(),
                fields: vec![
                    ("Capacity".into(), self.capacity.clone()),
                    ("Access Modes".into(), self.access_modes.clone()),
                    ("Reclaim Policy".into(), self.reclaim_policy.clone()),
                    ("Storage Class".into(), self.storage_class.clone()),
                ],
            },
            DetailSection {
                title: "Status".into(),
                fields: vec![("Phase".into(), self.status.clone()), ("Claim".into(), self.claim.clone())],
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

impl From<&PersistentVolume> for PersistentVolumeSummary {
    fn from(pv: &PersistentVolume) -> Self {
        let meta = &pv.metadata;
        let name = meta.name.clone().unwrap_or_default();

        let spec = pv.spec.as_ref();

        let capacity = spec
            .and_then(|s| s.capacity.as_ref())
            .and_then(|c| c.get("storage"))
            .map(|q| q.0.clone())
            .unwrap_or_else(|| "<none>".into());

        let access_modes =
            spec.and_then(|s| s.access_modes.as_ref()).map(|m| format_access_modes(m)).unwrap_or_default();

        let reclaim_policy =
            spec.and_then(|s| s.persistent_volume_reclaim_policy.as_ref()).cloned().unwrap_or_else(|| "Delete".into());

        let status = pv.status.as_ref().and_then(|s| s.phase.as_deref()).unwrap_or("Available").to_string();

        let claim = spec
            .and_then(|s| s.claim_ref.as_ref())
            .map(|cr| {
                let ns = cr.namespace.as_deref().unwrap_or("");
                let n = cr.name.as_deref().unwrap_or("");
                if ns.is_empty() {
                    n.to_string()
                } else {
                    format!("{ns}/{n}")
                }
            })
            .unwrap_or_default();

        let storage_class = spec.and_then(|s| s.storage_class_name.clone()).unwrap_or_default();

        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, capacity, access_modes, reclaim_policy, status, claim, storage_class, age }
    }
}

impl From<PersistentVolume> for PersistentVolumeSummary {
    fn from(p: PersistentVolume) -> Self {
        Self::from(&p)
    }
}
