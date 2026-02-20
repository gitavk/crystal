use std::fmt;

#[derive(Debug)]
pub enum KubeError {
    NoKubeconfig,
    ConnectionFailed(String),
    ApiError(String),
    WatchError(String),
}

impl fmt::Display for KubeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoKubeconfig => write!(f, "No kubeconfig found"),
            Self::ConnectionFailed(msg) => write!(f, "Connection failed: {msg}"),
            Self::ApiError(msg) => write!(f, "API error: {msg}"),
            Self::WatchError(msg) => write!(f, "Watch error: {msg}"),
        }
    }
}

impl std::error::Error for KubeError {}
