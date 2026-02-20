use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::channel::mpsc as futures_mpsc;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{AttachParams, AttachedProcess, TerminalSize};
use kube::{Api, Client};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

pub struct ExecSession {
    pub reader: ChannelReader,
    pub writer: ChannelWriter,
    pub resize_tx: futures_mpsc::Sender<TerminalSize>,
    pub exited: Arc<AtomicBool>,
}

impl ExecSession {
    pub async fn start(
        client: Client,
        pod_name: &str,
        namespace: &str,
        container: Option<&str>,
        command: Vec<String>,
    ) -> anyhow::Result<Self> {
        let pods: Api<Pod> = Api::namespaced(client, namespace);

        let mut ap = AttachParams::interactive_tty();
        if let Some(c) = container {
            ap = ap.container(c);
        }

        let cmd = if command.is_empty() { vec!["/bin/sh".to_string()] } else { command };

        let mut attached: AttachedProcess = pods.exec(pod_name, cmd, &ap).await?;

        let mut stdin_async = attached.stdin().ok_or_else(|| anyhow::anyhow!("exec: stdin not available"))?;
        let mut stdout_async = attached.stdout().ok_or_else(|| anyhow::anyhow!("exec: stdout not available"))?;
        let resize_tx =
            attached.terminal_size().ok_or_else(|| anyhow::anyhow!("exec: terminal resize not available"))?;

        let (stdout_tx, stdout_rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let exited = Arc::new(AtomicBool::new(false));
        let exited_clone = exited.clone();

        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            loop {
                match stdout_async.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if stdout_tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            exited_clone.store(true, Ordering::Release);
        });

        tokio::spawn(async move {
            while let Some(data) = stdin_rx.recv().await {
                if stdin_async.write_all(&data).await.is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            let _ = attached.join().await;
        });

        Ok(Self {
            reader: ChannelReader { rx: stdout_rx, pending: Vec::new(), pos: 0 },
            writer: ChannelWriter { tx: stdin_tx },
            resize_tx,
            exited,
        })
    }
}

pub struct ChannelReader {
    rx: std::sync::mpsc::Receiver<Vec<u8>>,
    pending: Vec<u8>,
    pos: usize,
}

impl std::io::Read for ChannelReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.pending.len() {
            match self.rx.try_recv() {
                Ok(data) => {
                    self.pending = data;
                    self.pos = 0;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    return Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "no data"));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return Ok(0);
                }
            }
        }
        let remaining = &self.pending[self.pos..];
        let n = buf.len().min(remaining.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        self.pos += n;
        Ok(n)
    }
}

pub struct ChannelWriter {
    tx: mpsc::UnboundedSender<Vec<u8>>,
}

impl std::io::Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.tx
            .send(buf.to_vec())
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "exec session closed"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
impl ExecSession {
    pub fn mock(
        stdout_rx: std::sync::mpsc::Receiver<Vec<u8>>,
        stdin_tx: mpsc::UnboundedSender<Vec<u8>>,
        resize_tx: futures_mpsc::Sender<TerminalSize>,
        exited: Arc<AtomicBool>,
    ) -> Self {
        Self {
            reader: ChannelReader { rx: stdout_rx, pending: Vec::new(), pos: 0 },
            writer: ChannelWriter { tx: stdin_tx },
            resize_tx,
            exited,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn channel_reader_returns_wouldblock_when_empty() {
        let (_tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let mut reader = ChannelReader { rx, pending: Vec::new(), pos: 0 };
        let mut buf = [0u8; 16];
        let err = reader.read(&mut buf).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
    }

    #[test]
    fn channel_reader_reads_sent_data() {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let mut reader = ChannelReader { rx, pending: Vec::new(), pos: 0 };
        tx.send(b"hello".to_vec()).unwrap();
        let mut buf = [0u8; 16];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"hello");
    }

    #[test]
    fn channel_reader_returns_eof_when_disconnected() {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        drop(tx);
        let mut reader = ChannelReader { rx, pending: Vec::new(), pos: 0 };
        let mut buf = [0u8; 16];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn channel_reader_handles_partial_reads() {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let mut reader = ChannelReader { rx, pending: Vec::new(), pos: 0 };
        tx.send(b"hello world".to_vec()).unwrap();
        let mut buf = [0u8; 5];
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"hello");
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b" worl");
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"d");
    }

    #[test]
    fn channel_writer_sends_data() {
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let mut writer = ChannelWriter { tx };
        use std::io::Write;
        writer.write_all(b"test").unwrap();
        let data = rx.try_recv().unwrap();
        assert_eq!(data, b"test");
    }

    #[test]
    fn channel_writer_returns_error_when_closed() {
        let (tx, rx) = mpsc::unbounded_channel::<Vec<u8>>();
        drop(rx);
        let mut writer = ChannelWriter { tx };
        use std::io::Write;
        let err = writer.write(b"test").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn exited_flag_defaults_to_false() {
        let exited = Arc::new(AtomicBool::new(false));
        assert!(!exited.load(Ordering::Acquire));
    }
}
