use thiserror::Error;

/// Maximum number of bytes of a wire line included verbatim in
/// [`FeedError::Decode`] error messages. Longer lines are truncated to
/// avoid leaking session keys / order details into log pipelines.
pub const MAX_LINE_FOR_DISPLAY: usize = 256;

#[derive(Debug, Error)]
pub enum FeedError {
    /// TLS handshake / negotiation error. Surfaced separately from
    /// [`FeedError::Io`] so callers can distinguish certificate / handshake
    /// failures from raw socket / network failures.
    #[error("TLS error: {0}")]
    Tls(#[from] rustls::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Wire frame failed JSON parsing. The `line` field is truncated at
    /// construction (see [`MAX_LINE_FOR_DISPLAY`]) to avoid leaking
    /// credentials in logs — callers should still avoid logging error
    /// payloads at INFO level.
    #[error("decode error on line {line:?}: {source}")]
    Decode {
        #[source]
        source: serde_json::Error,
        line: String,
    },
    #[error("encode error: {0}")]
    Encode(#[source] serde_json::Error),
    #[error("frame too large (max 1 MiB)")]
    FrameTooLarge,
    #[error("connection closed")]
    Closed,
}

/// Truncate a wire line for inclusion in a [`FeedError::Decode`] message.
/// UTF-8 boundary safe.
pub(crate) fn redact_line(line: String) -> String {
    if line.len() <= MAX_LINE_FOR_DISPLAY {
        return line;
    }
    let mut end = MAX_LINE_FOR_DISPLAY;
    while !line.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}…[truncated, {} of {} bytes]",
        &line[..end],
        end,
        line.len()
    )
}

/// A server-side error frame payload. Surfaced as a successful event
/// (`Event::Error(ServerError)`) — not as a Rust error type — because the
/// server communicates errors in-band over the feed protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerError {
    pub msg: String,
    pub cmd: serde_json::Value,
}
