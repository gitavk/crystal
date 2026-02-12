# Step 4.2 — Resource Summary Structs

> `feat(core): implement resource summary structs for all 14 types`

## Goal

Create a summary struct for each Kubernetes resource type. Each struct
implements `ResourceSummary` and is constructed via `From<K>` where `K` is
the corresponding k8s-openapi type. The existing `PodSummary` is refactored
into the new `resources/` module as the pattern to follow.

## Files

| File | Action |
|------|--------|
| `crates/crystal-core/src/resources/mod.rs` | NEW — module declarations, re-exports |
| `crates/crystal-core/src/resources/pod.rs` | NEW — move PodSummary from resource.rs |
| `crates/crystal-core/src/resources/deployment.rs` | NEW |
| `crates/crystal-core/src/resources/service.rs` | NEW |
| `crates/crystal-core/src/resources/statefulset.rs` | NEW |
| `crates/crystal-core/src/resources/daemonset.rs` | NEW |
| `crates/crystal-core/src/resources/job.rs` | NEW |
| `crates/crystal-core/src/resources/cronjob.rs` | NEW |
| `crates/crystal-core/src/resources/configmap.rs` | NEW |
| `crates/crystal-core/src/resources/secret.rs` | NEW |
| `crates/crystal-core/src/resources/ingress.rs` | NEW |
| `crates/crystal-core/src/resources/node.rs` | NEW |
| `crates/crystal-core/src/resources/namespace.rs` | NEW |
| `crates/crystal-core/src/resources/pv.rs` | NEW |
| `crates/crystal-core/src/resources/pvc.rs` | NEW |
| `crates/crystal-core/src/resource.rs` | UPDATE — remove PodSummary (moved), keep trait + DetailSection |
| `crates/crystal-core/src/lib.rs` | UPDATE — add `mod resources`, update re-exports |

## Module Registry

```rust
// crates/crystal-core/src/resources/mod.rs

mod pod;
mod deployment;
mod service;
mod statefulset;
mod daemonset;
mod job;
mod cronjob;
mod configmap;
mod secret;
mod ingress;
mod node;
mod namespace;
mod pv;
mod pvc;

pub use pod::PodSummary;
pub use deployment::DeploymentSummary;
pub use service::ServiceSummary;
pub use statefulset::StatefulSetSummary;
pub use daemonset::DaemonSetSummary;
pub use job::JobSummary;
pub use cronjob::CronJobSummary;
pub use configmap::ConfigMapSummary;
pub use secret::SecretSummary;
pub use ingress::IngressSummary;
pub use node::NodeSummary;
pub use namespace::NamespaceSummary;
pub use pv::PersistentVolumeSummary;
pub use pvc::PersistentVolumeClaimSummary;
```

## Pattern: PodSummary (refactored)

```rust
// crates/crystal-core/src/resources/pod.rs

use k8s_openapi::api::core::v1::Pod;
use crate::resource::{ResourceSummary, DetailSection, format_duration};

#[derive(Clone, Debug)]
pub struct PodSummary {
    pub name: String,
    pub namespace: String,
    pub status: PodPhase,
    pub ready: String,       // "1/2"
    pub restarts: i32,
    pub age: String,
    pub node: String,
}

impl From<Pod> for PodSummary {
    fn from(pod: Pod) -> Self {
        let meta = pod.metadata;
        let status = pod.status.unwrap_or_default();
        let spec = pod.spec.unwrap_or_default();
        // Extract fields from k8s-openapi types
        // Use jiff::Timestamp for age calculation (NOT chrono)
        Self { /* ... */ }
    }
}

impl ResourceSummary for PodSummary {
    fn name(&self) -> &str { &self.name }
    fn namespace(&self) -> Option<&str> { Some(&self.namespace) }
    fn status_display(&self) -> &str { self.status.as_str() }
    fn age(&self) -> String { self.age.clone() }

    fn columns() -> Vec<&'static str> {
        vec!["NAME", "READY", "STATUS", "RESTARTS", "AGE", "NODE"]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.name.clone(),
            self.ready.clone(),
            self.status_display().to_string(),
            self.restarts.to_string(),
            self.age.clone(),
            self.node.clone(),
        ]
    }

    fn detail_sections(&self) -> Vec<DetailSection> {
        vec![
            DetailSection {
                title: "Metadata".into(),
                fields: vec![
                    ("Name".into(), self.name.clone()),
                    ("Namespace".into(), self.namespace.clone()),
                    ("Node".into(), self.node.clone()),
                ],
            },
            DetailSection {
                title: "Status".into(),
                fields: vec![
                    ("Phase".into(), self.status_display().to_string()),
                    ("Ready".into(), self.ready.clone()),
                    ("Restarts".into(), self.restarts.to_string()),
                ],
            },
            // Additional sections: Containers, Volumes, Conditions
        ]
    }
}
```

