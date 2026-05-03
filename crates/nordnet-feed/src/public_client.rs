//! Public market-data feed client.
//!
//! [`PublicFeedClient`] owns one TLS connection to
//! `public.feed.nordnet.se`. The interaction model is:
//!
//! 1. [`PublicFeedClient::connect`] — open TCP+TLS, configure socket
//!    options (NODELAY, keepalive), apply connect timeout.
//! 2. [`PublicFeedClient::login`] — fire-and-forget; failure surfaces
//!    asynchronously via [`crate::PublicEvent::Error`] in the recv loop.
//! 3. [`PublicFeedClient::subscribe`] — one call per stream of interest.
//!    See the [command module](crate::command) for the subscribe →
//!    event mapping.
//! 4. [`PublicFeedClient::recv`] in a loop — typed events arrive here,
//!    including heartbeats, server errors, and `DecodeFailed` soft
//!    failures (non-terminal).
//!
//! Any [`crate::FeedError`] from any method is terminal; the client
//! transitions to `Closed` and a new client is required to continue.

use std::time::Duration;

use tokio::time::timeout;

use crate::command::{encode_login_frame, encode_subscribe_frame, LoginCommand, SubscribeArgs};
use crate::config::FeedConfig;
use crate::error::{redact_line, FeedError};
use crate::event::{Envelope, PublicEvent};
use crate::transport::{self, Inner};

use nordnet_model::auth::Session;
use nordnet_model::models::login::Feed;

/// Public market-data feed client. One connection per session, max.
///
/// All methods take `&mut self` — to run send and receive concurrently,
/// split externally with `tokio::io::split` plus `Arc<Mutex<...>>`. Not
/// provided by the crate.
///
/// # Termination semantics
///
/// Any error returned by [`Self::recv`], [`Self::login`],
/// [`Self::subscribe`], or [`Self::unsubscribe`], or a clean EOF from
/// `recv()`, puts the client in a terminal state. The transport is
/// dropped and every subsequent call returns [`FeedError::Closed`].
/// Callers must construct a new client (and re-login) to continue.
///
/// Per-frame payload type mismatches do NOT terminate — they surface
/// as [`PublicEvent::DecodeFailed`] events and the connection stays
/// open for the next frame.
///
/// # Example
///
/// ```no_run
/// use nordnet_feed::{PublicEvent, PublicFeedClient, MarketDataKind, SubscribeArgs};
/// use nordnet_model::auth::Session;
/// use nordnet_model::models::login::Feed;
///
/// # async fn run(feed: Feed, session: Session) -> Result<(), nordnet_feed::FeedError> {
/// let mut client = PublicFeedClient::connect(&feed).await?;
/// client.login(&session).await?;
///
/// // Drain pre-subscribe events until login is confirmed (or rejected).
/// loop {
///     match client.recv().await? {
///         Some(PublicEvent::Heartbeat) => continue,
///         Some(PublicEvent::Error(e)) => return Err(nordnet_feed::FeedError::Io(
///             std::io::Error::other(e.msg))),
///         Some(_) | None => break,
///     }
/// }
///
/// client.subscribe(SubscribeArgs::MarketData {
///     kind: MarketDataKind::Price,
///     market: 11,
///     identifier: "101".into(),
/// }).await?;
/// # Ok(()) }
/// ```
pub struct PublicFeedClient {
    /// `Some(inner)` while live. Set to `None` on first error / EOF —
    /// every subsequent call returns [`FeedError::Closed`].
    inner: Option<Inner>,
    heartbeat_timeout: Option<Duration>,
}

impl PublicFeedClient {
    /// Connect using [`FeedConfig::default`] tunables.
    ///
    /// # Errors
    ///
    /// - [`FeedError::ConnectTimeout`] — TCP+TLS handshake exceeded the
    ///   default 10s budget.
    /// - [`FeedError::Tls`] — TLS handshake / certificate failure.
    /// - [`FeedError::Io`] — raw socket / network failure.
    pub async fn connect(feed: &Feed) -> Result<Self, FeedError> {
        Self::connect_with(feed, &FeedConfig::default()).await
    }

