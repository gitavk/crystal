# Step 4.1 — Resource Types (ResourceKind + ResourceSummary)

> `feat(tui,core): expand ResourceKind enum and ResourceSummary trait`

## Goal

Expand the existing `ResourceKind` enum to cover all 14 supported Kubernetes
resource types. Extend the `ResourceSummary` trait with `row()` and
`detail_sections()` so every resource type can produce table rows and detail
views through a single generic interface.

## Files

| File | Action |
|------|--------|
| `crates/crystal-tui/src/pane.rs` | UPDATE — extend ResourceKind enum with missing variants |
| `crates/crystal-core/src/resource.rs` | UPDATE — extend ResourceSummary trait, add DetailSection |

## Existing State

`ResourceKind` already exists in `crystal-tui/src/pane.rs` with these variants:

```rust
pub enum ResourceKind {
    Pods,
    Deployments,
    Services,
    ConfigMaps,
    Secrets,
    Nodes,
    Namespaces,
    Custom(String),
}
```

`ResourceSummary` already exists in `crystal-core/src/resource.rs` with:

```rust
pub trait ResourceSummary: Send + Sync + Clone {
    fn name(&self) -> &str;
    fn namespace(&self) -> Option<&str>;
    fn status_display(&self) -> &str;
    fn age(&self) -> String;
    fn columns() -> Vec<&'static str>;
}
```

## Extended ResourceKind

```rust
// crates/crystal-tui/src/pane.rs

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Pods,
    Deployments,
    Services,
    StatefulSets,              // NEW
    DaemonSets,                // NEW
    Jobs,                      // NEW
    CronJobs,                  // NEW
    ConfigMaps,
    Secrets,
    Ingresses,                 // NEW
    Nodes,
    Namespaces,
    PersistentVolumes,         // NEW
    PersistentVolumeClaims,    // NEW
    Custom(String),
}

impl ResourceKind {
    /// Short name for the resource switcher command palette.
    /// Matches kubectl short names where they exist.
    pub fn short_name(&self) -> &str {
        match self {
            Self::Pods => "po",
            Self::Deployments => "deploy",
            Self::Services => "svc",
            Self::StatefulSets => "sts",
            Self::DaemonSets => "ds",
            Self::Jobs => "job",
            Self::CronJobs => "cj",
            Self::ConfigMaps => "cm",
            Self::Secrets => "secret",
            Self::Ingresses => "ing",
            Self::Nodes => "no",
            Self::Namespaces => "ns",
            Self::PersistentVolumes => "pv",
            Self::PersistentVolumeClaims => "pvc",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// Human-readable display name for headers and breadcrumbs.
    pub fn display_name(&self) -> &str {
        match self {
            Self::Pods => "Pods",
            Self::Deployments => "Deployments",
            Self::Services => "Services",
            Self::StatefulSets => "StatefulSets",
            Self::DaemonSets => "DaemonSets",
            Self::Jobs => "Jobs",
            Self::CronJobs => "CronJobs",
            Self::ConfigMaps => "ConfigMaps",
            Self::Secrets => "Secrets",
            Self::Ingresses => "Ingresses",
            Self::Nodes => "Nodes",
            Self::Namespaces => "Namespaces",
            Self::PersistentVolumes => "PersistentVolumes",
            Self::PersistentVolumeClaims => "PersistentVolumeClaims",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// All built-in variants in menu order (for resource switcher).
    pub fn all() -> &'static [ResourceKind] {
        &[
            Self::Pods,
            Self::Deployments,
            Self::Services,
            Self::StatefulSets,
            Self::DaemonSets,
            Self::Jobs,
            Self::CronJobs,
            Self::ConfigMaps,
            Self::Secrets,
            Self::Ingresses,
            Self::Nodes,
            Self::Namespaces,
            Self::PersistentVolumes,
            Self::PersistentVolumeClaims,
        ]
    }

    /// Whether this resource kind is namespaced (vs cluster-scoped).
    pub fn is_namespaced(&self) -> bool {
        !matches!(self, Self::Nodes | Self::Namespaces | Self::PersistentVolumes)
    }
}
```

## Extended ResourceSummary Trait

```rust
// crates/crystal-core/src/resource.rs

pub trait ResourceSummary: Send + Sync + Clone {
    fn name(&self) -> &str;
    fn namespace(&self) -> Option<&str>;
    fn status_display(&self) -> &str;
    fn age(&self) -> String;
    fn columns() -> Vec<&'static str>;

    /// Produce a row of cell values matching columns() order.
    /// Used by ResourceListWidget to render table rows.
    fn row(&self) -> Vec<String>;

    /// Produce detail sections for the detail pane.
    /// Each section has a title and key-value fields.
    fn detail_sections(&self) -> Vec<DetailSection>;
}

/// A named group of key-value fields for the detail view.
pub struct DetailSection {
    pub title: String,
    pub fields: Vec<(String, String)>,
}
```

## Why This Step First

Every subsequent step depends on these two types:

- Step 4.2 implements `ResourceSummary` for each resource type
- Step 4.3 uses `ResourceSummary` as the generic bound on the watcher
- Steps 4.5 and 4.6 render data through `row()` and `detail_sections()`
- Step 4.8 uses `ResourceKind::all()` and `short_name()` for the switcher

## Tests

- `ResourceKind::all()` returns exactly 14 variants
- `ResourceKind::short_name()` is unique for each built-in variant
- `ResourceKind::is_namespaced()` returns false only for Nodes, Namespaces, PersistentVolumes
- `ResourceKind` round-trips through `short_name()` → lookup (needed for the resource switcher)
