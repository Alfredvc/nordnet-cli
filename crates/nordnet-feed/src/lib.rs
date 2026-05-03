#![doc = include_str!("../README.md")]

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
