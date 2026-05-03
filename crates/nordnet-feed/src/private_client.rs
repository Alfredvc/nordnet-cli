//! Private account/order feed client.

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_util::codec::{Framed, LinesCodec};

use crate::codec::new_lines_codec;
use crate::command::{encode_login_frame, LoginCommand};
use crate::error::FeedError;
use crate::event::{Envelope, PrivateEvent};
use crate::public_client::map_lines_err;

use nordnet_model::models::login::Feed;

/// Private feed client. Receives auto-pushed account events (orders +
/// fills) — there is no subscribe API; login implicitly enrolls the
/// session for all account events.
///
/// All methods take `&mut self`. To run send and receive concurrently,
/// split externally with `tokio::io::split` plus `Arc<Mutex<...>>`.
pub struct PrivateFeedClient {
    inner: Inner,
}

enum Inner {
    Plain(Framed<TcpStream, LinesCodec>),
    Tls(Box<Framed<TlsStream<TcpStream>, LinesCodec>>),
}

impl PrivateFeedClient {
    /// Connect to `feed.hostname:feed.port`. TLS handshake iff
    /// `feed.encrypted == true` (Decision §3).
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

    /// Fire-and-forget login (Decision §4). After this returns,
    /// account events start arriving via [`Self::recv`].
    ///
    /// To detect login failure deterministically, drain `recv()` until
    /// you see either `Error` or a non-`Heartbeat` event before relying
    /// on the account stream — see the spec §"Login command" for the
    /// recommended pattern.
    pub async fn login(&mut self, session_key: &str) -> Result<(), FeedError> {
        let frame = encode_login_frame(&LoginCommand { session_key }).map_err(FeedError::Encode)?;
        match &mut self.inner {
            Inner::Plain(f) => f.send(frame).await.map_err(map_lines_err),
            Inner::Tls(f) => f.send(frame).await.map_err(map_lines_err),
        }
    }

    /// Receive the next event.
    ///
    /// `Ok(None)` on clean EOF between frames. `Err(FeedError::Closed)`
    /// on abrupt RST mid-frame. `Err(FeedError::Decode { .. })` on a
    /// clean FIN with partial data (the half-frame is delivered as a
    /// line and fails JSON parsing). All terminal.
    pub async fn recv(&mut self) -> Result<Option<PrivateEvent>, FeedError> {
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
        let event = PrivateEvent::from_envelope(env)
            .map_err(|source| FeedError::Decode { source, line })?;
        Ok(Some(event))
    }
}
