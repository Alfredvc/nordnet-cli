//! Private account/order feed client.

use std::time::Duration;

use tokio::time::timeout;

use crate::command::{encode_login_frame, LoginCommand};
use crate::config::FeedConfig;
use crate::error::{redact_line, FeedError};
use crate::event::{Envelope, PrivateEvent};
use crate::transport::{self, Inner};

use nordnet_model::auth::Session;
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
///
/// Per-frame payload type mismatches do NOT terminate — they surface
/// as [`PrivateEvent::DecodeFailed`] events and the connection stays
/// open for the next frame.
///
/// # Example
///
/// ```no_run
/// use nordnet_feed::{PrivateEvent, PrivateFeedClient};
/// use nordnet_model::auth::Session;
/// use nordnet_model::models::login::Feed;
///
/// # async fn run(feed: Feed, session: Session) -> Result<(), nordnet_feed::FeedError> {
/// let mut client = PrivateFeedClient::connect(&feed).await?;
/// client.login(&session).await?;
///
/// while let Some(event) = client.recv().await? {
///     match event {
///         PrivateEvent::Order(o) => println!("order {}", o.order_id),
///         PrivateEvent::Heartbeat => continue,
///         _ => {}
///     }
/// }
/// # Ok(()) }
/// ```
pub struct PrivateFeedClient {
    /// `Some(inner)` while live. Set to `None` on first error / EOF —
    /// every subsequent call returns [`FeedError::Closed`].
    inner: Option<Inner>,
    heartbeat_timeout: Option<Duration>,
}

impl PrivateFeedClient {
    /// Connect using [`FeedConfig::default`] tunables.
    pub async fn connect(feed: &Feed) -> Result<Self, FeedError> {
        Self::connect_with(feed, &FeedConfig::default()).await
    }

    /// Connect with explicit tunables (see [`FeedConfig`]).
    pub async fn connect_with(feed: &Feed, config: &FeedConfig) -> Result<Self, FeedError> {
        let inner = transport::connect(feed, config.connect_timeout).await?;
        Ok(Self {
            inner: Some(inner),
            heartbeat_timeout: config.heartbeat_timeout,
        })
    }

    /// Fire-and-forget login (Decision §4). After this returns,
    /// account events start arriving via [`Self::recv`].
    ///
    /// To detect login failure deterministically, drain `recv()` until
    /// you see either `Error` or a non-`Heartbeat` event before relying
    /// on the account stream.
    pub async fn login(&mut self, session: &Session) -> Result<(), FeedError> {
        let frame = encode_login_frame(&LoginCommand {
            session_key: &session.session_key,
        })
        .map_err(FeedError::Encode)?;
        self.send_line(frame).await
    }

    /// Receive the next event.
    ///
    /// `Ok(None)` on clean EOF between frames. `Err(FeedError::Closed)`
    /// on abrupt RST mid-frame. `Err(FeedError::Decode { .. })` on
    /// malformed envelope JSON (terminal — fundamentally broken
    /// stream). `Err(FeedError::HeartbeatTimeout(..))` if no frame
    /// arrived within the configured budget.
    ///
    /// Per-frame payload type mismatches surface as
    /// [`PrivateEvent::DecodeFailed`] — non-terminal — so a single bad
    /// payload does not kill the connection.
    ///
    /// All `Err(..)` and `Ok(None)` outcomes are terminal: the
    /// transport is dropped and every subsequent call returns
    /// [`FeedError::Closed`]. Stray blank lines are skipped silently.
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
        Ok(Some(PrivateEvent::from_envelope(env)))
    }

    /// Read the next non-empty line from the wire. Empty lines are
    /// skipped per NDJSON convention. Sets `self.inner = None` on EOF,
    /// transport error, or heartbeat-watchdog timeout.
    async fn recv_line(&mut self) -> Result<Option<String>, FeedError> {
        let watchdog = self.heartbeat_timeout;
        loop {
            let inner = self.inner.as_mut().ok_or(FeedError::Closed)?;
            let read = match watchdog {
                Some(t) => match timeout(t, inner.next_line()).await {
                    Ok(r) => r,
                    Err(_) => {
                        self.inner = None;
                        return Err(FeedError::HeartbeatTimeout(t));
                    }
                },
                None => inner.next_line().await,
            };
            match read {
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
