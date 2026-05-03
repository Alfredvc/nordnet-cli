#![doc = include_str!("../README.md")]
//!
//! # Client lifecycle
//!
//! Both [`PublicFeedClient`] and [`PrivateFeedClient`] follow the same
//! state progression:
//!
//! ```text
//!   connect ──► login ──► (subscribe)* ──► recv loop ──► Closed
//!                                            │
//!                                            └── any FeedError ──┘
//! ```
//!
//! - **`connect` / `connect_with`** opens TCP+TLS, configures kernel
//!   keepalive (~60s dead-peer detection) and `TCP_NODELAY`, and
//!   applies the connect-timeout budget.
//! - **`login`** is fire-and-forget — see Decision §4. The call returns
//!   as soon as the login frame is written; the server's accept /
//!   reject answer arrives asynchronously as a [`PublicEvent::Error`] /
//!   [`PrivateEvent::Error`] event in the recv loop.
//! - **`subscribe` / `unsubscribe`** are public-feed only. The private
//!   feed has no subscribe API — login implicitly enrolls the session
//!   for all account events. See [`PublicFeedClient::subscribe`] for
//!   the public mapping; the [command module](crate::command#subscribe--event-mapping)
//!   has the full table.
//! - **`recv`** drives the typed event stream. Heartbeats arrive every
//!   5s when idle (per Nordnet); the watchdog (default 15s) detects
//!   half-open connections that kernel keepalive has not yet flagged.
//!
//! ## Terminal state and reconnection
//!
//! Any [`FeedError`] returned by any method, or `Ok(None)` from
//! [`PublicFeedClient::recv`] / [`PrivateFeedClient::recv`], puts the
//! client in a terminal `Closed` state. The transport is dropped and
//! every subsequent call returns [`FeedError::Closed`]. There is no
//! reconnect API: callers construct a fresh client and re-login. This
//! is deliberate — reconnection policy (backoff, retry budgets,
//! authentication-token refresh via [`nordnet_model::auth::Session`])
//! is application-specific and lives outside the crate.
//!
//! Per-frame payload mismatches are **not** terminal: they surface as
//! [`PublicEvent::DecodeFailed`] / [`PrivateEvent::DecodeFailed`] and
//! the connection stays open. Only envelope-level JSON failure
//! ([`FeedError::Decode`]) terminates.

mod codec;
pub mod command;
mod config;
mod error;
pub mod event;
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
