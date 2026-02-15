use std::time::Duration;

use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, StatefulSet};
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::api::core::v1::{
    ConfigMap, Namespace, Node, PersistentVolume, PersistentVolumeClaim, Pod, Secret, Service,
};
use k8s_openapi::api::networking::v1::Ingress;

use crate::resource::ResourceSummary;

use super::*;

fn default_pod() -> Pod {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "nginx", "namespace": "default" },
        "spec": { "nodeName": "node-1", "containers": [{ "name": "nginx", "image": "nginx:latest" }] },
        "status": {
            "phase": "Running",
            "containerStatuses": [{
                "name": "nginx",
                "ready": true,
                "restartCount": 0,
                "image": "nginx:latest",
                "imageID": "",
                "containerID": "",
                "started": true,
                "state": { "running": {} }
            }]
        }
    }))
    .unwrap()
}

fn default_deployment() -> Deployment {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": { "name": "my-app", "namespace": "default" },
        "spec": { "replicas": 3, "selector": { "matchLabels": {} }, "template": { "metadata": {}, "spec": { "containers": [] } } },
        "status": { "replicas": 3, "readyReplicas": 3, "updatedReplicas": 3, "availableReplicas": 3 }
    }))
    .unwrap()
}

fn default_service() -> Service {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": { "name": "my-svc", "namespace": "default" },
        "spec": {
            "type": "ClusterIP",
            "clusterIP": "10.0.0.1",
            "ports": [{ "port": 80, "protocol": "TCP" }]
        }
    }))
    .unwrap()
}

fn default_statefulset() -> StatefulSet {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "StatefulSet",
        "metadata": { "name": "redis", "namespace": "default" },
        "spec": { "replicas": 3, "selector": { "matchLabels": {} }, "template": { "metadata": {}, "spec": { "containers": [] } }, "serviceName": "redis" },
        "status": { "replicas": 3, "readyReplicas": 2 }
    }))
    .unwrap()
}

fn default_daemonset() -> DaemonSet {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "DaemonSet",
        "metadata": { "name": "fluentd", "namespace": "kube-system" },
        "spec": { "selector": { "matchLabels": {} }, "template": { "metadata": {}, "spec": { "containers": [] } } },
        "status": { "desiredNumberScheduled": 5, "currentNumberScheduled": 5, "numberReady": 4 }
    }))
    .unwrap()
}

fn default_job() -> Job {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": { "name": "migration", "namespace": "default" },
        "spec": { "completions": 1, "template": { "spec": { "containers": [], "restartPolicy": "Never" } } },
        "status": { "succeeded": 1 }
    }))
    .unwrap()
}

fn default_cronjob() -> CronJob {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "batch/v1",
        "kind": "CronJob",
        "metadata": { "name": "backup", "namespace": "default" },
        "spec": {
            "schedule": "0 2 * * *",
            "jobTemplate": { "spec": { "template": { "spec": { "containers": [], "restartPolicy": "Never" } } } }
        },
        "status": { "active": [] }
    }))
    .unwrap()
}

fn default_configmap() -> ConfigMap {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "ConfigMap",
        "metadata": { "name": "app-config", "namespace": "default" },
        "data": { "key1": "val1", "key2": "val2" }
    }))
    .unwrap()
}

fn default_secret() -> Secret {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Secret",
        "metadata": { "name": "db-creds", "namespace": "default" },
        "type": "Opaque",
        "data": { "username": "dXNlcg==", "password": "cGFzcw==" }
    }))
    .unwrap()
}

fn default_ingress() -> Ingress {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "networking.k8s.io/v1",
        "kind": "Ingress",
        "metadata": { "name": "web-ing", "namespace": "default" },
        "spec": {
            "ingressClassName": "nginx",
            "rules": [{ "host": "example.com", "http": { "paths": [{ "path": "/", "pathType": "Prefix", "backend": { "service": { "name": "web", "port": { "number": 80 } } } }] } }]
        }
    }))
    .unwrap()
}

fn default_node() -> Node {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Node",
        "metadata": { "name": "worker-1", "labels": { "node-role.kubernetes.io/worker": "" } },
        "status": {
            "conditions": [{ "type": "Ready", "status": "True" }],
            "nodeInfo": {
                "kubeletVersion": "v1.28.0",
                "machineID": "", "systemUUID": "", "bootID": "",
                "kernelVersion": "", "osImage": "", "containerRuntimeVersion": "",
                "kubeProxyVersion": "", "operatingSystem": "linux", "architecture": "amd64"
            }
        }
    }))
    .unwrap()
}

