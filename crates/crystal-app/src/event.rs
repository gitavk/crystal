use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};
use crystal_core::informer::ResourceEvent;
use crystal_core::PodSummary;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    #[allow(dead_code)]
    Resize(u16, u16),
    KubeUpdate(ResourceEvent<PodSummary>),
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

    pub fn forward_kube_events(&self, mut kube_rx: mpsc::Receiver<ResourceEvent<PodSummary>>) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            while let Some(event) = kube_rx.recv().await {
                if tx.send(AppEvent::KubeUpdate(event)).is_err() {
                    break;
                }
            }
        });
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
