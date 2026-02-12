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

#[test]
fn pod_summary_row_returns_six_cells() {
    let summary = PodSummary {
        name: "nginx".into(),
        namespace: "default".into(),
        status: PodPhase::Running,
        ready: "1/1".into(),
        restarts: 3,
        age: Duration::from_secs(7200),
        node: Some("node-1".into()),
    };
    let row = summary.row();
    assert_eq!(row.len(), 6);
    assert_eq!(row[0], "nginx");
    assert_eq!(row[1], "1/1");
    assert_eq!(row[2], "Running");
    assert_eq!(row[3], "3");
    assert_eq!(row[4], "2h");
    assert_eq!(row[5], "node-1");
}

#[test]
fn pod_summary_detail_sections_has_metadata_and_status() {
    let summary = PodSummary {
        name: "web".into(),
        namespace: "prod".into(),
        status: PodPhase::Pending,
        ready: "0/2".into(),
        restarts: 0,
        age: Duration::from_secs(60),
        node: None,
    };
    let sections = summary.detail_sections();
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].title, "Metadata");
    assert_eq!(sections[1].title, "Status");
    assert_eq!(sections[0].fields.len(), 4); // no node
    assert_eq!(sections[1].fields[0], ("Ready".into(), "0/2".into()));
}

#[test]
fn pod_summary_detail_sections_includes_node_when_present() {
    let summary = PodSummary {
        name: "api".into(),
        namespace: "default".into(),
        status: PodPhase::Running,
        ready: "1/1".into(),
        restarts: 0,
        age: Duration::from_secs(300),
        node: Some("worker-2".into()),
    };
    let sections = summary.detail_sections();
    assert_eq!(sections[0].fields.len(), 5);
    assert_eq!(sections[0].fields[4], ("Node".into(), "worker-2".into()));
}
