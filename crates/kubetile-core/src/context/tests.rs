use super::*;

#[test]
fn new_resolver_has_no_context() {
    let resolver = ContextResolver::new();
    assert!(resolver.resolve().is_none());
    assert!(resolver.context_name().is_none());
    assert!(resolver.namespace().is_none());
}

#[test]
fn set_and_resolve_context() {
    let mut resolver = ContextResolver::new();
    resolver.set_context(ClusterContext { name: "minikube".into(), namespace: "default".into() });
    let ctx = resolver.resolve().unwrap();
    assert_eq!(ctx.name, "minikube");
    assert_eq!(ctx.namespace, "default");
}

#[test]
fn set_namespace_updates_only_namespace() {
    let mut resolver = ContextResolver::new();
    resolver.set_context(ClusterContext { name: "minikube".into(), namespace: "default".into() });
    resolver.set_namespace("kube-system");
    assert_eq!(resolver.context_name(), Some("minikube"));
    assert_eq!(resolver.namespace(), Some("kube-system"));
}

#[test]
fn set_namespace_noop_without_context() {
    let mut resolver = ContextResolver::new();
    resolver.set_namespace("kube-system");
    assert!(resolver.resolve().is_none());
}

#[test]
fn env_vars_empty_without_context() {
    let resolver = ContextResolver::new();
    assert!(resolver.env_vars().is_empty());
}

#[test]
fn env_vars_produces_correct_pairs() {
    let mut resolver = ContextResolver::new();
    resolver.set_context(ClusterContext { name: "prod".into(), namespace: "monitoring".into() });
    let vars = resolver.env_vars();
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0], ("K8S_CONTEXT".into(), "prod".into()));
    assert_eq!(vars[1], ("K8S_NAMESPACE".into(), "monitoring".into()));
}
