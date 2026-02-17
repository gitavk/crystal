use std::any::Any;
use std::cell::RefCell;
use std::io::Write;
use std::sync::mpsc as std_mpsc;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crystal_terminal::render_terminal_screen;
use crystal_tui::pane::{Pane, PaneCommand, ViewType};
use crystal_tui::theme::Theme;

pub struct ExecPane {
    view_type: ViewType,
    pod_name: String,
    container: String,
    namespace: String,
    pty_master: Option<Box<dyn MasterPty + Send>>,
    child: Option<Box<dyn Child + Send + Sync>>,
    output_rx: Option<std_mpsc::Receiver<Vec<u8>>>,
    writer: Option<Box<dyn Write + Send>>,
    vt: RefCell<vt100::Parser>,
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
            pty_master: None,
            child: None,
            output_rx: None,
            writer: None,
            vt: RefCell::new(vt100::Parser::new(48, 160, 10_000)),
            status: "Connecting...".into(),
            exited: false,
        }
    }

    pub fn spawn_kubectl(&mut self, context: Option<&str>) -> anyhow::Result<()> {
        let pty_system = native_pty_system();
        let pty_size = PtySize { cols: 160, rows: 48, pixel_width: 0, pixel_height: 0 };
        let pair = pty_system.openpty(pty_size)?;

        let mut cmd = CommandBuilder::new("kubectl");
        cmd.arg("exec");
        cmd.arg("-it");
        cmd.arg("-n");
        cmd.arg(&self.namespace);
        if let Some(ctx) = context {
            cmd.arg("--context");
            cmd.arg(ctx);
        }
        cmd.arg(&self.pod_name);
        if self.container != "auto" {
            cmd.arg("-c");
            cmd.arg(&self.container);
        }
        cmd.arg("--");
        cmd.arg("sh");
        cmd.arg("-c");
        cmd.arg(
            r#"if command -v zsh >/dev/null 2>&1; then exec zsh -i; fi; if command -v bash >/dev/null 2>&1; then exec bash -i; fi; exec sh -i"#,
        );

        tracing::info!(
            "exec: spawning kubectl exec -it -n {} {} (context: {:?}, container: {})",
            self.namespace,
            self.pod_name,
            context,
            self.container,
        );
        let child = pair.slave.spawn_command(cmd)?;
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        let (tx, rx) = std_mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            use std::io::Read;
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        self.pty_master = Some(pair.master);
        self.child = Some(child);
        self.output_rx = Some(rx);
        self.writer = Some(writer);
        self.status = "Connected".into();
        self.exited = false;

        Ok(())
    }

    pub fn exited(&self) -> bool {
        self.exited
    }

    pub fn poll(&mut self) {
        if let Some(rx) = &self.output_rx {
            while let Ok(data) = rx.try_recv() {
                self.vt.borrow_mut().process(&data);
            }
        }

        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    self.status = "Exited".into();
                    self.exited = true;
                }
                Ok(None) => {}
                Err(_) => {
                    self.status = "Exited".into();
                    self.exited = true;
                }
            }
        }
    }

    fn render_title(&self) -> String {
        format!("[exec:{}/{} @ {}]", self.pod_name, self.container, self.namespace)
    }
}

impl Pane for ExecPane {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool, theme: &Theme) {
        let border_style = if focused { theme.border_active } else { theme.border };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(format!(" {} ", self.render_title()))
            .title_style(Style::default().fg(theme.accent).bold());

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

        if let Some(pty_master) = &self.pty_master {
            let _ = pty_master.resize(PtySize { cols, rows, pixel_width: 0, pixel_height: 0 });
        }

        render_terminal_screen(vt.screen(), content_area, frame.buffer_mut());
        if self.status == "Connecting..." {
            frame.render_widget(Paragraph::new("Waiting for exec output..."), content_area);
        }

        let footer_area =
            Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(1), width: inner.width, height: 1 };
        frame.render_widget(
            Paragraph::new(format!("{} | Insert mode to type", self.status)).style(theme.status_bar),
            footer_area,
        );
    }

    fn handle_command(&mut self, cmd: &PaneCommand) {
        if let PaneCommand::SendInput(input) = cmd {
            if let Some(writer) = self.writer.as_mut() {
                let _ = writer.write_all(input.as_bytes());
                let _ = writer.flush();
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

impl Drop for ExecPane {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
