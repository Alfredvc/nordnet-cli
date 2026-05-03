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
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use socket2::{SockRef, TcpKeepalive};
use tokio::net::TcpStream;
use tokio::time::{timeout_at, Instant};
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_util::codec::{Framed, LinesCodec, LinesCodecError};

use crate::codec::new_lines_codec;
use crate::error::FeedError;
use nordnet_model::models::login::Feed;

/// TCP keepalive idle time before the first probe. Sized for a long-lived
/// streaming feed: detect a half-open connection well before any common
/// NAT / firewall idle-timeout (typically 30–60 min) and well after a
/// burst of ticks could go quiet during a quiet market window.
const TCP_KEEPALIVE_IDLE: Duration = Duration::from_secs(30);
/// Interval between keepalive probes once the idle timer fires.
const TCP_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
/// Number of failed probes before the kernel declares the peer dead.
/// 30s idle + 3 × 10s interval ≈ dead-peer detection within ~60s
/// (independent of the application-layer heartbeat watchdog).
const TCP_KEEPALIVE_RETRIES: u32 = 3;

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

/// Connect to `feed.hostname:feed.port` within `connect_timeout`. The
/// budget covers BOTH the TCP handshake and the TLS handshake (when
/// `feed.encrypted == true`) — `timeout_at` shares one deadline across
/// the two phases.
///
/// TLS handshake iff `feed.encrypted == true` (Decision §3 — honors the
/// structured wire field instead of the Python reference impl's
/// `port == 443` heuristic).
///
/// On the resulting socket: `TCP_NODELAY` is enabled (low-latency
/// command writes) and TCP keepalive is configured (kernel-level
/// dead-peer detection at ~60s, independent of the application-layer
/// heartbeat watchdog).
pub(crate) async fn connect(feed: &Feed, connect_timeout: Duration) -> Result<Inner, FeedError> {
    let deadline = Instant::now() + connect_timeout;
    let addr = format!("{}:{}", feed.hostname, feed.port);

    let tcp = match timeout_at(deadline, TcpStream::connect(&addr)).await {
        Ok(r) => r?,
        Err(_) => return Err(FeedError::ConnectTimeout(connect_timeout)),
    };
    configure_tcp(&tcp)?;

    if feed.encrypted {
        let connector = TlsConnector::from(tls_config());
        let server_name = feed.hostname.clone().try_into().map_err(
            |e: rustls::pki_types::InvalidDnsNameError| {
                FeedError::Io(io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))
            },
        )?;
        let tls = match timeout_at(deadline, connector.connect(server_name, tcp)).await {
            Ok(r) => r.map_err(map_tls_connect_err)?,
            Err(_) => return Err(FeedError::ConnectTimeout(connect_timeout)),
        };
        Ok(Inner::Tls(Box::new(Framed::new(tls, new_lines_codec()))))
    } else {
        Ok(Inner::Plain(Framed::new(tcp, new_lines_codec())))
    }
}

/// Apply low-latency + dead-peer-detection socket options.
fn configure_tcp(tcp: &TcpStream) -> Result<(), FeedError> {
    tcp.set_nodelay(true)?;
    // Set every TcpKeepalive parameter we care about on a single call —
    // on Windows, `set_tcp_keepalive` resets unspecified parameters back
    // to OS defaults (per socket2 docs).
    let keepalive = TcpKeepalive::new()
        .with_time(TCP_KEEPALIVE_IDLE)
        .with_interval(TCP_KEEPALIVE_INTERVAL)
        .with_retries(TCP_KEEPALIVE_RETRIES);
    SockRef::from(tcp).set_tcp_keepalive(&keepalive)?;
    Ok(())
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
/// a TCP RST. If the type-check passes but the downcast somehow fails
/// (impossible under current `tokio_rustls`), surface a synthetic
/// `FeedError::Io` rather than panicking — keeps the production binary
/// crash-free under unforeseen library churn.
fn map_tls_connect_err(io: io::Error) -> FeedError {
    if io.get_ref().is_some_and(|e| e.is::<rustls::Error>()) {
        match io.into_inner().and_then(|e| e.downcast::<rustls::Error>().ok()) {
            Some(rustls_err) => FeedError::Tls(*rustls_err),
            None => FeedError::Io(io::Error::other(
                "rustls error type-erased after type check",
            )),
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