    /// Connect with explicit tunables.
    ///
    /// `config.connect_timeout` bounds combined TCP + TLS handshake
    /// time. `config.heartbeat_timeout` is applied to every subsequent
    /// [`Self::recv`] call.
    ///
    /// # Errors
    ///
    /// - [`FeedError::ConnectTimeout`] — combined TCP+TLS handshake
    ///   exceeded `config.connect_timeout`.
    /// - [`FeedError::Tls`] — TLS handshake / certificate failure.
    /// - [`FeedError::Io`] — raw socket / network failure.
    pub async fn connect_with(feed: &Feed, config: &FeedConfig) -> Result<Self, FeedError> {
        let inner = transport::connect(feed, config.connect_timeout).await?;
        Ok(Self {
            inner: Some(inner),
            heartbeat_timeout: config.heartbeat_timeout,
        })
    }

    /// Fire-and-forget login (Decision §4). Writes the login frame and
    /// returns `Ok(())`. Server errors arrive via [`Self::recv`] as
    /// [`PublicEvent::Error`].
    ///
    /// To detect login failure deterministically, drain `recv()` until
    /// you see either `Error` or a non-`Heartbeat` event before calling
    /// [`Self::subscribe`] — see the type-level example.
    ///
    /// # Errors
    ///
    /// - [`FeedError::Closed`] — client is already terminal.
    /// - [`FeedError::Encode`] — JSON serialization of the login frame
    ///   failed (should not happen in practice).
    /// - [`FeedError::Io`] / [`FeedError::FrameTooLarge`] — write-side
    ///   transport failure.
    pub async fn login(&mut self, session: &Session) -> Result<(), FeedError> {
        let frame = encode_login_frame(&LoginCommand {
            session_key: &session.session_key,
        })
        .map_err(FeedError::Encode)?;
        self.send_line(frame).await
    }

    /// Writes the subscribe frame. Successful return means the frame
    /// was *written*, NOT that the server accepted the subscription —
    /// rate-limit / unknown-instrument / unauthorized rejections arrive
    /// asynchronously as [`PublicEvent::Error`] frames.
    ///
    /// See the [command module mapping table](crate::command#subscribe--event-mapping)
    /// for which event variant each `SubscribeArgs` produces.
    ///
    /// # Errors
    ///
    /// - [`FeedError::Closed`] — client is already terminal.
    /// - [`FeedError::Encode`] — JSON serialization failed.
    /// - [`FeedError::Io`] / [`FeedError::FrameTooLarge`] — write-side
    ///   transport failure.
    #[doc(alias = "listen")]
    #[doc(alias = "watch")]
    pub async fn subscribe(&mut self, args: SubscribeArgs) -> Result<(), FeedError> {
        let frame = encode_subscribe_frame("subscribe", &args).map_err(FeedError::Encode)?;
        self.send_line(frame).await
    }

    /// Mirror of [`Self::subscribe`] for stopping a feed. Pass the same
    /// `SubscribeArgs` value you used to subscribe (the type derives
    /// `Eq + Hash` so callers can stash it).
    ///
    /// # Errors
    ///
    /// Same shape as [`Self::subscribe`].
    pub async fn unsubscribe(&mut self, args: SubscribeArgs) -> Result<(), FeedError> {
        let frame = encode_subscribe_frame("unsubscribe", &args).map_err(FeedError::Encode)?;
        self.send_line(frame).await
    }

    /// Receive the next event.
    ///
    /// Returns `Ok(None)` on clean EOF (peer closed cleanly between
    /// frames). Per-frame payload type mismatches surface as
    /// [`PublicEvent::DecodeFailed`] — non-terminal — so a single bad
    /// payload does not kill the connection. Stray blank lines on the
    /// wire (NDJSON keepalive convention) are skipped silently.
    ///
    /// All `Err(..)` and `Ok(None)` outcomes are terminal: the
    /// transport is dropped and every subsequent call returns
    /// [`FeedError::Closed`].
    ///
    /// # Errors
    ///
    /// - [`FeedError::Closed`] — client was already terminal, or peer
    ///   hung up via abrupt RST mid-frame.
    /// - [`FeedError::Decode`] — envelope JSON is malformed (terminal —
    ///   fundamentally broken stream). Per-payload mismatches go to
    ///   [`PublicEvent::DecodeFailed`] instead.
    /// - [`FeedError::HeartbeatTimeout`] — no frame within
    ///   `config.heartbeat_timeout`.
    /// - [`FeedError::FrameTooLarge`] — frame exceeded 1 MiB.
    /// - [`FeedError::Io`] — read-side transport failure.
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
        Ok(Some(PublicEvent::from_envelope(env)))
    }

    /// Read the next non-empty line from the wire. Empty lines (stray
    /// `\n\n` keepalives, peer-flush artifacts) are skipped per NDJSON
    /// convention. Sets `self.inner = None` on EOF, transport error, or
    /// heartbeat-watchdog timeout.
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
