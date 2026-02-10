use super::*;

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
