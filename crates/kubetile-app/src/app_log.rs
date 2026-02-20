use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, OnceLock};

use tracing_subscriber::fmt::MakeWriter;

const MAX_LOG_LINES: usize = 2000;

static LOG_BUFFER: OnceLock<Arc<Mutex<VecDeque<String>>>> = OnceLock::new();

fn buffer() -> Arc<Mutex<VecDeque<String>>> {
    LOG_BUFFER.get_or_init(|| Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES)))).clone()
}

fn is_suppressed(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("fsnotify") || lower.contains("too many open files")
}

fn commit_line(line: &str) {
    if line.is_empty() || is_suppressed(line) {
        return;
    }
    let buf = buffer();
    let Ok(mut guard) = buf.lock() else { return };
    guard.push_back(line.to_string());
    while guard.len() > MAX_LOG_LINES {
        let _ = guard.pop_front();
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
        AppLogWriter { line_buf: String::new() }
    }
}

/// Buffers bytes until a complete newline-terminated line is received, then
/// applies the suppression filter on the full line. This prevents false
/// negatives when the tracing formatter splits a single log event across
/// multiple `write()` calls.
pub struct AppLogWriter {
    line_buf: String,
}

impl Write for AppLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        self.line_buf.push_str(&text);

        while let Some(pos) = self.line_buf.find('\n') {
            let line = self.line_buf.drain(..=pos).collect::<String>();
            commit_line(line.trim_end_matches('\n'));
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.line_buf.is_empty() {
            let line = std::mem::take(&mut self.line_buf);
            commit_line(line.trim_end_matches('\n'));
        }
        Ok(())
    }
}
