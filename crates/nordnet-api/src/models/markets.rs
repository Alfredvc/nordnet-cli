//! Models for the `markets` resource group.
//!
//! Derived from `docs-extract/_definitions/Market.md`.
//!
//! The `Market` definition documents three fields:
//!
//! - `country` — optional string (ISO country code; available for all
//!   non-virtual markets).
//! - `market_id` — required `integer(int64)`. Modelled with the
//!   [`MarketId`] newtype per CONTRACTS.md.
//! - `name` — required string (the market name).

use crate::ids::MarketId;
use serde::{Deserialize, Serialize};

/// A market entry as returned by `GET /markets` and
/// `GET /markets/{market_id}`.
///
/// Schema source: `docs-extract/_definitions/Market.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Market {
    /// ISO country code (e.g. `"SE"`, `"NO"`). Optional per the schema —
    /// available for all non-virtual markets.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub country: Option<String>,
    /// The Nordnet unique market identifier.
    pub market_id: MarketId,
    /// The market name (translated per the `Accept-Language` request header).
    pub name: String,
}
