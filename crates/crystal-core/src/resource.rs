use std::fmt;
use std::time::Duration;

use jiff::Timestamp;
use k8s_openapi::api::core::v1::Pod;

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
    pub status: PodPhase,
    pub ready: String,
    pub restarts: i32,
    pub age: Duration,
    pub node: Option<String>,
}

pub trait ResourceSummary: Send + Sync {
    fn name(&self) -> &str;
    fn namespace(&self) -> Option<&str>;
    fn status_display(&self) -> String;
    fn age(&self) -> Duration;
    fn columns(&self) -> Vec<(&str, String)>;
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
        ]
    }
}

impl From<&Pod> for PodSummary {
    fn from(pod: &Pod) -> Self {
        let metadata = &pod.metadata;
        let name = metadata.name.clone().unwrap_or_default();
        let namespace = metadata.namespace.clone().unwrap_or_else(|| "default".into());

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

        let age = metadata
            .creation_timestamp
            .as_ref()
            .and_then(|ts| {
                let created = ts.0;
                let now = Timestamp::now();
                let diff = now.since(created).ok()?;
                Some(Duration::from_secs(diff.get_seconds().unsigned_abs()))
            })
            .unwrap_or_default();

        let node = pod.spec.as_ref().and_then(|s| s.node_name.clone());

        Self { name, namespace, status, ready, restarts, age, node }
    }
}

pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pod_phase_display() {
        assert_eq!(PodPhase::Running.to_string(), "Running");
        assert_eq!(PodPhase::Failed.to_string(), "Failed");
        assert_eq!(PodPhase::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn pod_summary_columns_returns_seven_entries() {
        let summary = PodSummary {
            name: "nginx".into(),
            namespace: "default".into(),
            status: PodPhase::Running,
            ready: "1/1".into(),
            restarts: 0,
            age: Duration::from_secs(3600),
            node: Some("node-1".into()),
        };
        let cols = summary.columns();
        assert_eq!(cols.len(), 7);
        assert_eq!(cols[0], ("NAME", "nginx".into()));
        assert_eq!(cols[2], ("STATUS", "Running".into()));
        assert_eq!(cols[5], ("AGE", "1h".into()));
    }

    #[test]
    fn resource_summary_trait_is_object_safe() {
        let summary = PodSummary {
            name: "test".into(),
            namespace: "default".into(),
            status: PodPhase::Pending,
            ready: "0/1".into(),
            restarts: 2,
            age: Duration::from_secs(120),
            node: None,
        };
        let boxed: Box<dyn ResourceSummary> = Box::new(summary);
        assert_eq!(boxed.name(), "test");
        assert_eq!(boxed.status_display(), "Pending");
    }

    #[test]
    fn format_duration_ranges() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m");
        assert_eq!(format_duration(Duration::from_secs(7200)), "2h");
        assert_eq!(format_duration(Duration::from_secs(172800)), "2d");
    }
}
