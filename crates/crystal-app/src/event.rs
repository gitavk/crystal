use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};
use crystal_core::{ExecSession, KubeClient, LogStream, PortForward};
use crystal_tui::pane::{PaneId, ResourceKind};
use crystal_tui::widgets::toast::ToastMessage;
use tokio::sync::mpsc;

pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    #[allow(dead_code)]
    Resize(u16, u16),
    /// Resource update for a specific pane.
    /// The Vec<Vec<String>> is pre-rendered rows (via ResourceSummary::row()).
    /// This erases the generic S type so AppEvent doesn't need type params.
    ResourceUpdate {
        pane_id: PaneId,
        #[allow(dead_code)]
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    ResourceError {
        pane_id: PaneId,
        error: String,
    },
    Toast(ToastMessage),
    YamlReady {
        pane_id: PaneId,
        kind: ResourceKind,
        name: String,
        content: String,
    },
    LogsStreamReady {
        pane_id: PaneId,
        stream: LogStream,
    },
    LogsSnapshotReady {
        pane_id: PaneId,
        lines: Vec<String>,
    },
    LogsStreamError {
        pane_id: PaneId,
        error: String,
    },
    ExecSessionReady {
        pane_id: PaneId,
        session: ExecSession,
    },
    ExecSessionError {
        pane_id: PaneId,
        error: String,
    },
    PortForwardReady {
        forward: PortForward,
    },
    PortForwardPromptReady {
        pod: String,
        namespace: String,
        suggested_remote: u16,
    },
    ContextSwitchReady {
        client: KubeClient,
        namespaces: Vec<String>,
    },
    ContextSwitchError {
        context: String,
        error: String,
    },
}

pub struct EventHandler {
    tx: mpsc::UnboundedSender<AppEvent>,
    rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_clone = tx.clone();

        tokio::spawn(async move {
            let mut tick_interval = tokio::time::interval(tick_rate);
            loop {
                let event = tokio::select! {
                    _ = tick_interval.tick() => AppEvent::Tick,
                    maybe = poll_crossterm_event() => match maybe {
                        Some(e) => e,
                        None => continue,
                    },
                };
                if tx_clone.send(event).is_err() {
                    break;
                }
            }
        });

        Self { tx, rx }
    }

    pub fn app_tx(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.tx.clone()
    }

    pub async fn next(&mut self) -> anyhow::Result<AppEvent> {
        self.rx.recv().await.ok_or_else(|| anyhow::anyhow!("Event channel closed"))
    }
}

async fn poll_crossterm_event() -> Option<AppEvent> {
    let event = tokio::task::spawn_blocking(|| {
        if event::poll(Duration::from_millis(50)).ok()? {
            event::read().ok()
        } else {
            None
        }
    })
    .await
    .ok()??;

    match event {
        Event::Key(key) => Some(AppEvent::Key(key)),
        Event::Resize(w, h) => Some(AppEvent::Resize(w, h)),
        _ => None,
    }
}
