//! Streaming feeds for the Nordnet External API v2.
//!
//! Sibling to `nordnet-api`. Both crates share `nordnet-model` types
//! but have independent transports — no `reqwest` here.
//!
//! Two feed types: [`PublicFeedClient`] for market data subscriptions,
//! [`PrivateFeedClient`] for account/order events (auto-pushed after
//! login).
//!
//! # Production hardening
//!
//! - TCP `SO_KEEPALIVE` is configured at connect time (kernel-level
//!   dead-peer detection ~60s).
//! - `TCP_NODELAY` is enabled (low-latency command writes).
//! - Connect timeout (default 10s) bounds combined TCP + TLS handshake
//!   time — see [`FeedConfig::connect_timeout`].
//! - A heartbeat watchdog (default 15s) detects half-open connections
//!   that survive the kernel-level keepalive — see
//!   [`FeedConfig::heartbeat_timeout`].
//!
//! Override defaults with [`FeedConfig`] + `connect_with`.

mod codec;
mod command;
mod config;
mod error;
mod event;
pub mod private;
mod private_client;
pub mod public;
mod public_client;
mod transport;

pub use codec::MAX_FRAME_BYTES;
pub use command::{MarketDataKind, SubscribeArgs};
pub use config::{FeedConfig, DEFAULT_CONNECT_TIMEOUT, DEFAULT_HEARTBEAT_TIMEOUT};
pub use error::{FeedError, ServerError};
pub use event::{PrivateEvent, PublicEvent};
pub use private_client::PrivateFeedClient;
pub use public_client::PublicFeedClient;

/// Re-export of [`serde_json::Value`] — the carry type for `Unknown`
/// and `DecodeFailed` event variants and `ServerError::cmd`. Re-exported
/// so consumers can inspect these fields without depending on
/// `serde_json` directly.
pub use serde_json::Value;
