//! Streaming feeds for the Nordnet External API v2.
//!
//! Sibling to `nordnet-api`. Both crates share `nordnet-model` types
//! but have independent transports — no reqwest here, no tokio there.
//!
//! Two feed types: [`PublicFeedClient`] for market data subscriptions,
//! [`PrivateFeedClient`] for account/order events (auto-pushed after
//! login). See the design doc for protocol details.

pub mod codec;
pub mod command;
pub mod error;
pub mod event;
pub mod private;
pub mod private_client;
pub mod public;
pub mod public_client;
mod transport;

pub use command::{MarketDataKind, SubscribeArgs};
pub use error::{FeedError, ServerError};
pub use event::{PrivateEvent, PublicEvent};
pub use private_client::PrivateFeedClient;
pub use public_client::PublicFeedClient;
