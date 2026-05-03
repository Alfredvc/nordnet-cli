//! Public market-data feed client.

use crate::command::{encode_login_frame, encode_subscribe_frame, LoginCommand, SubscribeArgs};
use crate::error::{redact_line, FeedError};
use crate::event::{Envelope, PublicEvent};
use crate::transport::{self, Inner};

use nordnet_model::models::login::Feed;

/// Public market-data feed client. One connection per session, max.
///
/// All methods take `&mut self` — to run send and receive concurrently,
/// split externally with `tokio::io::split` plus `Arc<Mutex<...>>`. Not
/// provided by the crate.
///
/// # Termination semantics
///
/// Any error returned by [`Self::recv`], [`Self::send`-style methods],
/// or a clean EOF from `recv()` puts the client in a terminal state.
/// The transport is dropped and every subsequent call returns
/// [`FeedError::Closed`]. Callers must construct a new client (and
/// re-login) to continue.
pub struct PublicFeedClient {
    /// `Some(inner)` while live. Set to `None` on first error / EOF —
    /// every subsequent call returns [`FeedError::Closed`].
    inner: Option<Inner>,
}

impl PublicFeedClient {
    /// Connect to `feed.hostname:feed.port`. Performs a TLS handshake
    /// iff `feed.encrypted == true` (per design Decision §3 — honors
    /// the structured wire field instead of the Python reference impl's
    /// `port == 443` heuristic).
    pub async fn connect(feed: &Feed) -> Result<Self, FeedError> {
        Ok(Self {
            inner: Some(transport::connect(feed).await?),
        })
    }

    /// Fire-and-forget login (Decision §4). Writes the login frame and
    /// returns `Ok(())`. Server errors arrive via [`Self::recv`] as
    /// [`PublicEvent::Error`].
    ///
    /// To detect login failure deterministically, drain `recv()` until
    /// you see either `Error` or a non-`Heartbeat` event before calling
    /// [`Self::subscribe`] — see the spec §"Login command" for the
    /// recommended pattern.
    pub async fn login(&mut self, session_key: &str) -> Result<(), FeedError> {
        let frame = encode_login_frame(&LoginCommand { session_key }).map_err(FeedError::Encode)?;
        self.send_line(frame).await
    }

    /// Writes the subscribe frame. Successful return means the frame
    /// was *written*, NOT that the server accepted the subscription —
    /// rate-limit / unknown-instrument / unauthorized rejections arrive
    /// asynchronously as [`PublicEvent::Error`] frames.
    pub async fn subscribe(&mut self, args: SubscribeArgs) -> Result<(), FeedError> {
        let frame = encode_subscribe_frame("subscribe", &args).map_err(FeedError::Encode)?;
        self.send_line(frame).await
    }

    /// Mirror of [`Self::subscribe`] for stopping a feed. Pass the same
    /// `SubscribeArgs` value you used to subscribe (the type derives
    /// `Eq + Hash` so callers can stash it).
    pub async fn unsubscribe(&mut self, args: SubscribeArgs) -> Result<(), FeedError> {
        let frame = encode_subscribe_frame("unsubscribe", &args).map_err(FeedError::Encode)?;
        self.send_line(frame).await
    }

    /// Receive the next event.
    ///
    /// Returns `Ok(None)` on clean EOF (peer closed cleanly between
    /// frames). Returns `Err(FeedError::Closed)` if the peer hung up
    /// via abrupt RST mid-frame. Returns `Err(FeedError::Decode { .. })`
    /// if the peer sent a clean FIN with a partial frame buffered (the
    /// half-frame is delivered as a line and fails JSON parsing — the
    /// truncated line is attached for diagnostics).
    ///
    /// All error / EOF outcomes are terminal: the transport is dropped
    /// and every subsequent call returns [`FeedError::Closed`]. Stray
    /// blank lines on the wire (NDJSON keepalive convention) are
    /// skipped silently rather than producing a `Decode` error.
    pub async fn recv(&mut self) -> Result<Option<PublicEvent>, FeedError> {
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
        match PublicEvent::from_envelope(env) {
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
