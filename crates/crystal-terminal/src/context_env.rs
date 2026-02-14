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
    /// Inherits the current process environment, then overlays cluster-specific variables.
    pub fn to_env_map(&self) -> HashMap<String, String> {
        let mut env: HashMap<String, String> = std::env::vars().collect();
        env.insert("KUBECONFIG".into(), self.kubeconfig.to_string_lossy().into_owned());
        env.insert("CRYSTAL_CONTEXT".into(), self.context.clone());
        env.insert("CRYSTAL_NAMESPACE".into(), self.namespace.clone());
        env.insert("CRYSTAL_CLUSTER".into(), self.cluster_name.clone());
        env
    }

    /// Generate a shell init script that configures kubectl context.
    pub fn shell_init_script(&self) -> String {
        format!(
            "export KUBECONFIG='{kubeconfig}'\n\
             kubectl config use-context '{context}'\n\
             kubectl config set-context --current --namespace='{namespace}'\n\
             export PS1='[crystal:{cluster}/{namespace}] $ '\n",
            kubeconfig = self.kubeconfig.display(),
            context = self.context,
            namespace = self.namespace,
            cluster = self.cluster_name,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_env() -> ContextEnv {
        ContextEnv {
            kubeconfig: PathBuf::from("/home/user/.kube/config"),
            context: "prod-east".into(),
            namespace: "default".into(),
            cluster_name: "prod-east-cluster".into(),
        }
    }

    #[test]
    fn env_map_contains_kubeconfig() {
        let env = sample_env().to_env_map();
        assert_eq!(env.get("KUBECONFIG").unwrap(), "/home/user/.kube/config");
    }

    #[test]
    fn env_map_contains_crystal_vars() {
        let env = sample_env().to_env_map();
        assert_eq!(env.get("CRYSTAL_CONTEXT").unwrap(), "prod-east");
        assert_eq!(env.get("CRYSTAL_NAMESPACE").unwrap(), "default");
        assert_eq!(env.get("CRYSTAL_CLUSTER").unwrap(), "prod-east-cluster");
    }

    #[test]
    fn env_map_inherits_process_env() {
        let env = sample_env().to_env_map();
        assert!(env.contains_key("HOME") || env.contains_key("PATH"));
    }

    #[test]
    fn init_script_sets_context() {
        let script = sample_env().shell_init_script();
        assert!(script.contains("kubectl config use-context 'prod-east'"));
    }

    #[test]
    fn init_script_sets_namespace() {
        let script = sample_env().shell_init_script();
        assert!(script.contains("kubectl config set-context --current --namespace='default'"));
    }

    #[test]
    fn init_script_sets_ps1() {
        let script = sample_env().shell_init_script();
        assert!(script.contains("[crystal:prod-east-cluster/default]"));
    }

    #[test]
    fn paths_with_spaces_are_quoted() {
        let ctx = ContextEnv {
            kubeconfig: PathBuf::from("/home/my user/ku be/config"),
            context: "my context".into(),
            namespace: "my ns".into(),
            cluster_name: "my cluster".into(),
        };
        let script = ctx.shell_init_script();
        assert!(script.contains("'/home/my user/ku be/config'"));
        assert!(script.contains("'my context'"));
    }
}
