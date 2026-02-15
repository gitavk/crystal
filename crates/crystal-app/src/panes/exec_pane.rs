use std::any::Any;
use std::cell::RefCell;
use std::io::{Read, Write};
use std::sync::atomic::Ordering;

use kube::api::TerminalSize;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crystal_core::ExecSession;
use crystal_terminal::render_terminal_screen;
use crystal_tui::pane::{Pane, PaneCommand, ViewType};
use crystal_tui::theme;

pub struct ExecPane {
    view_type: ViewType,
    pod_name: String,
    container: String,
    namespace: String,
    session: Option<ExecSession>,
    vt: RefCell<vt100::Parser>,
    desired_size: RefCell<Option<(u16, u16)>>,
    applied_size: Option<(u16, u16)>,
    status: String,
    exited: bool,
}

impl ExecPane {
    pub fn new(pod_name: String, container: String, namespace: String) -> Self {
        Self {
            view_type: ViewType::Exec(pod_name.clone()),
            pod_name,
            container,
            namespace,
            session: None,
            vt: RefCell::new(vt100::Parser::new(48, 160, 10_000)),
            desired_size: RefCell::new(None),
            applied_size: None,
            status: "Connecting...".into(),
            exited: false,
        }
    }

    pub fn attach_session(&mut self, session: ExecSession) {
        self.session = Some(session);
        self.applied_size = None;
        self.status = "Connected".into();
        self.exited = false;
    }

    pub fn set_error(&mut self, error: String) {
        self.session = None;
        self.status = format!("Error: {error}");
        self.exited = false;
    }

    pub fn exited(&self) -> bool {
        self.exited
    }

    pub fn poll(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };

        if let Some((cols, rows)) = *self.desired_size.borrow() {
            if self.applied_size != Some((cols, rows))
                && session.resize_tx.try_send(TerminalSize { width: cols, height: rows }).is_ok()
            {
                self.applied_size = Some((cols, rows));
            }
        }

        let mut buf = [0u8; 4096];
        loop {
            match session.reader.read(&mut buf) {
                Ok(0) => {
                    self.status = "Exited".into();
                    self.exited = true;
                    break;
                }
                Ok(n) => {
                    self.vt.borrow_mut().process(&buf[..n]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    self.status = format!("I/O error: {e}");
                    break;
                }
            }
        }

        if session.exited.load(Ordering::Acquire) {
            self.status = "Exited".into();
            self.exited = true;
        }
    }

    fn render_title(&self) -> String {
        format!("[exec:{}/{} @ {}]", self.pod_name, self.container, self.namespace)
    }
}

impl Pane for ExecPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_color = if focused { theme::ACCENT } else { theme::BORDER_COLOR };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(format!(" {} ", self.render_title()))
            .title_style(Style::default().fg(theme::ACCENT).bold());

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height == 0 {
            return;
        }

        let content_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: inner.height.saturating_sub(1) };
        let mut vt = self.vt.borrow_mut();
        let rows = content_area.height.max(1);
        let cols = content_area.width.max(1);
        vt.set_size(rows, cols);
        self.desired_size.replace(Some((cols, rows)));
        render_terminal_screen(vt.screen(), content_area, frame.buffer_mut());
        if self.status == "Connecting..." {
            frame.render_widget(Paragraph::new("Waiting for exec output..."), content_area);
        }

        let footer_area =
            Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(1), width: inner.width, height: 1 };
        frame.render_widget(
            Paragraph::new(format!("{} | Insert mode to type", self.status))
                .style(Style::default().fg(theme::TEXT_DIM).bg(theme::STATUS_BG)),
            footer_area,
        );
    }

    fn handle_command(&mut self, cmd: &PaneCommand) {
        if let PaneCommand::SendInput(input) = cmd {
            if let Some(session) = self.session.as_mut() {
                if session.writer.write_all(input.as_bytes()).is_err() {
                    self.status = "Write failed".into();
                }
            }
        }
    }

    fn view_type(&self) -> &ViewType {
        &self.view_type
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