fn default_namespace() -> Namespace {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Namespace",
        "metadata": { "name": "production" },
        "status": { "phase": "Active" }
    }))
    .unwrap()
}

fn default_pv() -> PersistentVolume {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "PersistentVolume",
        "metadata": { "name": "pv-data" },
        "spec": {
            "capacity": { "storage": "10Gi" },
            "accessModes": ["ReadWriteOnce"],
            "persistentVolumeReclaimPolicy": "Retain",
            "storageClassName": "standard",
            "claimRef": { "namespace": "default", "name": "data-claim" }
        },
        "status": { "phase": "Bound" }
    }))
    .unwrap()
}

fn default_pvc() -> PersistentVolumeClaim {
    serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "PersistentVolumeClaim",
        "metadata": { "name": "data-claim", "namespace": "default" },
        "spec": {
            "accessModes": ["ReadWriteOnce"],
            "storageClassName": "standard",
            "volumeName": "pv-data",
            "resources": { "requests": { "storage": "10Gi" } }
        },
        "status": {
            "phase": "Bound",
            "capacity": { "storage": "10Gi" },
            "accessModes": ["ReadWriteOnce"]
        }
    }))
    .unwrap()
}

// --- Pod ---

#[test]
fn pod_phase_display() {
    assert_eq!(PodPhase::Running.to_string(), "Running");
    assert_eq!(PodPhase::Failed.to_string(), "Failed");
    assert_eq!(PodPhase::Unknown.to_string(), "Unknown");
}

#[test]
fn pod_summary_columns_and_row_length() {
    let s = PodSummary::from(&default_pod());
    assert_eq!(s.columns().len(), 7);
    assert_eq!(s.row().len(), 7);
}

#[test]
fn pod_summary_from_k8s() {
    let s = PodSummary::from(&default_pod());
    assert_eq!(s.name, "nginx");
    assert_eq!(s.namespace, "default");
    assert_eq!(s.status, PodPhase::Running);
    assert_eq!(s.ready, "1/1");
    assert_eq!(s.restarts, 0);
    assert_eq!(s.node, Some("node-1".into()));
}

#[test]
fn pod_summary_row_values() {
    let s = PodSummary {
        name: "nginx".into(),
        namespace: "default".into(),
        status: PodPhase::Running,
        ready: "1/1".into(),
        restarts: 0,
        age: Duration::from_secs(300),
        node: Some("node-1".into()),
    };
    let row = s.row();
    assert_eq!(row, vec!["nginx", "default", "1/1", "Running", "0", "5m", "node-1"]);
}

#[test]
fn pod_summary_detail_sections() {
    let s = PodSummary::from(&default_pod());
    let sections = s.detail_sections();
    assert!(!sections.is_empty());
    assert_eq!(sections[0].title, "Metadata");
}

