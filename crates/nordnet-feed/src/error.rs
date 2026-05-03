use thiserror::Error;

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("TLS error: {0}")]
    Tls(#[from] rustls::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("decode error on line {line:?}: {source}")]
    Decode {
        #[source]
        source: serde_json::Error,
        line: String,
    },
    #[error("encode error: {0}")]
    Encode(serde_json::Error),
    #[error("frame too large: {bytes} bytes (max 1 MiB)")]
    FrameTooLarge { bytes: usize },
    #[error("connection closed")]
    Closed,
}

/// A server-side error frame payload. Surfaced as a successful event
/// (`Event::Error(ServerError)`) — not as a Rust error type — because the
/// server communicates errors in-band over the feed protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerError {
    pub msg: String,
    pub cmd: serde_json::Value,
}
