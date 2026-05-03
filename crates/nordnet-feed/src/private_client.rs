//! Private account/order feed client.

use crate::command::{encode_login_frame, LoginCommand};
use crate::error::{redact_line, FeedError};
use crate::event::{Envelope, PrivateEvent};
use crate::transport::{self, Inner};

use nordnet_model::models::login::Feed;

/// Private feed client. Receives auto-pushed account events (orders +
/// fills) — there is no subscribe API; login implicitly enrolls the
/// session for all account events.
///
/// All methods take `&mut self`. To run send and receive concurrently,
/// split externally with `tokio::io::split` plus `Arc<Mutex<...>>`.
///
/// # Termination semantics
///
/// Any error returned by [`Self::recv`] / [`Self::login`], or a clean
/// EOF from `recv()`, puts the client in a terminal state. The
/// transport is dropped and every subsequent call returns
/// [`FeedError::Closed`]. Callers must construct a new client to
/// continue.
pub struct PrivateFeedClient {
    /// `Some(inner)` while live. Set to `None` on first error / EOF —
    /// every subsequent call returns [`FeedError::Closed`].
    inner: Option<Inner>,
}

impl PrivateFeedClient {
    /// Connect to `feed.hostname:feed.port`. TLS handshake iff
    /// `feed.encrypted == true` (Decision §3).
    pub async fn connect(feed: &Feed) -> Result<Self, FeedError> {
        Ok(Self {
            inner: Some(transport::connect(feed).await?),
        })
    }

    /// Fire-and-forget login (Decision §4). After this returns,
    /// account events start arriving via [`Self::recv`].
    ///
    /// To detect login failure deterministically, drain `recv()` until
    /// you see either `Error` or a non-`Heartbeat` event before relying
    /// on the account stream — see the spec §"Login command" for the
    /// recommended pattern.
    pub async fn login(&mut self, session_key: &str) -> Result<(), FeedError> {
        let frame = encode_login_frame(&LoginCommand { session_key }).map_err(FeedError::Encode)?;
        self.send_line(frame).await
    }

    /// Receive the next event.
    ///
    /// `Ok(None)` on clean EOF between frames. `Err(FeedError::Closed)`
    /// on abrupt RST mid-frame. `Err(FeedError::Decode { .. })` on a
    /// clean FIN with partial data (the half-frame is delivered as a
    /// line and fails JSON parsing — the truncated line is attached
    /// for diagnostics).
    ///
    /// All error / EOF outcomes are terminal: the transport is dropped
    /// and every subsequent call returns [`FeedError::Closed`]. Stray
    /// blank lines on the wire (NDJSON keepalive convention) are
    /// skipped silently rather than producing a `Decode` error.
    pub async fn recv(&mut self) -> Result<Option<PrivateEvent>, FeedError> {
        let line = match self.recv_line().await? {
            None => return Ok(None),
            Some(s) => s,
        };
        let env: Envelope = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(source) => {
                self.inner = None;
                return Err(FeedError::Decode {
                    source,
                    line: redact_line(line),
                });
            }
        };
        match PrivateEvent::from_envelope(env) {
            Ok(e) => Ok(Some(e)),
            Err(source) => {
                self.inner = None;
                Err(FeedError::Decode {
                    source,
                    line: redact_line(line),
                })
            }
        }
    }

    /// Read the next non-empty line from the wire. Empty lines (stray
    /// `\n\n` keepalives, peer-flush artifacts) are skipped per NDJSON
    /// convention. Sets `self.inner = None` on EOF or transport error.
    async fn recv_line(&mut self) -> Result<Option<String>, FeedError> {
        if self.inner.is_none() {
            return Err(FeedError::Closed);
        }
        loop {
            let inner = self.inner.as_mut().expect("checked Some above");
            match inner.next_line().await {
                None => {
                    self.inner = None;
                    return Ok(None);
                }
                Some(Err(e)) => {
                    self.inner = None;
                    return Err(transport::map_lines_err(e));
                }
                Some(Ok(s)) if s.is_empty() => continue,
                Some(Ok(s)) => return Ok(Some(s)),
            }
        }
    }

    async fn send_line(&mut self, line: String) -> Result<(), FeedError> {
        let inner = self.inner.as_mut().ok_or(FeedError::Closed)?;
        match inner.send_line(line).await {
            Ok(()) => Ok(()),
            Err(e) => {
                self.inner = None;
                Err(transport::map_lines_err(e))
            }
        }
    }
}
