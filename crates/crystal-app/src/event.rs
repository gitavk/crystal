use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    #[allow(dead_code)]
    Resize(u16, u16),
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

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
                if tx.send(event).is_err() {
                    break;
                }
            }
        });

        Self { rx }
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
