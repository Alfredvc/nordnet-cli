//! Shared TCP / TLS transport for both feed clients.
//!
//! Holds the `Inner` framed-stream enum, the cached `Arc<ClientConfig>`
//! used for every TLS connect, and the `LinesCodecError` → `FeedError`
//! mapping. Both [`crate::PublicFeedClient`] and
//! [`crate::PrivateFeedClient`] share this code path so behavior stays
//! in lockstep.

use std::io;
use std::sync::Arc;
use std::sync::OnceLock;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

use crate::codec::new_lines_codec;
use crate::error::FeedError;
use nordnet_model::models::login::Feed;

/// Framed transport — either plain TCP or TLS-over-TCP.
pub(crate) enum Inner {
    Plain(Framed<TcpStream, LinesCodec>),
    Tls(Box<Framed<TlsStream<TcpStream>, LinesCodec>>),
}

impl Inner {
    pub(crate) async fn next_line(&mut self) -> Option<Result<String, LinesCodecError>> {
        match self {
            Inner::Plain(f) => f.next().await,
            Inner::Tls(f) => f.next().await,
        }
    }

    pub(crate) async fn send_line(&mut self, line: String) -> Result<(), LinesCodecError> {
        match self {
            Inner::Plain(f) => f.send(line).await,
            Inner::Tls(f) => f.send(line).await,
        }
    }
}

/// Connect to `feed.hostname:feed.port`. TLS handshake iff
/// `feed.encrypted == true` (Decision §3 — honors the structured wire
/// field instead of the Python reference impl's `port == 443` heuristic).
pub(crate) async fn connect(feed: &Feed) -> Result<Inner, FeedError> {
    let addr = format!("{}:{}", feed.hostname, feed.port);
    let tcp = TcpStream::connect(&addr).await?;
    if feed.encrypted {
        let connector = TlsConnector::from(tls_config());
        let server_name = feed.hostname.clone().try_into().map_err(
            |e: rustls::pki_types::InvalidDnsNameError| {
                FeedError::Io(io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))
            },
        )?;
        let tls = connector
            .connect(server_name, tcp)
            .await
            .map_err(map_tls_connect_err)?;
        Ok(Inner::Tls(Box::new(Framed::new(tls, new_lines_codec()))))
    } else {
        Ok(Inner::Plain(Framed::new(tcp, new_lines_codec())))
    }
}

/// Cached `rustls::ClientConfig` — built once per process. Avoids paying
/// the ~150-cert webpki-roots clone on every reconnect.
fn tls_config() -> Arc<rustls::ClientConfig> {
    static TLS_CONFIG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    TLS_CONFIG
        .get_or_init(|| {
            let mut roots = rustls::RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            Arc::new(
                rustls::ClientConfig::builder()
                    .with_root_certificates(roots)
                    .with_no_client_auth(),
            )
        })
        .clone()
}

/// `tokio_rustls` wraps `rustls::Error` inside `io::Error` (handshake
/// failures, cert validation errors, peer-cert errors). Unwrap so they
/// surface as [`FeedError::Tls`] instead of being indistinguishable from
/// a TCP RST.
fn map_tls_connect_err(io: io::Error) -> FeedError {
    let is_rustls = io.get_ref().is_some_and(|e| e.is::<rustls::Error>());
    if is_rustls {
        let inner = io.into_inner().expect("get_ref returned Some");
        match inner.downcast::<rustls::Error>() {
            Ok(rustls_err) => FeedError::Tls(*rustls_err),
            Err(_) => unreachable!("checked is::<rustls::Error>()"),
        }
    } else {
        FeedError::Io(io)
    }
}

/// Map `LinesCodec` errors into our error taxonomy.
///
/// `LinesCodec::Io` is plain I/O — `UnexpectedEof` mid-frame is mapped
/// to [`FeedError::Closed`]. `LinesCodec::MaxLineLengthExceeded` maps to
/// [`FeedError::FrameTooLarge`].
pub(crate) fn map_lines_err(e: LinesCodecError) -> FeedError {
    match e {
        LinesCodecError::Io(io) => {
            if io.kind() == io::ErrorKind::UnexpectedEof {
                FeedError::Closed
            } else {
                FeedError::Io(io)
            }
        }
        LinesCodecError::MaxLineLengthExceeded => FeedError::FrameTooLarge,
    }
}
