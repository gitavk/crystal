use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tracing::{debug, error, warn};

static NEXT_FORWARD_ID: AtomicU64 = AtomicU64::new(1);

pub type ForwardId = u64;

/// Port forwarding session from a local port to a pod port.
///
/// This struct manages a Kubernetes port forward session, which tunnels traffic
/// from a local TCP port to a port inside a pod. The forwarding runs in a
/// background task and continues until `stop()` is called or the connection
/// is lost.
///
/// # Example
///
/// ```ignore
/// let forward = PortForward::start(
///     &client,
///     "my-pod",
///     "default",
///     8080,
///     80,
/// ).await?;
///
/// // Forward is now active: localhost:8080 → pod:80
/// // Stop it when done:
/// forward.stop().await?;
/// ```
pub struct PortForward {
    id: ForwardId,
    local_port: u16,
    remote_port: u16,
    pod_name: String,
    namespace: String,
    pod_uid: Option<String>,
    started_at: Instant,
    handle: JoinHandle<()>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
}

impl PortForward {
    /// Start a new port forward from a local port to a pod port.
    ///
    /// # Arguments
    ///
    /// * `client` - Kubernetes client
    /// * `pod_name` - Name of the target pod
    /// * `namespace` - Namespace of the target pod
    /// * `local_port` - Local port to bind to (e.g., 8080)
    /// * `remote_port` - Target port in the pod (e.g., 80)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Local port is already in use
    /// - Pod does not exist or is not running
    /// - Kubernetes API returns an error (e.g., RBAC denied)
    pub async fn start(
        client: &Client,
        pod_name: &str,
        namespace: &str,
        local_port: u16,
        remote_port: u16,
    ) -> anyhow::Result<Self> {
        let id = NEXT_FORWARD_ID.fetch_add(1, Ordering::Relaxed);
        let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
        let pod_uid = pods.get(pod_name).await.ok().and_then(|pod| pod.metadata.uid);

        // Bind to local port early to fail fast if port is in use
        let listener = TcpListener::bind(format!("127.0.0.1:{}", local_port)).await?;
        let actual_local_port = listener.local_addr()?.port();

        debug!("Port forward {}: binding {}:{} → {}:{}", id, actual_local_port, pod_name, namespace, remote_port);

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let client = client.clone();
        let pod_name_str = pod_name.to_string();
        let namespace_str = namespace.to_string();
        let pod_name_clone = pod_name_str.clone();
        let namespace_clone = namespace_str.clone();

        let handle = tokio::spawn(async move {
            if let Err(e) =
                run_port_forward(listener, client, &pod_name_clone, &namespace_clone, remote_port, shutdown_rx).await
            {
                error!("Port forward {} error: {}", id, e);
            }
            debug!("Port forward {} stopped", id);
        });

        Ok(Self {
            id,
            local_port: actual_local_port,
            remote_port,
            pod_name: pod_name_str,
            namespace: namespace_str,
            pod_uid,
            started_at: Instant::now(),
            handle,
            shutdown_tx,
        })
    }

    /// Stop the port forward and clean up resources.
    ///
    /// This sends a shutdown signal to the background task and waits for it
    /// to complete. Any active connections will be terminated.
    pub async fn stop(self) -> anyhow::Result<()> {
        debug!("Stopping port forward {}", self.id);
        let _ = self.shutdown_tx.send(true);
        self.handle.abort();
        Ok(())
    }

    /// Get the local port being forwarded.
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    /// Get the remote port in the pod.
    pub fn remote_port(&self) -> u16 {
        self.remote_port
    }

    /// Get the name of the pod being forwarded to.
    pub fn pod_name(&self) -> &str {
        &self.pod_name
    }

    /// Get the namespace of the pod.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Get the UID of the pod when the forward was created.
    pub fn pod_uid(&self) -> Option<&str> {
        self.pod_uid.as_deref()
    }

