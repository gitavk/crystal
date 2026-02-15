use std::time::Duration;

use futures::AsyncBufReadExt;
use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::api::LogParams;
use kube::{Api, Client};
use tokio::sync::mpsc;
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: Option<jiff::Timestamp>,
    pub content: String,
    pub container: String,
    pub is_stderr: bool,
}

#[derive(Debug, Clone)]
pub struct LogRequest {
    pub pod_name: String,
    pub namespace: String,
    pub container: Option<String>,
    pub follow: bool,
    pub tail_lines: Option<i64>,
    pub since_seconds: Option<i64>,
    pub previous: bool,
    pub timestamps: bool,
}

impl Default for LogRequest {
    fn default() -> Self {
        Self {
            pod_name: String::new(),
            namespace: String::new(),
            container: None,
            follow: true,
            tail_lines: Some(1000),
            since_seconds: None,
            previous: false,
            timestamps: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamStatus {
    Streaming,
    Reconnecting { attempt: u32 },
    Stopped,
    Error,
}

pub struct LogStream {
    rx: mpsc::UnboundedReceiver<LogLine>,
    status_rx: mpsc::UnboundedReceiver<StreamStatus>,
    status: StreamStatus,
    cancel: tokio::sync::watch::Sender<bool>,
}

impl LogStream {
    pub async fn start(client: &Client, request: LogRequest) -> anyhow::Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let (status_tx, status_rx) = mpsc::unbounded_channel();
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

        let client = client.clone();
        let request = request.clone();

        tokio::spawn(async move {
            stream_logs(client, request, tx, status_tx, cancel_rx).await;
        });

        Ok(Self { rx, status_rx, status: StreamStatus::Streaming, cancel: cancel_tx })
    }

    pub fn next_lines(&mut self) -> Vec<LogLine> {
        let mut lines = Vec::new();
        while let Ok(line) = self.rx.try_recv() {
            lines.push(line);
        }
        while let Ok(status) = self.status_rx.try_recv() {
            self.status = status;
        }
        lines
    }

    pub fn status(&self) -> StreamStatus {
        self.status
    }

    pub fn stop(&self) {
        let _ = self.cancel.send(true);
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, StreamStatus::Streaming | StreamStatus::Reconnecting { .. })
    }
}

async fn stream_logs(
    client: Client,
    request: LogRequest,
    tx: mpsc::UnboundedSender<LogLine>,
    status_tx: mpsc::UnboundedSender<StreamStatus>,
    mut cancel_rx: tokio::sync::watch::Receiver<bool>,
) {
    let pods: Api<Pod> = Api::namespaced(client, &request.namespace);
    let container = request.container.clone().unwrap_or_default();
    let mut consecutive_failures: u32 = 0;
    let mut last_line_seen_at: Option<std::time::Instant> = None;

    loop {
        if *cancel_rx.borrow() {
            let _ = status_tx.send(StreamStatus::Stopped);
            return;
        }

        let mut params = LogParams {
            follow: request.follow,
            previous: request.previous,
            timestamps: request.timestamps,
            ..Default::default()
        };

        if let Some(tail) = request.tail_lines {
            if consecutive_failures == 0 {
                params.tail_lines = Some(tail);
            }
        }

        if !container.is_empty() {
            params.container = Some(container.clone());
        }

        let reconnect_since = reconnect_since_seconds(request.since_seconds, last_line_seen_at.map(|t| t.elapsed()));

        if let Some(since) = reconnect_since {
            params.since_seconds = Some(since);
        }

        match pods.log_stream(&request.pod_name, &params).await {
            Ok(log_stream) => {
                consecutive_failures = 0;
                let _ = status_tx.send(StreamStatus::Streaming);

                let mut lines_stream = log_stream.lines();
                loop {
                    tokio::select! {
                        item = lines_stream.next() => {
                            match item {
                                Some(Ok(raw_line)) => {
                                    let log_line = parse_log_line(&raw_line, &container);
                                    if tx.send(log_line).is_err() {
                                        return;
                                    }
                                    last_line_seen_at = Some(std::time::Instant::now());
                                }
                                Some(Err(e)) => {
                                    warn!("Log stream error: {e}");
                                    break;
                                }
                                None => {
                                    debug!("Log stream ended");
                                    if !request.follow {
                                        let _ = status_tx.send(StreamStatus::Stopped);
                                        return;
                                    }
                                    break;
                                }
                            }
                        }
                        _ = cancel_rx.changed() => {
                            let _ = status_tx.send(StreamStatus::Stopped);
                            return;
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to start log stream: {e}");
                consecutive_failures += 1;
            }
        }

        if consecutive_failures >= 5 {
            let _ = status_tx.send(StreamStatus::Error);
            return;
        }

        let backoff = backoff_duration(consecutive_failures);
        let _ = status_tx.send(StreamStatus::Reconnecting { attempt: consecutive_failures });
        debug!("Reconnecting in {}s (attempt {})", backoff.as_secs(), consecutive_failures);

        tokio::select! {
            _ = tokio::time::sleep(backoff) => {}
            _ = cancel_rx.changed() => {
                let _ = status_tx.send(StreamStatus::Stopped);
                return;
            }
        }
    }
}

fn reconnect_since_seconds(request_since_seconds: Option<i64>, since_last_line: Option<Duration>) -> Option<i64> {
    if request_since_seconds.is_some() {
        return request_since_seconds;
    }
    since_last_line.map(|d| d.as_secs().saturating_add(1) as i64)
}

fn backoff_duration(attempt: u32) -> Duration {
    let secs = (1u64 << attempt.min(5)).min(30);
    Duration::from_secs(secs)
}

fn parse_log_line(raw: &str, default_container: &str) -> LogLine {
    let (timestamp, content) =
        if let Some(rest) = try_parse_timestamp_prefix(raw) { rest } else { (None, raw.to_string()) };

    LogLine { timestamp, content, container: default_container.to_string(), is_stderr: false }
}

fn try_parse_timestamp_prefix(line: &str) -> Option<(Option<jiff::Timestamp>, String)> {
    // K8s log timestamps: "2024-01-15T10:30:00.123456789Z content..."
    if line.len() < 20 {
        return None;
    }
    let space_idx = line.find(' ')?;
    let maybe_ts = &line[..space_idx];
    match maybe_ts.parse::<jiff::Timestamp>() {
        Ok(ts) => Some((Some(ts), line[space_idx + 1..].to_string())),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn parse_log_line_with_timestamp() {
        let raw = "2024-01-15T10:30:00.123456789Z hello world";
        let line = parse_log_line(raw, "main");
        assert!(line.timestamp.is_some());
        assert_eq!(line.content, "hello world");
        assert_eq!(line.container, "main");
        assert!(!line.is_stderr);
    }

    #[test]
    fn parse_log_line_without_timestamp() {
        let raw = "just some log output";
        let line = parse_log_line(raw, "sidecar");
        assert!(line.timestamp.is_none());
        assert_eq!(line.content, "just some log output");
        assert_eq!(line.container, "sidecar");
    }

    #[test]
    fn parse_log_line_empty_string() {
        let raw = "";
        let line = parse_log_line(raw, "ctr");
        assert!(line.timestamp.is_none());
        assert_eq!(line.content, "");
    }

    #[test]
    fn backoff_exponential_with_cap() {
        assert_eq!(backoff_duration(0), Duration::from_secs(1));
        assert_eq!(backoff_duration(1), Duration::from_secs(2));
        assert_eq!(backoff_duration(2), Duration::from_secs(4));
        assert_eq!(backoff_duration(3), Duration::from_secs(8));
        assert_eq!(backoff_duration(4), Duration::from_secs(16));
        assert_eq!(backoff_duration(5), Duration::from_secs(30));
        assert_eq!(backoff_duration(10), Duration::from_secs(30));
    }

    #[test]
    fn log_request_defaults() {
        let req = LogRequest::default();
        assert!(req.follow);
        assert_eq!(req.tail_lines, Some(1000));
        assert!(req.timestamps);
        assert!(!req.previous);
        assert!(req.container.is_none());
    }

    #[test]
    fn reconnect_since_prefers_request_value() {
        let computed = reconnect_since_seconds(Some(42), Some(Duration::from_secs(3)));
        assert_eq!(computed, Some(42));
    }

    #[test]
    fn reconnect_since_uses_elapsed_plus_one_second() {
        let computed = reconnect_since_seconds(None, Some(Duration::from_secs(3)));
        assert_eq!(computed, Some(4));
    }

    #[test]
    fn reconnect_since_none_when_no_signal() {
        let computed = reconnect_since_seconds(None, None);
        assert_eq!(computed, None);
    }

    #[test]
    fn try_parse_timestamp_prefix_valid() {
        let line = "2024-06-01T12:00:00Z some content here";
        let result = try_parse_timestamp_prefix(line);
        assert!(result.is_some());
        let (ts, content) = result.unwrap();
        assert!(ts.is_some());
        assert_eq!(content, "some content here");
    }

    #[test]
    fn try_parse_timestamp_prefix_invalid() {
        let line = "not-a-timestamp some content";
        let result = try_parse_timestamp_prefix(line);
        assert!(result.is_none());
    }

    #[test]
    fn try_parse_timestamp_prefix_short_line() {
        let line = "short";
        let result = try_parse_timestamp_prefix(line);
        assert!(result.is_none());
    }

    #[test]
    fn stream_status_variants() {
        assert_eq!(StreamStatus::Streaming, StreamStatus::Streaming);
        assert_eq!(StreamStatus::Reconnecting { attempt: 1 }, StreamStatus::Reconnecting { attempt: 1 });
        assert_ne!(StreamStatus::Streaming, StreamStatus::Stopped);
    }

    #[tokio::test]
    async fn log_stream_next_lines_returns_empty_when_no_data() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let (_status_tx, status_rx) = mpsc::unbounded_channel();
        let (cancel_tx, _cancel_rx) = tokio::sync::watch::channel(false);

        let mut stream = LogStream { rx, status_rx, status: StreamStatus::Streaming, cancel: cancel_tx };

        let lines = stream.next_lines();
        assert!(lines.is_empty());
    }

    #[tokio::test]
    async fn log_stream_next_lines_drains_channel() {
        let (tx, rx) = mpsc::unbounded_channel();
        let (_status_tx, status_rx) = mpsc::unbounded_channel();
        let (cancel_tx, _cancel_rx) = tokio::sync::watch::channel(false);

        tx.send(LogLine { timestamp: None, content: "line 1".into(), container: "main".into(), is_stderr: false })
            .unwrap();
        tx.send(LogLine { timestamp: None, content: "line 2".into(), container: "main".into(), is_stderr: false })
            .unwrap();

        let mut stream = LogStream { rx, status_rx, status: StreamStatus::Streaming, cancel: cancel_tx };

        let lines = stream.next_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].content, "line 1");
        assert_eq!(lines[1].content, "line 2");
    }

    #[tokio::test]
    async fn log_stream_stop_sets_cancel() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let (_status_tx, status_rx) = mpsc::unbounded_channel();
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

        let stream = LogStream { rx, status_rx, status: StreamStatus::Streaming, cancel: cancel_tx };

        assert!(!*cancel_rx.borrow());
        stream.stop();
        assert!(*cancel_rx.borrow());
    }

    #[test]
    fn log_stream_is_active() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let (_status_tx, status_rx) = mpsc::unbounded_channel();
        let (cancel_tx, _cancel_rx) = tokio::sync::watch::channel(false);

        let mut stream = LogStream { rx, status_rx, status: StreamStatus::Streaming, cancel: cancel_tx };

        assert!(stream.is_active());
        stream.status = StreamStatus::Reconnecting { attempt: 1 };
        assert!(stream.is_active());
        stream.status = StreamStatus::Stopped;
        assert!(!stream.is_active());
        stream.status = StreamStatus::Error;
        assert!(!stream.is_active());
    }

    #[tokio::test]
    async fn log_stream_status_updates_from_channel() {
        let (_tx, rx) = mpsc::unbounded_channel();
        let (status_tx, status_rx) = mpsc::unbounded_channel();
        let (cancel_tx, _cancel_rx) = tokio::sync::watch::channel(false);

        let mut stream = LogStream { rx, status_rx, status: StreamStatus::Streaming, cancel: cancel_tx };

        status_tx.send(StreamStatus::Reconnecting { attempt: 1 }).unwrap();
        stream.next_lines();
        assert_eq!(stream.status(), StreamStatus::Reconnecting { attempt: 1 });
    }
}
