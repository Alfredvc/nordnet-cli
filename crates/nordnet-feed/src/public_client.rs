//! Public market-data feed client.

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_util::codec::Framed;
use tokio_util::codec::LinesCodec;

use crate::codec::new_lines_codec;
use crate::command::{encode_login_frame, encode_subscribe_frame, LoginCommand, SubscribeArgs};
use crate::error::FeedError;
use crate::event::{Envelope, PublicEvent};

use nordnet_model::models::login::Feed;

/// Public market-data feed client. One connection per session, max.
///
/// All methods take `&mut self` — to run send and receive concurrently,
/// split externally with `tokio::io::split` plus `Arc<Mutex<...>>`. Not
/// provided by the crate.
pub struct PublicFeedClient {
    inner: Inner,
}

enum Inner {
    Plain(Framed<TcpStream, LinesCodec>),
    Tls(Box<Framed<TlsStream<TcpStream>, LinesCodec>>),
}

impl PublicFeedClient {
    /// Connect to `feed.hostname:feed.port`. Performs a TLS handshake
    /// iff `feed.encrypted == true` (per design Decision §3 — honors
    /// the structured wire field instead of the Python reference impl's
    /// `port == 443` heuristic).
    pub async fn connect(feed: &Feed) -> Result<Self, FeedError> {
        let addr = format!("{}:{}", feed.hostname, feed.port);
        let tcp = TcpStream::connect(&addr).await?;
        let inner = if feed.encrypted {
            let mut roots = rustls::RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            let cfg = rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();
            let connector = TlsConnector::from(Arc::new(cfg));
            let server_name = feed.hostname.clone().try_into().map_err(
                |e: rustls::pki_types::InvalidDnsNameError| {
                    FeedError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        e.to_string(),
                    ))
                },
            )?;
            let tls = connector.connect(server_name, tcp).await?;
            Inner::Tls(Box::new(Framed::new(tls, new_lines_codec())))
        } else {
            Inner::Plain(Framed::new(tcp, new_lines_codec()))
        };
        Ok(Self { inner })
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

    /// Receive the next event. Returns `Ok(None)` on clean EOF (peer
    /// closed cleanly between frames). Returns `Err(FeedError::Closed)`
    /// if the peer closed mid-frame.
    pub async fn recv(&mut self) -> Result<Option<PublicEvent>, FeedError> {
        let line = match &mut self.inner {
            Inner::Plain(f) => f.next().await,
            Inner::Tls(f) => f.next().await,
        };
        let line = match line {
            None => return Ok(None),
            Some(Err(e)) => return Err(map_lines_err(e)),
            Some(Ok(s)) => s,
        };
        let env: Envelope = serde_json::from_str(&line).map_err(|source| FeedError::Decode {
            source,
            line: line.clone(),
        })?;
        let event =
            PublicEvent::from_envelope(env).map_err(|source| FeedError::Decode { source, line })?;
        Ok(Some(event))
    }

    async fn send_line(&mut self, line: String) -> Result<(), FeedError> {
        match &mut self.inner {
            Inner::Plain(f) => f.send(line).await.map_err(map_lines_err),
            Inner::Tls(f) => f.send(line).await.map_err(map_lines_err),
        }
    }
}

/// Map `LinesCodec` errors into our error taxonomy.
///
/// `LinesCodec::Io` is plain I/O. `LinesCodec::MaxLineLengthExceeded`
/// is the 1 MiB cap (we don't expose a counter — log the error if you
/// need byte counts).
pub(crate) fn map_lines_err(e: tokio_util::codec::LinesCodecError) -> FeedError {
    use tokio_util::codec::LinesCodecError;
    match e {
        LinesCodecError::Io(io) => {
            // EOF mid-frame surfaces as UnexpectedEof. Spec wants this
            // mapped to Closed.
            if io.kind() == std::io::ErrorKind::UnexpectedEof {
                FeedError::Closed
            } else {
                FeedError::Io(io)
            }
        }
        LinesCodecError::MaxLineLengthExceeded => FeedError::FrameTooLarge {
            bytes: crate::codec::MAX_FRAME_BYTES + 1,
        },
    }
}
