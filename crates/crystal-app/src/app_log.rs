use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, OnceLock};

use tracing_subscriber::fmt::MakeWriter;

const MAX_LOG_LINES: usize = 2000;

static LOG_BUFFER: OnceLock<Arc<Mutex<VecDeque<String>>>> = OnceLock::new();

fn buffer() -> Arc<Mutex<VecDeque<String>>> {
    LOG_BUFFER.get_or_init(|| Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES)))).clone()
}

fn push_lines(text: &str) {
    let buf = buffer();
    let mut guard = match buf.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        guard.push_back(line.to_string());
        while guard.len() > MAX_LOG_LINES {
            let _ = guard.pop_front();
        }
    }
}

pub fn recent_lines(limit: usize) -> Vec<String> {
    let buf = buffer();
    let guard = match buf.lock() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    let skip = guard.len().saturating_sub(limit);
    guard.iter().skip(skip).cloned().collect()
}

#[derive(Clone, Default)]
pub struct AppLogMakeWriter;

impl<'a> MakeWriter<'a> for AppLogMakeWriter {
    type Writer = AppLogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        AppLogWriter { stderr: io::stderr() }
    }
}

pub struct AppLogWriter {
    stderr: io::Stderr,
}

impl Write for AppLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.stderr.write(buf)?;
        if written > 0 {
            push_lines(&String::from_utf8_lossy(&buf[..written]));
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stderr.flush()
    }
}
