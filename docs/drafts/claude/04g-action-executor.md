# Step 4.7 — ActionExecutor

> `feat(core): add ActionExecutor for delete, scale, restart, get_yaml`

## Goal

Implement the `ActionExecutor` in crystal-core that performs write operations
against the Kubernetes API. This is the only place where mutations happen —
all paths go through this type. The executor uses a cloned `kube::Client`
to avoid borrow conflicts with `&mut App`.

## Files

| File | Action |
|------|--------|
| `crates/crystal-core/src/actions.rs` | NEW — ResourceAction enum + ActionExecutor |
| `crates/crystal-core/src/lib.rs` | UPDATE — add `mod actions`, export types |

## ResourceAction Enum

```rust
// crates/crystal-core/src/actions.rs

/// Actions that can be performed on a Kubernetes resource.
/// Used by the UI to check which actions are available for a resource kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceAction {
    Delete,
    ViewYaml,
    Describe,
    ViewLogs,
    Exec,
    Scale(i32),
    RestartRollout,
}
```

## ActionExecutor

```rust
pub struct ActionExecutor {
    client: Client,
}

impl ActionExecutor {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}
```

**Borrow checker pattern:** The `Client` is obtained by cloning from
`KubeClient::inner_client()` before entering `&mut self` methods on App.
This is the same pattern used for watcher setup in Stage 2.

```rust
// In App::handle_command():
let client = self.kube_client.as_ref().unwrap().inner_client();
let executor = ActionExecutor::new(client);
// Now safe to call &mut self methods on App while executor runs
```

## Delete

```rust
impl ActionExecutor {
    /// Delete a namespaced resource by name.
    /// Returns Ok(()) on success, Err on API failure.
    pub async fn delete<K>(&self, name: &str, ns: &str) -> anyhow::Result<()>
    where
        K: Resource<DynamicType = ()> + Clone + DeserializeOwned + Debug,
    {
        let api: Api<K> = Api::namespaced(self.client.clone(), ns);
        let dp = DeleteParams::default();
        api.delete(name, &dp).await?;
        Ok(())
    }

    /// Delete a cluster-scoped resource by name.
    pub async fn delete_cluster<K>(&self, name: &str) -> anyhow::Result<()>
    where
        K: Resource<DynamicType = ()> + Clone + DeserializeOwned + Debug,
    {
        let api: Api<K> = Api::all(self.client.clone());
        let dp = DeleteParams::default();
        api.delete(name, &dp).await?;
        Ok(())
    }
}
```

## Scale

```rust
impl ActionExecutor {
    /// Scale a Deployment or StatefulSet to the given replica count.
    /// Uses JSON Merge Patch on spec.replicas.
    pub async fn scale(&self, kind: &ResourceKind, name: &str, ns: &str, replicas: i32)
        -> anyhow::Result<()>
    {
        let patch = serde_json::json!({
            "spec": { "replicas": replicas }
        });
        let pp = PatchParams::apply("crystal");

        match kind {
            ResourceKind::Deployments => {
                let api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
                api.patch(name, &pp, &Patch::Merge(&patch)).await?;
            }
            ResourceKind::StatefulSets => {
                let api: Api<StatefulSet> = Api::namespaced(self.client.clone(), ns);
                api.patch(name, &pp, &Patch::Merge(&patch)).await?;
            }
            _ => anyhow::bail!("Scale not supported for {:?}", kind),
        }
        Ok(())
    }
}
```

## Restart Rollout

```rust
impl ActionExecutor {
    /// Trigger a rolling restart on a Deployment by patching the
    /// `kubectl.kubernetes.io/restartedAt` annotation.
    /// This is the same mechanism as `kubectl rollout restart`.
    pub async fn restart_rollout(&self, name: &str, ns: &str) -> anyhow::Result<()> {
        let now = jiff::Timestamp::now().to_string();
        let patch = serde_json::json!({
            "spec": {
                "template": {
                    "metadata": {
                        "annotations": {
                            "kubectl.kubernetes.io/restartedAt": now
                        }
                    }
                }
            }
        });
        let pp = PatchParams::apply("crystal");
        let api: Api<Deployment> = Api::namespaced(self.client.clone(), ns);
        api.patch(name, &pp, &Patch::Merge(&patch)).await?;
        Ok(())
    }
}
```

