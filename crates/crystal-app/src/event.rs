use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};
use crystal_core::{KubeClient, LogLine, LogStream, PortForward};
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
        lines: Vec<LogLine>,
    },
    LogsStreamError {
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
    NamespacesUpdated {
        namespaces: Vec<String>,
    },
}

pub struct EventHandler {
    tx: mpsc::UnboundedSender<AppEvent>,
    rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let input_tx = tx.clone();
        std::thread::spawn(move || loop {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if input_tx.send(AppEvent::Key(key)).is_err() {
                        break;
                    }
                }
                Ok(Event::Resize(w, h)) => {
                    if input_tx.send(AppEvent::Resize(w, h)).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        });

        let tick_tx = tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_rate);
            loop {
                interval.tick().await;
                if tick_tx.send(AppEvent::Tick).is_err() {
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

    pub fn drain_pending(&mut self) -> Vec<AppEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.rx.try_recv() {
            events.push(event);
        }
        events
    }
}
