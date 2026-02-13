# Step 5.3 — Context-Aware Environment

> `feat(terminal): build context-aware environment for cluster shells`

## Goal

Build `ContextEnv` — the struct that captures the current cluster context and
generates environment variables and shell init scripts. This is what makes the
terminal "cluster-aware": every spawned shell automatically has KUBECONFIG set,
the right context selected, and a prompt showing cluster/namespace.

## Files

| File | Action |
|------|--------|
| `crates/crystal-terminal/src/context_env.rs` | NEW — ContextEnv builder |

## Data Structures

```rust
// crates/crystal-terminal/src/context_env.rs

use std::collections::HashMap;
use std::path::PathBuf;

pub struct ContextEnv {
    pub kubeconfig: PathBuf,
    pub context: String,
    pub namespace: String,
    pub cluster_name: String,
}

impl ContextEnv {
    /// Generate env vars map for PTY session.
    /// Inherits the current process environment, then overlays
    /// cluster-specific variables.
    pub fn to_env_map(&self) -> HashMap<String, String> {
        let mut env = std::env::vars().collect::<HashMap<_, _>>();
        env.insert("KUBECONFIG".into(), self.kubeconfig.to_string_lossy().into());
        env.insert("CRYSTAL_CONTEXT".into(), self.context.clone());
        env.insert("CRYSTAL_NAMESPACE".into(), self.namespace.clone());
        env.insert("CRYSTAL_CLUSTER".into(), self.cluster_name.clone());
        env
    }

    /// Generate a shell init script that configures kubectl context.
    /// This script runs inside the spawned shell on startup.
    pub fn shell_init_script(&self) -> String {
        format!(
            "export KUBECONFIG={kubeconfig}\n\
             kubectl config use-context {context}\n\
             kubectl config set-context --current --namespace={namespace}\n\
             export PS1='[crystal:{cluster}/{namespace}] $ '\n",
            kubeconfig = self.kubeconfig.display(),
            context = self.context,
            namespace = self.namespace,
            cluster = self.cluster_name,
        )
    }
}
```

## Environment Variables

| Variable | Source | Purpose |
|----------|--------|---------|
| `KUBECONFIG` | App's active kubeconfig path | kubectl/helm auto-discovery |
| `CRYSTAL_CONTEXT` | App's selected context name | Shell scripts, prompt |
| `CRYSTAL_NAMESPACE` | App's selected namespace | Shell scripts, prompt |
| `CRYSTAL_CLUSTER` | App's resolved cluster name | Shell scripts, prompt |

## Shell Init Strategy

The init script approach:
1. PTY spawns a shell (e.g., `/bin/bash --rcfile <(script)`)
2. The init script sets KUBECONFIG, switches context, sets namespace
3. Custom PS1 prompt shows `[crystal:cluster/namespace] $`
4. User's own `.bashrc`/`.zshrc` runs after (or before, depending on shell)

Alternative: pass env vars only (no init script) and let the user's shell
config handle the rest. The env vars are always set either way.

## Tests

- `to_env_map()` contains `KUBECONFIG` with the correct path
- `to_env_map()` contains `CRYSTAL_CONTEXT`, `CRYSTAL_NAMESPACE`, `CRYSTAL_CLUSTER`
- `to_env_map()` inherits existing process env vars (e.g., `HOME`)
- `shell_init_script()` contains `kubectl config use-context <context>`
- `shell_init_script()` contains `kubectl config set-context --current --namespace=<ns>`
- `shell_init_script()` sets PS1 with cluster and namespace
- Paths with spaces are handled correctly in the init script

## Demo

- [ ] Build a ContextEnv, call `to_env_map()` → verify all 4 CRYSTAL_ vars present
- [ ] Call `shell_init_script()` → verify it parses as valid shell
- [ ] Spawn a PTY with the env map → `echo $CRYSTAL_CLUSTER` prints the cluster name
