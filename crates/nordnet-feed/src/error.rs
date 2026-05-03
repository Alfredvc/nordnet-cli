use std::time::Duration;
use thiserror::Error;

/// Maximum number of bytes of a wire line included verbatim in
/// [`FeedError::Decode`] error messages. Longer lines are truncated to
/// avoid leaking session keys / order details into log pipelines.
pub(crate) const MAX_LINE_FOR_DISPLAY: usize = 256;

#[derive(Debug, Error)]
pub enum FeedError {
    /// TLS handshake / negotiation error. Surfaced separately from
    /// [`FeedError::Io`] so callers can distinguish certificate / handshake
    /// failures from raw socket / network failures.
    #[error("TLS error: {0}")]
    Tls(#[from] rustls::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Wire frame failed envelope-level JSON parsing. The connection is
    /// fundamentally broken; the client is in a terminal state. The
    /// `line` field is truncated to 256 bytes (UTF-8 char-boundary
    /// safe) to avoid leaking credentials in logs — callers should
    /// still avoid logging error payloads at INFO level.
    ///
    /// Per-payload type mismatches (e.g. a `price` frame whose `bid` is
    /// the wrong shape) do NOT surface here — they arrive as the
    /// non-terminal `DecodeFailed` event variant on the feed enum.
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
    /// TCP / TLS connect did not complete within the configured budget
    /// (see `FeedConfig::connect_timeout`).
    #[error("connect timed out after {0:?}")]
    ConnectTimeout(Duration),
    /// No frame received from the server within the configured budget
    /// (see `FeedConfig::heartbeat_timeout`). Detects half-open
    /// connections that the OS has not torn down. Terminal — the client
    /// is now `Closed`.
    #[error("no frame received within {0:?} (heartbeat watchdog)")]
    HeartbeatTimeout(Duration),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_line_passes_short_input_unchanged() {
        let s = "short".to_owned();
        assert_eq!(redact_line(s), "short");
    }

    #[test]
    fn redact_line_passes_input_at_max_unchanged() {
        let s = "a".repeat(MAX_LINE_FOR_DISPLAY);
        let out = redact_line(s.clone());
        assert_eq!(out, s);
    }

    #[test]
    fn redact_line_truncates_oversized_input() {
        let s = "a".repeat(MAX_LINE_FOR_DISPLAY + 100);
        let out = redact_line(s);
        assert!(out.contains("…[truncated"));
        assert!(out.starts_with(&"a".repeat(MAX_LINE_FOR_DISPLAY)));
    }

    #[test]
    fn redact_line_truncates_on_utf8_char_boundary() {
        // Build a string whose byte-256 boundary lands inside a multi-byte
        // codepoint. `é` is two bytes; pad with 255 ASCII bytes then `é`.
        let mut s = "a".repeat(MAX_LINE_FOR_DISPLAY - 1);
        s.push('é');
        s.push_str(&"b".repeat(100));
        let out = redact_line(s);
        // Must not panic; truncation should land on a char boundary.
        assert!(out.contains("…[truncated"));
        // The cut should have stepped back from byte 256 (mid-é) to 255.
        assert!(out.starts_with(&"a".repeat(MAX_LINE_FOR_DISPLAY - 1)));
    }
}
