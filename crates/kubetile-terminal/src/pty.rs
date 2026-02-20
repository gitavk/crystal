use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
}

impl PtySession {
    pub fn spawn(
        shell: &str,
        cwd: Option<&Path>,
        env: HashMap<String, String>,
        size: (u16, u16),
    ) -> anyhow::Result<Self> {
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

        Ok(Self { master: pair.master, child, reader, writer })
    }

    pub fn resize(&self, cols: u16, rows: u16) -> anyhow::Result<()> {
        self.master.resize(PtySize { cols, rows, pixel_width: 0, pixel_height: 0 })?;
        Ok(())
    }

    pub fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn read(&mut self, buf: &mut [u8]) -> anyhow::Result<usize> {
        let n = self.reader.read(buf)?;
        Ok(n)
    }

    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub fn kill(&mut self) -> anyhow::Result<()> {
        self.child.kill()?;
        self.child.wait()?;
        Ok(())
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
    fn spawn_creates_live_process() {
        let mut session = PtySession::spawn(default_shell(), None, HashMap::new(), (80, 24)).unwrap();
        assert!(session.is_alive());
        session.kill().unwrap();
    }

    #[test]
    fn write_and_read_echo() {
        let mut session = PtySession::spawn(default_shell(), None, HashMap::new(), (80, 24)).unwrap();

        session.write(b"echo hello\n").unwrap();

        thread::sleep(Duration::from_millis(500));

        let mut buf = [0u8; 4096];
        let mut output = String::new();
        // Read available output in a loop (non-blocking reads may return partial data)
        for _ in 0..10 {
            match session.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
                Err(_) => break,
            }
            if output.contains("hello") {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }

        assert!(output.contains("hello"), "Expected 'hello' in output, got: {output}");
        session.kill().unwrap();
    }

    #[test]
    fn resize_succeeds() {
        let mut session = PtySession::spawn(default_shell(), None, HashMap::new(), (80, 24)).unwrap();
        assert!(session.resize(40, 10).is_ok());
        session.kill().unwrap();
    }

    #[test]
    fn kill_terminates_child() {
        let mut session = PtySession::spawn(default_shell(), None, HashMap::new(), (80, 24)).unwrap();
        assert!(session.is_alive());
        session.kill().unwrap();
        assert!(!session.is_alive());
    }

    #[test]
    fn spawn_invalid_shell_returns_error() {
        let result = PtySession::spawn("/nonexistent/shell", None, HashMap::new(), (80, 24));
        assert!(result.is_err());
    }

    #[test]
    fn spawn_with_custom_env() {
        let mut env = HashMap::new();
        env.insert("CRYSTAL_TEST_VAR".to_string(), "crystal_value".to_string());

        let mut session = PtySession::spawn(default_shell(), None, env, (80, 24)).unwrap();

        session.write(b"echo $CRYSTAL_TEST_VAR\n").unwrap();

        thread::sleep(Duration::from_millis(500));

        let mut buf = [0u8; 4096];
        let mut output = String::new();
        for _ in 0..10 {
            match session.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
                Err(_) => break,
            }
            if output.contains("crystal_value") {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }

        assert!(output.contains("crystal_value"), "Expected 'crystal_value' in output, got: {output}");
        session.kill().unwrap();
    }
}
