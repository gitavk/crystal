use std::time::Duration;

use k8s_openapi::api::batch::v1::Job;

use crate::resource::{calculate_age, format_duration, DetailSection, ResourceSummary};

#[derive(Debug, Clone)]
pub struct JobSummary {
    pub name: String,
    pub namespace: String,
    pub completions: String,
    pub duration: String,
    pub age: Duration,
}

impl ResourceSummary for JobSummary {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }

    fn status_display(&self) -> String {
        self.completions.clone()
    }

    fn age(&self) -> Duration {
        self.age
    }

    fn columns(&self) -> Vec<(&str, String)> {
        vec![
            ("NAME", self.name.clone()),
            ("NAMESPACE", self.namespace.clone()),
            ("COMPLETIONS", self.completions.clone()),
            ("DURATION", self.duration.clone()),
            ("AGE", format_duration(self.age)),
        ]
    }

    fn row(&self) -> Vec<String> {
        vec![self.name.clone(), self.completions.clone(), self.duration.clone(), format_duration(self.age)]
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
                    ("Completions".into(), self.completions.clone()),
                    ("Duration".into(), self.duration.clone()),
                ],
            },
        ]
    }
}

impl From<&Job> for JobSummary {
    fn from(job: &Job) -> Self {
        let meta = &job.metadata;
        let name = meta.name.clone().unwrap_or_default();
        let namespace = meta.namespace.clone().unwrap_or_else(|| "default".into());

        let spec = job.spec.as_ref();
        let desired = spec.and_then(|s| s.completions).unwrap_or(1);

        let status = job.status.as_ref();
        let succeeded = status.and_then(|s| s.succeeded).unwrap_or(0);
        let completions = format!("{succeeded}/{desired}");

        let duration = status
            .and_then(|s| {
                let start = s.start_time.as_ref()?;
                let end = s.completion_time.as_ref();
                let end_ts = end.map(|t| t.0).unwrap_or_else(jiff::Timestamp::now);
                let diff = end_ts.since(start.0).ok()?;
                Some(format_duration(Duration::from_secs(diff.get_seconds().unsigned_abs())))
            })
            .unwrap_or_else(|| "<none>".into());

        let age = calculate_age(meta.creation_timestamp.as_ref());

        Self { name, namespace, completions, duration, age }
    }
}

impl From<Job> for JobSummary {
    fn from(j: Job) -> Self {
        Self::from(&j)
    }
}
