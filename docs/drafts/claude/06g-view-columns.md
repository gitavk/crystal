# Step 6.7 — Per-Resource View Column Configuration

> `feat(config): add per-resource view column configuration`

## Goal

Allow users to control which columns appear in each resource type's table view.
Defaults cover the most useful columns; users can add, remove, or reorder them.

## Files

| File | Action |
|------|--------|
| `crates/crystal-config/src/views.rs` | NEW — ViewsConfig, ResourceViewConfig |
| `crates/crystal-config/src/defaults.toml` | EXPAND — add [views.*] sections |
| `crates/crystal-app/src/app.rs` | UPDATE — pass column config to resource panes |

## Config Types

```rust
// crates/crystal-config/src/views.rs

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct ViewsConfig {
    pub pods: ResourceViewConfig,
    pub deployments: ResourceViewConfig,
    pub services: ResourceViewConfig,
    pub statefulsets: ResourceViewConfig,
    pub daemonsets: ResourceViewConfig,
    pub jobs: ResourceViewConfig,
    pub cronjobs: ResourceViewConfig,
    pub configmaps: ResourceViewConfig,
    pub secrets: ResourceViewConfig,
    pub ingresses: ResourceViewConfig,
    pub nodes: ResourceViewConfig,
    pub namespaces: ResourceViewConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResourceViewConfig {
    pub columns: Vec<String>,
}
```

## defaults.toml additions

```toml
[views.pods]
columns = ["name", "ready", "status", "restarts", "age", "node"]

[views.deployments]
columns = ["name", "ready", "up-to-date", "available", "age"]

[views.services]
columns = ["name", "type", "cluster-ip", "external-ip", "ports", "age"]

[views.statefulsets]
columns = ["name", "ready", "age"]

[views.daemonsets]
columns = ["name", "desired", "current", "ready", "age"]

[views.jobs]
columns = ["name", "completions", "duration", "age"]

[views.cronjobs]
columns = ["name", "schedule", "suspend", "active", "last-schedule", "age"]

[views.configmaps]
columns = ["name", "data", "age"]

[views.secrets]
columns = ["name", "type", "data", "age"]

[views.ingresses]
columns = ["name", "class", "hosts", "address", "ports", "age"]

[views.nodes]
columns = ["name", "status", "roles", "age", "version"]

[views.namespaces]
columns = ["name", "status", "age"]
```

## Integration

The resource pane currently builds its own column headers from the K8s API
response. With view config, it should:

1. Check `config.views.<resource_type>.columns` for the active resource
2. Filter and reorder the API response columns to match
3. Unknown column names are silently ignored (forward-compatible with
   future columns)

## Notes

- Column names are lowercase kebab-case strings matching what the K8s watcher
  returns as headers.
- Users can add custom columns if the watcher provides them (e.g. labels,
  annotations in future stages).
- An empty `columns = []` means "show all columns in API order" (escape hatch).

## Tests

- View column config round-trips through serde
- ResourceViewConfig with unknown column names doesn't error
- Empty columns list returns all available columns
- Column order in config matches display order
