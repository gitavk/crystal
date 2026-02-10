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
