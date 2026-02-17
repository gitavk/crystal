use super::*;

use std::env;
use std::fs;

use tempfile::tempdir;

const SAMPLE_KUBECONFIG: &str = "\
apiVersion: v1
kind: Config
current-context: sample-context
clusters:
- name: sample-context
  cluster:
    server: https://example.local
contexts:
- name: sample-context
  context:
    cluster: sample-context
    user: sample-user
users:
- name: sample-user
  user:
    token: token
";

#[tokio::test]
#[ignore] // requires a running cluster
async fn connect_and_list_namespaces() {
    let client = KubeClient::from_kubeconfig().await;
    match &client {
        Ok(c) => {
            println!("Connected to context: {}", c.context());
            println!("Default namespace: {}", c.namespace());
            let ns = c.list_namespaces().await.unwrap();
            println!("Namespaces: {ns:?}");
            assert!(ns.contains(&"default".to_string()));
        }
        Err(e) => {
            println!("Connection failed: {e:?}");
            panic!("from_kubeconfig() failed: {e}");
        }
    }
}

#[test]
fn kubeconfig_from_env_skips_missing_and_blank_paths() {
    let dir = tempdir().expect("tempdir");
    let valid_path = dir.path().join("valid-config");
    fs::write(&valid_path, SAMPLE_KUBECONFIG).expect("write kubeconfig");
    let missing_path = dir.path().join("missing-config");

    let previous = env::var_os("KUBECONFIG");
    let sep = if cfg!(windows) { ';' } else { ':' };
    let paths = format!(
        "{}{}{}{}{}",
        missing_path.display(),
        sep,
        "",
        sep,
        valid_path.display()
    );
    env::set_var("KUBECONFIG", &paths);

    let loaded = KubeClient::load_kubeconfig_from_env().expect("load kubeconfig");
    assert!(loaded.is_some());
    let config = loaded.unwrap();
    assert_eq!(config.current_context.as_deref(), Some("sample-context"));
    assert!(config.contexts.iter().any(|ctx| ctx.name == "sample-context"));

    if let Some(previous) = previous {
        env::set_var("KUBECONFIG", previous);
    } else {
        env::remove_var("KUBECONFIG");
    }
}

#[test]
fn kubeconfig_from_env_returns_none_when_all_paths_are_missing() {
    let previous = env::var_os("KUBECONFIG");
    let sep = if cfg!(windows) { ';' } else { ':' };
    let missing_path = "/nope/does/not/exist";
    let paths = format!("{missing_path}{sep}");
    env::set_var("KUBECONFIG", &paths);

    let loaded = KubeClient::load_kubeconfig_from_env().expect("load kubeconfig");
    assert!(loaded.is_none());

    if let Some(previous) = previous {
        env::set_var("KUBECONFIG", previous);
    } else {
        env::remove_var("KUBECONFIG");
    }
}