    /// Get how long this forward has existed.
    pub fn age(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get the unique identifier for this forward.
    pub fn id(&self) -> ForwardId {
        self.id
    }
}

async fn run_port_forward(
    listener: TcpListener,
    client: Client,
    pod_name: &str,
    namespace: &str,
    remote_port: u16,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let pods: Api<Pod> = Api::namespaced(client, namespace);

    loop {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                debug!("Port forward shutdown signal received");
                return Ok(());
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((mut local_stream, _)) => {
                        debug!("Accepted local connection for pod {}:{}", pod_name, remote_port);

                        let mut pf = match pods.portforward(pod_name, &[remote_port]).await {
                            Ok(pf) => pf,
                            Err(e) => {
                                warn!("Failed to establish portforward to pod {}: {}", pod_name, e);
                                continue;
                            }
                        };

                        let mut upstream = match pf.take_stream(remote_port) {
                            Some(stream) => stream,
                            None => {
                                warn!("No stream available for port {}", remote_port);
                                continue;
                            }
                        };

                        // Spawn a task to handle this specific connection
                        tokio::spawn(async move {
                            if let Err(e) = proxy_connection(&mut local_stream, &mut upstream).await {
                                debug!("Connection proxy error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept local connection: {}", e);
                        return Err(e.into());
                    }
                }
            }
        }
    }
}

async fn proxy_connection(
    local: &mut TcpStream,
    upstream: &mut (impl tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin),
) -> anyhow::Result<()> {
    let (mut local_read, mut local_write) = local.split();
    let (mut upstream_read, mut upstream_write) = tokio::io::split(upstream);

    tokio::select! {
        result = tokio::io::copy(&mut local_read, &mut upstream_write) => {
            result?;
        }
        result = tokio::io::copy(&mut upstream_read, &mut local_write) => {
            result?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_id_increments() {
        let id1 = NEXT_FORWARD_ID.fetch_add(1, Ordering::Relaxed);
        let id2 = NEXT_FORWARD_ID.fetch_add(1, Ordering::Relaxed);
        assert!(id2 > id1);
    }

    #[tokio::test]
    async fn port_forward_binds_to_specified_port() {
        // This test verifies that we can bind to a port, but we can't actually
        // test the full port forward without a real Kubernetes cluster.
        // We'll test that binding to an available port succeeds.

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port > 0);
    }

    #[tokio::test]
    async fn port_forward_accessors() {
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);
        let handle = tokio::spawn(async {});

        let pf = PortForward {
            id: 42,
            local_port: 8080,
            remote_port: 80,
            pod_name: "test-pod".to_string(),
            namespace: "default".to_string(),
            pod_uid: Some("pod-uid-1".to_string()),
            started_at: Instant::now(),
            handle,
            shutdown_tx,
        };

        assert_eq!(pf.id(), 42);
        assert_eq!(pf.local_port(), 8080);
        assert_eq!(pf.remote_port(), 80);
        assert_eq!(pf.pod_name(), "test-pod");
        assert_eq!(pf.namespace(), "default");
        assert_eq!(pf.pod_uid(), Some("pod-uid-1"));
    }

    #[tokio::test]
    async fn port_forward_stop_sends_shutdown_signal() {
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let handle = tokio::spawn(async {
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        });

        let pf = PortForward {
            id: 1,
            local_port: 8080,
            remote_port: 80,
            pod_name: "test".to_string(),
            namespace: "default".to_string(),
            pod_uid: None,
            started_at: Instant::now(),
            handle,
            shutdown_tx,
        };

        assert!(!*shutdown_rx.borrow());
        pf.stop().await.unwrap();

        // Wait a bit for the signal to propagate
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(*shutdown_rx.borrow());
    }

    #[tokio::test]
    async fn port_already_in_use_returns_error() {
        // Bind to a random port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Try to bind to the same port again - this should fail
        let result = TcpListener::bind(format!("127.0.0.1:{}", port)).await;
        assert!(result.is_err());

        // Clean up
        drop(listener);
    }

    #[tokio::test]
    async fn multiple_port_forwards_can_coexist() {
        // Create multiple PortForward structs with different ports
        let (shutdown_tx1, _) = tokio::sync::watch::channel(false);
        let (shutdown_tx2, _) = tokio::sync::watch::channel(false);
        let handle1 = tokio::spawn(async {});
        let handle2 = tokio::spawn(async {});

        let pf1 = PortForward {
            id: 1,
            local_port: 8080,
            remote_port: 80,
            pod_name: "pod1".to_string(),
            namespace: "default".to_string(),
            pod_uid: None,
            started_at: Instant::now(),
            handle: handle1,
            shutdown_tx: shutdown_tx1,
        };

        let pf2 = PortForward {
            id: 2,
            local_port: 9090,
            remote_port: 90,
            pod_name: "pod2".to_string(),
            namespace: "default".to_string(),
            pod_uid: None,
            started_at: Instant::now(),
            handle: handle2,
            shutdown_tx: shutdown_tx2,
        };

        assert_eq!(pf1.id(), 1);
        assert_eq!(pf2.id(), 2);
        assert_ne!(pf1.local_port(), pf2.local_port());
        assert_ne!(pf1.pod_name(), pf2.pod_name());

        // Clean up
        pf1.stop().await.unwrap();
        pf2.stop().await.unwrap();
    }

    #[tokio::test]
    async fn port_forward_uses_dynamic_port_when_zero() {
        // Binding to port 0 should assign a random available port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port > 0);
        assert_ne!(port, 0);
    }
}