#[test]
fn pod_summary_missing_status() {
    let pod: Pod = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1", "kind": "Pod",
        "metadata": { "name": "bare" }
    }))
    .unwrap();
    let s = PodSummary::from(&pod);
    assert_eq!(s.name, "bare");
    assert_eq!(s.status, PodPhase::Unknown);
    assert_eq!(s.ready, "0/0");
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
fn pod_summary_row_includes_namespace_column() {
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
    assert_eq!(row.len(), 7);
    assert_eq!(row[0], "nginx");
    assert_eq!(row[1], "default");
    assert_eq!(row[2], "1/1");
    assert_eq!(row[3], "Running");
    assert_eq!(row[4], "3");
    assert_eq!(row[5], "2h");
    assert_eq!(row[6], "node-1");
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
    assert_eq!(sections[0].fields.len(), 4);
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

// --- Deployment ---

#[test]
fn deployment_summary_columns_and_row_length() {
    let s = DeploymentSummary::from(&default_deployment());
    assert_eq!(s.columns().len(), 6);
    assert_eq!(s.row().len(), 5);
}

#[test]
fn deployment_summary_from_k8s() {
    let s = DeploymentSummary::from(&default_deployment());
    assert_eq!(s.name, "my-app");
    assert_eq!(s.ready, "3/3");
    assert_eq!(s.up_to_date, 3);
    assert_eq!(s.available, 3);
}

#[test]
fn deployment_summary_row_values() {
    let s = DeploymentSummary {
        name: "my-app".into(),
        namespace: "default".into(),
        ready: "3/3".into(),
        up_to_date: 3,
        available: 3,
        age: Duration::from_secs(86400),
    };
    let row = s.row();
    assert_eq!(row, vec!["my-app", "3/3", "3", "3", "1d"]);
}

#[test]
fn deployment_summary_detail_sections() {
    let s = DeploymentSummary::from(&default_deployment());
    let sections = s.detail_sections();
    assert!(!sections.is_empty());
    assert_eq!(sections[0].title, "Metadata");
}

#[test]
fn deployment_summary_missing_status() {
    let d: Deployment = serde_json::from_value(serde_json::json!({
        "apiVersion": "apps/v1", "kind": "Deployment",
        "metadata": { "name": "bare" },
        "spec": { "replicas": 1, "selector": { "matchLabels": {} }, "template": { "metadata": {}, "spec": { "containers": [] } } }
    }))
    .unwrap();
    let s = DeploymentSummary::from(&d);
    assert_eq!(s.ready, "0/0");
    assert_eq!(s.up_to_date, 0);
}

// --- Service ---

#[test]
fn service_summary_columns_and_row_length() {
    let s = ServiceSummary::from(&default_service());
    assert_eq!(s.columns().len(), 7);
    assert_eq!(s.row().len(), 6);
}

#[test]
fn service_summary_from_k8s() {
    let s = ServiceSummary::from(&default_service());
    assert_eq!(s.name, "my-svc");
    assert_eq!(s.type_, "ClusterIP");
    assert_eq!(s.cluster_ip, "10.0.0.1");
    assert_eq!(s.ports, "80/TCP");
}

#[test]
fn service_summary_detail_sections() {
    let s = ServiceSummary::from(&default_service());
    let sections = s.detail_sections();
    assert!(!sections.is_empty());
}

// --- StatefulSet ---

#[test]
fn statefulset_summary_columns_and_row_length() {
    let s = StatefulSetSummary::from(&default_statefulset());
    assert_eq!(s.columns().len(), 4);
    assert_eq!(s.row().len(), 3);
}

#[test]
fn statefulset_summary_from_k8s() {
    let s = StatefulSetSummary::from(&default_statefulset());
    assert_eq!(s.name, "redis");
    assert_eq!(s.ready, "2/3");
}

#[test]
fn statefulset_summary_detail_sections() {
    let s = StatefulSetSummary::from(&default_statefulset());
    assert!(!s.detail_sections().is_empty());
}

// --- DaemonSet ---

#[test]
fn daemonset_summary_columns_and_row_length() {
    let s = DaemonSetSummary::from(&default_daemonset());
    assert_eq!(s.columns().len(), 6);
    assert_eq!(s.row().len(), 5);
}

#[test]
fn daemonset_summary_from_k8s() {
    let s = DaemonSetSummary::from(&default_daemonset());
    assert_eq!(s.name, "fluentd");
    assert_eq!(s.desired, 5);
    assert_eq!(s.current, 5);
    assert_eq!(s.ready, 4);
}

#[test]
fn daemonset_summary_detail_sections() {
    let s = DaemonSetSummary::from(&default_daemonset());
    assert!(!s.detail_sections().is_empty());
}

// --- Job ---

#[test]
fn job_summary_columns_and_row_length() {
    let s = JobSummary::from(&default_job());
    assert_eq!(s.columns().len(), 5);
    assert_eq!(s.row().len(), 4);
}

#[test]
fn job_summary_from_k8s() {
    let s = JobSummary::from(&default_job());
    assert_eq!(s.name, "migration");
    assert_eq!(s.completions, "1/1");
}

#[test]
fn job_summary_detail_sections() {
    let s = JobSummary::from(&default_job());
    assert!(!s.detail_sections().is_empty());
}

// --- CronJob ---

#[test]
fn cronjob_summary_columns_and_row_length() {
    let s = CronJobSummary::from(&default_cronjob());
    assert_eq!(s.columns().len(), 7);
    assert_eq!(s.row().len(), 6);
}

#[test]
fn cronjob_summary_from_k8s() {
    let s = CronJobSummary::from(&default_cronjob());
    assert_eq!(s.name, "backup");
    assert_eq!(s.schedule, "0 2 * * *");
    assert!(!s.suspend);
    assert_eq!(s.active, 0);
}

#[test]
fn cronjob_summary_detail_sections() {
    let s = CronJobSummary::from(&default_cronjob());
    let sections = s.detail_sections();
    assert_eq!(sections.len(), 3);
}

// --- ConfigMap ---

#[test]
fn configmap_summary_columns_and_row_length() {
    let s = ConfigMapSummary::from(&default_configmap());
    assert_eq!(s.columns().len(), 4);
    assert_eq!(s.row().len(), 3);
}

#[test]
fn configmap_summary_from_k8s() {
    let s = ConfigMapSummary::from(&default_configmap());
    assert_eq!(s.name, "app-config");
    assert_eq!(s.data_count, 2);
}

#[test]
fn configmap_summary_detail_sections() {
    let s = ConfigMapSummary::from(&default_configmap());
    assert!(!s.detail_sections().is_empty());
}

// --- Secret ---

#[test]
fn secret_summary_columns_and_row_length() {
    let s = SecretSummary::from(&default_secret());
    assert_eq!(s.columns().len(), 5);
    assert_eq!(s.row().len(), 4);
}

#[test]
fn secret_summary_from_k8s() {
    let s = SecretSummary::from(&default_secret());
    assert_eq!(s.name, "db-creds");
    assert_eq!(s.type_, "Opaque");
    assert_eq!(s.data_count, 2);
}

#[test]
fn secret_detail_sections_never_show_values() {
    let s = SecretSummary::from(&default_secret());
    let sections = s.detail_sections();
    for section in &sections {
        for (_key, value) in &section.fields {
            assert!(!value.contains("dXNlcg=="), "secret value leaked in detail_sections");
            assert!(!value.contains("cGFzcw=="), "secret value leaked in detail_sections");
        }
    }
}

#[test]
fn secret_summary_detail_sections() {
    let s = SecretSummary::from(&default_secret());
    assert!(!s.detail_sections().is_empty());
}

// --- Ingress ---

#[test]
fn ingress_summary_columns_and_row_length() {
    let s = IngressSummary::from(&default_ingress());
    assert_eq!(s.columns().len(), 7);
    assert_eq!(s.row().len(), 6);
}

#[test]
fn ingress_summary_from_k8s() {
    let s = IngressSummary::from(&default_ingress());
    assert_eq!(s.name, "web-ing");
    assert_eq!(s.class, "nginx");
    assert_eq!(s.hosts, "example.com");
    assert_eq!(s.ports, "80");
}

#[test]
fn ingress_summary_detail_sections() {
    let s = IngressSummary::from(&default_ingress());
    assert!(!s.detail_sections().is_empty());
}

// --- Node ---

#[test]
fn node_summary_columns_and_row_length() {
    let s = NodeSummary::from(&default_node());
    assert_eq!(s.columns().len(), 5);
    assert_eq!(s.row().len(), 5);
}

#[test]
fn node_summary_from_k8s() {
    let s = NodeSummary::from(&default_node());
    assert_eq!(s.name, "worker-1");
    assert_eq!(s.status, "Ready");
    assert_eq!(s.roles, "worker");
    assert_eq!(s.version, "v1.28.0");
}

#[test]
fn node_summary_namespace_is_none() {
    let s = NodeSummary::from(&default_node());
    assert_eq!(s.namespace(), None);
}

#[test]
fn node_summary_detail_sections() {
    let s = NodeSummary::from(&default_node());
    assert!(!s.detail_sections().is_empty());
}

// --- Namespace ---

#[test]
fn namespace_summary_columns_and_row_length() {
    let s = NamespaceSummary::from(&default_namespace());
    assert_eq!(s.columns().len(), 3);
    assert_eq!(s.row().len(), 3);
}

#[test]
fn namespace_summary_from_k8s() {
    let s = NamespaceSummary::from(&default_namespace());
    assert_eq!(s.name, "production");
    assert_eq!(s.status, "Active");
}

#[test]
fn namespace_summary_namespace_is_none() {
    let s = NamespaceSummary::from(&default_namespace());
    assert_eq!(s.namespace(), None);
}

#[test]
fn namespace_summary_detail_sections() {
    let s = NamespaceSummary::from(&default_namespace());
    assert!(!s.detail_sections().is_empty());
}

// --- PersistentVolume ---

#[test]
fn pv_summary_columns_and_row_length() {
    let s = PersistentVolumeSummary::from(&default_pv());
    assert_eq!(s.columns().len(), 8);
    assert_eq!(s.row().len(), 8);
}

#[test]
fn pv_summary_from_k8s() {
    let s = PersistentVolumeSummary::from(&default_pv());
    assert_eq!(s.name, "pv-data");
    assert_eq!(s.capacity, "10Gi");
    assert_eq!(s.access_modes, "RWO");
    assert_eq!(s.reclaim_policy, "Retain");
    assert_eq!(s.status, "Bound");
    assert_eq!(s.claim, "default/data-claim");
    assert_eq!(s.storage_class, "standard");
}

#[test]
fn pv_summary_namespace_is_none() {
    let s = PersistentVolumeSummary::from(&default_pv());
    assert_eq!(s.namespace(), None);
}

#[test]
fn pv_summary_detail_sections() {
    let s = PersistentVolumeSummary::from(&default_pv());
    assert!(!s.detail_sections().is_empty());
}

// --- PersistentVolumeClaim ---

#[test]
fn pvc_summary_columns_and_row_length() {
    let s = PersistentVolumeClaimSummary::from(&default_pvc());
    assert_eq!(s.columns().len(), 8);
    assert_eq!(s.row().len(), 7);
}

#[test]
fn pvc_summary_from_k8s() {
    let s = PersistentVolumeClaimSummary::from(&default_pvc());
    assert_eq!(s.name, "data-claim");
    assert_eq!(s.status, "Bound");
    assert_eq!(s.volume, "pv-data");
    assert_eq!(s.capacity, "10Gi");
    assert_eq!(s.access_modes, "RWO");
    assert_eq!(s.storage_class, "standard");
}

#[test]
fn pvc_summary_detail_sections() {
    let s = PersistentVolumeClaimSummary::from(&default_pvc());
    assert!(!s.detail_sections().is_empty());
}

// --- Cross-cutting: minimal/empty objects don't panic ---

#[test]
fn empty_pod_does_not_panic() {
    let pod: Pod = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1", "kind": "Pod", "metadata": {}
    }))
    .unwrap();
    let _ = PodSummary::from(&pod);
}

