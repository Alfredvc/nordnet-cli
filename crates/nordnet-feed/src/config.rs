//! Configuration knobs for [`crate::PublicFeedClient`] and
//! [`crate::PrivateFeedClient`].

use std::time::Duration;

/// Default budget shared between TCP connect and TLS handshake.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Default heartbeat-watchdog budget. Nordnet sends a server-to-client
/// heartbeat every 5s when idle (per spec); 15s = 3× margin. See
/// [Heartbeat Events](https://www.nordnet.se/externalapi/docs/feeds#heartbeat-events).
pub const DEFAULT_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);

/// Tunables applied at connect time and on every `recv()`.
///
/// Construct with [`FeedConfig::default`] for production-sane values, or
/// override fields explicitly. Pass to
/// [`crate::PublicFeedClient::connect_with`] /
/// [`crate::PrivateFeedClient::connect_with`].
#[derive(Debug, Clone)]
pub struct FeedConfig {
    /// Combined budget for TCP connect + (optional) TLS handshake. The
    /// budget covers BOTH phases via a single shared deadline.
    pub connect_timeout: Duration,
    /// Maximum time a `recv()` call will wait for the next frame before
    /// returning [`crate::FeedError::HeartbeatTimeout`]. Detects
    /// half-open connections (NAT timeouts, firewall drops, server
    /// hangs) that the kernel-level TCP keepalive has not yet flagged.
    /// Set to `None` to disable the watchdog.
    pub heartbeat_timeout: Option<Duration>,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            heartbeat_timeout: Some(DEFAULT_HEARTBEAT_TIMEOUT),
        }
    }
}
