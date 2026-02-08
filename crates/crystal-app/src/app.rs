use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::backend::Backend;
use ratatui::Terminal;

use crate::event::{AppEvent, EventHandler};

pub struct App {
    pub running: bool,
    pub tick_rate: Duration,
}

impl App {
    pub fn new(tick_rate_ms: u64) -> Self {
        Self { running: true, tick_rate: Duration::from_millis(tick_rate_ms) }
    }

    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> anyhow::Result<()> {
        let mut events = EventHandler::new(self.tick_rate);

        while self.running {
            terminal.draw(crystal_tui::layout::render_root)?;

            match events.next().await? {
                AppEvent::Key(key) => self.handle_key(key),
                AppEvent::Tick => {}
                AppEvent::Resize(_, _) => {}
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        if let KeyCode::Char('q') = key.code {
            self.running = false;
        }
    }
}