#[test]
fn empty_deployment_does_not_panic() {
    let d: Deployment = serde_json::from_value(serde_json::json!({
        "apiVersion": "apps/v1", "kind": "Deployment", "metadata": {},
        "spec": { "selector": { "matchLabels": {} }, "template": { "metadata": {}, "spec": { "containers": [] } } }
    }))
    .unwrap();
    let _ = DeploymentSummary::from(&d);
}

#[test]
fn empty_service_does_not_panic() {
    let s: Service = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1", "kind": "Service", "metadata": {}
    }))
    .unwrap();
    let _ = ServiceSummary::from(&s);
}

#[test]
fn empty_node_does_not_panic() {
    let n: Node = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1", "kind": "Node", "metadata": {}
    }))
    .unwrap();
    let _ = NodeSummary::from(&n);
}

#[test]
fn empty_namespace_does_not_panic() {
    let ns: Namespace = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1", "kind": "Namespace", "metadata": {}
    }))
    .unwrap();
    let _ = NamespaceSummary::from(&ns);
}

#[test]
fn empty_secret_does_not_panic() {
    let s: Secret = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1", "kind": "Secret", "metadata": {}
    }))
    .unwrap();
    let _ = SecretSummary::from(&s);
}

#[test]
fn empty_configmap_does_not_panic() {
    let cm: ConfigMap = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1", "kind": "ConfigMap", "metadata": {}
    }))
    .unwrap();
    let _ = ConfigMapSummary::from(&cm);
}

#[test]
fn empty_pv_does_not_panic() {
    let pv: PersistentVolume = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1", "kind": "PersistentVolume", "metadata": {}
    }))
    .unwrap();
    let _ = PersistentVolumeSummary::from(&pv);
}

#[test]
fn empty_pvc_does_not_panic() {
    let pvc: PersistentVolumeClaim = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1", "kind": "PersistentVolumeClaim", "metadata": {}
    }))
    .unwrap();
    let _ = PersistentVolumeClaimSummary::from(&pvc);
}