## Get YAML

```rust
impl ActionExecutor {
    /// Fetch a resource and serialize it to YAML.
    pub async fn get_yaml<K>(&self, name: &str, ns: &str) -> anyhow::Result<String>
    where
        K: Resource<DynamicType = ()> + Clone + DeserializeOwned + Serialize + Debug,
    {
        let api: Api<K> = Api::namespaced(self.client.clone(), ns);
        let obj = api.get(name).await?;
        let yaml = serde_yaml::to_string(&obj)?;
        Ok(yaml)
    }

    /// Fetch a cluster-scoped resource and serialize to YAML.
    pub async fn get_yaml_cluster<K>(&self, name: &str) -> anyhow::Result<String>
    where
        K: Resource<DynamicType = ()> + Clone + DeserializeOwned + Serialize + Debug,
    {
        let api: Api<K> = Api::all(self.client.clone());
        let obj = api.get(name).await?;
        let yaml = serde_yaml::to_string(&obj)?;
        Ok(yaml)
    }
}
```

## Describe

```rust
impl ActionExecutor {
    /// Build a describe-style output for a resource.
    /// Fetches the resource object + related events and formats them.
    pub async fn describe<K>(&self, name: &str, ns: &str) -> anyhow::Result<String>
    where
        K: Resource<DynamicType = ()> + Clone + DeserializeOwned + Debug,
    {
        let api: Api<K> = Api::namespaced(self.client.clone(), ns);
        let obj = api.get(name).await?;

        // Fetch events related to this resource
        let events_api: Api<Event> = Api::namespaced(self.client.clone(), ns);
        let lp = ListParams::default()
            .fields(&format!("involvedObject.name={}", name));
        let events = events_api.list(&lp).await?;

        // Format as describe-style output
        let mut output = String::new();
        // ... format object fields ...
        // ... format events sorted by last timestamp ...
        Ok(output)
    }
}
```

## Available Actions Per ResourceKind

Helper function used by the UI to show/hide keybinding hints:

```rust
impl ResourceAction {
    /// Which actions are available for a given resource kind.
    pub fn available_for(kind: &ResourceKind) -> Vec<ResourceAction> {
        let mut actions = vec![
            ResourceAction::Delete,
            ResourceAction::ViewYaml,
            ResourceAction::Describe,
        ];
        match kind {
            ResourceKind::Pods => {
                actions.push(ResourceAction::ViewLogs);
                actions.push(ResourceAction::Exec);
            }
            ResourceKind::Deployments => {
                actions.push(ResourceAction::Scale(0));
                actions.push(ResourceAction::RestartRollout);
            }
            ResourceKind::StatefulSets => {
                actions.push(ResourceAction::Scale(0));
            }
            _ => {}
        }
        actions
    }
}
```

## Dependencies

- `serde_yaml` crate for YAML serialization (add to crystal-core Cargo.toml)
- `k8s_openapi::api::core::v1::Event` for describe events

## Tests

- `ActionExecutor::delete::<Pod>()` compiles and type-checks
- `ActionExecutor::scale()` rejects unsupported resource kinds
- `ActionExecutor::restart_rollout()` produces correct patch JSON
- `ActionExecutor::get_yaml::<Pod>()` returns valid YAML string
- `ResourceAction::available_for(Pods)` includes ViewLogs and Exec
- `ResourceAction::available_for(Deployments)` includes Scale and RestartRollout
- `ResourceAction::available_for(ConfigMaps)` does not include Scale/Logs/Exec

## Demo

- [ ] Delete a pod → pod disappears from list after watcher update
- [ ] Scale deployment from 3 → 5 → replica count changes
- [ ] Restart deployment → pods start rolling
- [ ] Get YAML → valid YAML content returned
- [ ] Describe → formatted output with events at bottom
