use std::time::Duration;

use k8s_openapi::api::batch::v1::CronJob;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct CronJobSummary {
    pub name: String,
    pub namespace: String,
    pub schedule: String,
    pub suspend: bool,
    pub active: i32,
    pub last_schedule: String,
    pub age: Duration,
}

impl ResourceSummary for CronJobSummary {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }

    fn status_display(&self) -> String {
        if self.suspend {
            "Suspended".into()
        } else {
            "Active".into()
        }
    }

    fn age(&self) -> Duration {
        self.age
    }

    fn columns(&self) -> Vec<(&str, String)> {
        vec![
            ("NAME", self.name.clone()),
            ("NAMESPACE", self.namespace.clone()),
            ("SCHEDULE", self.schedule.clone()),
            ("SUSPEND", self.suspend.to_string()),
            ("ACTIVE", self.active.to_string()),
            ("LAST SCHEDULE", self.last_schedule.clone()),
            ("AGE", format_duration(self.age)),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.schedule.clone(),
            self.suspend.to_string(),
            self.active.to_string(),
            self.last_schedule.clone(),
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
                fields: vec![("Schedule".into(), self.schedule.clone()), ("Suspend".into(), self.suspend.to_string())],
            },
            DetailSection {
                title: "Status".into(),
                fields: vec![
                    ("Active".into(), self.active.to_string()),
                    ("Last Schedule".into(), self.last_schedule.clone()),
                ],
            },
        ]
    }
}

impl From<&CronJob> for CronJobSummary {
    fn from(cj: &CronJob) -> Self {
        let meta = &cj.metadata;
        let name = meta.name.clone().unwrap_or_default();
        let namespace = meta.namespace.clone().unwrap_or_else(|| "default".into());

        let spec = cj.spec.as_ref();
        let schedule = spec.map(|s| s.schedule.clone()).unwrap_or_default();
        let suspend = spec.and_then(|s| s.suspend).unwrap_or(false);

        let status = cj.status.as_ref();
        let active = status.and_then(|s| s.active.as_ref()).map(|a| a.len() as i32).unwrap_or(0);

        let last_schedule = status
            .and_then(|s| s.last_schedule_time.as_ref())
            .and_then(|ts| {
                let diff = jiff::Timestamp::now().since(ts.0).ok()?;
                Some(format_duration(Duration::from_secs(diff.get_seconds().unsigned_abs())))
            })
            .unwrap_or_else(|| "<none>".into());

        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, namespace, schedule, suspend, active, last_schedule, age }
    }
}

impl From<CronJob> for CronJobSummary {
    fn from(c: CronJob) -> Self {
        Self::from(&c)
    }
}