## All Summary Structs

Each follows the same pattern: struct fields, `From<K>`, `ResourceSummary` impl.

| Struct | K8s Type | Key Fields |
|--------|----------|------------|
| `PodSummary` | `Pod` | name, namespace, status, ready, restarts, age, node |
| `DeploymentSummary` | `Deployment` | name, namespace, ready ("3/3"), up_to_date, available, age |
| `ServiceSummary` | `Service` | name, namespace, type_, cluster_ip, external_ip, ports, age |
| `StatefulSetSummary` | `StatefulSet` | name, namespace, ready, age |
| `DaemonSetSummary` | `DaemonSet` | name, namespace, desired, current, ready, age |
| `JobSummary` | `Job` | name, namespace, completions ("1/1"), duration, age |
| `CronJobSummary` | `CronJob` | name, namespace, schedule, suspend, active, last_schedule |
| `ConfigMapSummary` | `ConfigMap` | name, namespace, data_count, age |
| `SecretSummary` | `Secret` | name, namespace, type_, data_count, age |
| `IngressSummary` | `Ingress` | name, namespace, class, hosts, address, ports, age |
| `NodeSummary` | `Node` | name, status, roles, age, version |
| `NamespaceSummary` | `Namespace` | name, status, age |
| `PersistentVolumeSummary` | `PersistentVolume` | name, capacity, access_modes, status, claim |
| `PersistentVolumeClaimSummary` | `PersistentVolumeClaim` | name, status, volume, capacity, access_modes |

## k8s-openapi Import Paths

```rust
use k8s_openapi::api::core::v1::{Pod, Service, ConfigMap, Secret, Namespace,
                                  PersistentVolume, PersistentVolumeClaim, Node};
use k8s_openapi::api::apps::v1::{Deployment, StatefulSet, DaemonSet};
use k8s_openapi::api::batch::v1::{Job, CronJob};
use k8s_openapi::api::networking::v1::Ingress;
```

## Time Handling

All age calculations use `jiff::Timestamp`, not `chrono::DateTime`:

```rust
use jiff::Timestamp;

fn calculate_age(creation: Option<&k8s_openapi::apimachinery::pkg::apis::meta::v1::Time>) -> String {
    let Some(time) = creation else { return "Unknown".into() };
    // k8s_openapi::Time wraps jiff::Timestamp
    let duration = Timestamp::now().since(time.0);
    format_duration(duration)
}
```

## Detail Sections Per Resource

| Resource | Sections |
|----------|----------|
| Pod | Metadata, Status, Containers (name/image/ready/restarts per container), Volumes, Conditions |
| Deployment | Metadata, Status (ready/up-to-date/available), Strategy, Conditions |
| Service | Metadata, Spec (type/cluster-ip/ports), Endpoints |
| StatefulSet | Metadata, Status (ready/current), Update Strategy |
| DaemonSet | Metadata, Status (desired/current/ready/misscheduled) |
| Job | Metadata, Status (active/succeeded/failed), Spec (completions/parallelism) |
| CronJob | Metadata, Spec (schedule/suspend/concurrency), Status (active/last-schedule) |
| ConfigMap | Metadata, Data (key names only — no values for security) |
| Secret | Metadata, Type, Data (key names only — never show values) |
| Ingress | Metadata, Rules (host/path/backend per rule), TLS |
| Node | Metadata, Status (conditions), Capacity (cpu/memory), Info (os/arch/runtime) |
| Namespace | Metadata, Status |
| PV | Metadata, Spec (capacity/access-modes/reclaim-policy/storage-class), Status |
| PVC | Metadata, Spec (access-modes/storage-class/requested), Status (bound volume) |

## Tests

- Each summary struct's `columns().len()` equals `row().len()`
- Each `From<K>` implementation handles missing optional fields gracefully (no panics)
- `PodSummary::from(pod_json)` produces expected row values from sample JSON
- `DeploymentSummary::from(deploy_json)` produces expected row values
- `detail_sections()` returns non-empty sections for each type
- Age calculation handles None creation timestamp → "Unknown"
- Secret detail_sections never includes actual secret values (only key names)

## Demo

- [ ] `PodSummary::row()` produces `["nginx", "1/1", "Running", "0", "5m", "node-1"]`
- [ ] `DeploymentSummary::row()` produces `["my-app", "3/3", "3", "3", "1d"]`
- [ ] All 14 types compile and pass row/column length assertion
