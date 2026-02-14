use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

pub type SessionId = u64;

#[derive(Debug, Clone, PartialEq)]
pub enum SessionKind {
    Shell,
    Exec { pod: String, container: String, namespace: String },
}

pub struct TerminalManager {
    sessions: HashMap<SessionId, TerminalSession>,
    next_id: u64,
}

struct TerminalSession {
    pty_master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    reader: Box<dyn Read + Send>,
    writer: Box<dyn std::io::Write + Send>,
    vt: vt100::Parser,
    title: String,
    kind: SessionKind,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self { sessions: HashMap::new(), next_id: 1 }
    }

    pub fn spawn_shell(
        &mut self,
        shell: &str,
        cwd: Option<&Path>,
        env: HashMap<String, String>,
        size: (u16, u16),
    ) -> anyhow::Result<SessionId> {
        let pty_system = native_pty_system();
        let pty_size = PtySize { cols: size.0, rows: size.1, pixel_width: 0, pixel_height: 0 };
        let pair = pty_system.openpty(pty_size)?;

        let mut cmd = CommandBuilder::new(shell);
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }
        for (key, value) in &env {
            cmd.env(key, value);
        }

        let child = pair.slave.spawn_command(cmd)?;
        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        let id = self.next_id;
        self.next_id += 1;

        let session = TerminalSession {
            pty_master: pair.master,
            child,
            reader,
            writer,
            vt: vt100::Parser::new(size.1, size.0, 0),
            title: format!("shell-{id}"),
            kind: SessionKind::Shell,
        };
        self.sessions.insert(id, session);
        Ok(id)
    }

    pub fn write_input(&mut self, id: SessionId, data: &[u8]) -> anyhow::Result<()> {
        let session = self.sessions.get_mut(&id).ok_or_else(|| anyhow::anyhow!("unknown session {id}"))?;
        use std::io::Write;
        session.writer.write_all(data)?;
        session.writer.flush()?;
        Ok(())
    }

    pub fn resize(&mut self, id: SessionId, cols: u16, rows: u16) -> anyhow::Result<()> {
        let session = self.sessions.get_mut(&id).ok_or_else(|| anyhow::anyhow!("unknown session {id}"))?;
        session.pty_master.resize(PtySize { cols, rows, pixel_width: 0, pixel_height: 0 })?;
        session.vt.set_size(rows, cols);
        Ok(())
    }

    pub fn poll_output(&mut self, id: SessionId) -> anyhow::Result<()> {
        let session = self.sessions.get_mut(&id).ok_or_else(|| anyhow::anyhow!("unknown session {id}"))?;
        let mut buf = [0u8; 4096];
        loop {
            match session.reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => session.vt.process(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    pub fn poll_all(&mut self) -> Vec<SessionId> {
        let mut exited = Vec::new();
        for (&id, session) in &mut self.sessions {
            match session.child.try_wait() {
                Ok(Some(_)) => exited.push(id),
                Ok(None) => {}
                Err(_) => exited.push(id),
            }
        }
        exited
    }

    pub fn close(&mut self, id: SessionId) -> anyhow::Result<()> {
        let mut session = self.sessions.remove(&id).ok_or_else(|| anyhow::anyhow!("unknown session {id}"))?;
        let _ = session.child.kill();
        let _ = session.child.wait();
        Ok(())
    }

    pub fn screen(&self, id: SessionId) -> Option<&vt100::Screen> {
        self.sessions.get(&id).map(|s| s.vt.screen())
    }

    pub fn session_info(&self, id: SessionId) -> Option<(&SessionKind, &str)> {
        self.sessions.get(&id).map(|s| (&s.kind, s.title.as_str()))
    }

    pub fn has_session(&self, id: SessionId) -> bool {
        self.sessions.contains_key(&id)
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn default_shell() -> &'static str {
        "/bin/sh"
    }

    #[test]
    fn spawn_shell_returns_valid_session_id() {
        let mut mgr = TerminalManager::new();
        let id = mgr.spawn_shell(default_shell(), None, HashMap::new(), (80, 24)).unwrap();
        assert!(mgr.has_session(id));
        assert_eq!(mgr.session_count(), 1);
        mgr.close(id).unwrap();
    }

    #[test]
    fn spawn_shell_returns_different_ids() {
        let mut mgr = TerminalManager::new();
        let id1 = mgr.spawn_shell(default_shell(), None, HashMap::new(), (80, 24)).unwrap();
        let id2 = mgr.spawn_shell(default_shell(), None, HashMap::new(), (80, 24)).unwrap();
        assert_ne!(id1, id2);
        assert_eq!(mgr.session_count(), 2);
        mgr.close(id1).unwrap();
        mgr.close(id2).unwrap();
    }

    #[test]
    fn write_input_invalid_session_returns_error() {
        let mut mgr = TerminalManager::new();
        assert!(mgr.write_input(9999, b"hello").is_err());
    }

    #[test]
    fn close_removes_session() {
        let mut mgr = TerminalManager::new();
        let id = mgr.spawn_shell(default_shell(), None, HashMap::new(), (80, 24)).unwrap();
        assert!(mgr.has_session(id));
        mgr.close(id).unwrap();
        assert!(!mgr.has_session(id));
    }

    #[test]
    fn screen_returns_none_for_unknown_id() {
        let mgr = TerminalManager::new();
        assert!(mgr.screen(42).is_none());
    }

    #[test]
    fn poll_all_detects_exited_sessions() {
        let mut mgr = TerminalManager::new();
        let id = mgr.spawn_shell(default_shell(), None, HashMap::new(), (80, 24)).unwrap();

        mgr.write_input(id, b"exit\n").unwrap();
        thread::sleep(Duration::from_millis(500));

        let exited = mgr.poll_all();
        assert!(exited.contains(&id));
        mgr.close(id).unwrap();
    }

    #[test]
    fn resize_propagates_to_pty_and_vt() {
        let mut mgr = TerminalManager::new();
        let id = mgr.spawn_shell(default_shell(), None, HashMap::new(), (80, 24)).unwrap();

        assert!(mgr.resize(id, 120, 40).is_ok());

        let screen = mgr.screen(id).unwrap();
        let (rows, cols) = screen.size();
        assert_eq!(rows, 40);
        assert_eq!(cols, 120);

        mgr.close(id).unwrap();
    }

    #[test]
    fn session_info_returns_kind_and_title() {
        let mut mgr = TerminalManager::new();
        let id = mgr.spawn_shell(default_shell(), None, HashMap::new(), (80, 24)).unwrap();

        let (kind, title) = mgr.session_info(id).unwrap();
        assert_eq!(*kind, SessionKind::Shell);
        assert!(title.starts_with("shell-"));

        mgr.close(id).unwrap();
    }
}
