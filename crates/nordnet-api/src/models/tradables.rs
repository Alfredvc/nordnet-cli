//! Models for the `tradables` resource group.
//!
//! Derived strictly from these schema files in `docs-extract/_definitions/`:
//!
//! - `TradableInfo.md`
//! - `TradablePublicTrades.md`
//! - `TradableEligibility.md`
//! - `PublicTrade.md`
//! - `CalendarDay.md`
//! - `OrderType.md`
//!
//! Per CONTRACTS.md, every referenced type is defined locally here. Cross-group
//! deduplication (e.g. with the structurally similar `instruments` group types)
//! is deferred to Phase 3X.
//!
//! ## Doc notes (for Phase 3X reconciliation)
//!
//! - `TradableEligibility.market_id` is documented as `integer(int32)` while
//!   every other `market_id` in the API is `integer(int64)`. We keep the
//!   uniform [`MarketId`] newtype (which is `i64`) and flag the asymmetry
//!   here. Phase 3X may either widen the docs upstream or introduce a
//!   narrower newtype.
//! - `CalendarDay.date` is `string(date)` (YYYY-MM-DD). It is kept as a
//!   plain `String` here — wiring `time::Date` would require a custom serde
//!   adapter, which is out of scope for the typed binding's first pass.
//!   Phase 3X may introduce a strongly-typed wrapper.
//! - `CalendarDay.open` / `CalendarDay.close` and
//!   `PublicTrade.tick_timestamp` / `PublicTrade.trade_timestamp` are
//!   `integer(int64)` UNIX-millisecond epoch timestamps. They are kept as
//!   plain `i64` (no `EpochMillis` newtype exists under
//!   `crate::models::shared`).
//! - `PublicTrade.price` is `number(double)`. It is typed as
//!   [`rust_decimal::Decimal`] (with the `arbitrary_precision` adapter)
//!   per CONTRACTS.md — never `f64`. Because of this `PublicTrade` and
//!   `TradablePublicTrades` cannot derive [`Eq`].

use crate::ids::{MarketId, TradableId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Tradable lookup key: `[market_id]:[identifier]` (e.g. `11:101` for ERIC B).
///
/// Constructed by callers and passed to the resource methods on
/// [`crate::Client`]. The wire form (`{market_id}:{identifier}`) is produced
/// by the [`std::fmt::Display`] impl.
///
/// Multi-key lookups (the API accepts a comma-separated list in the path)
/// are not modelled here — Phase 4 is expected to add a small helper for
/// that shape so the typed API stays single-key by default.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TradableKey {
    /// Market identifier component of the key.
    pub market_id: MarketId,
    /// Tradable identifier component of the key.
    pub identifier: TradableId,
}

impl TradableKey {
    /// Construct a new [`TradableKey`] from its two components.
    pub fn new(market_id: MarketId, identifier: TradableId) -> Self {
        Self {
            market_id,
            identifier,
        }
    }
}

impl std::fmt::Display for TradableKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.market_id.0, self.identifier.0)
    }
}

/// One trading-calendar day for a tradable.
///
/// Schema: `_definitions/CalendarDay.md`. All fields are required.
///
/// `open` and `close` are UNIX-millisecond epoch timestamps (see module
/// doc note); `date` is a `YYYY-MM-DD` string (see module doc note).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CalendarDay {
    /// The market closing time. UNIX timestamp in milliseconds.
    pub close: i64,
    /// The date formatted as `YYYY-MM-DD`.
    pub date: String,
    /// The market opening time. UNIX timestamp in milliseconds.
    pub open: i64,
}

/// One allowed order type for a tradable: a `(name, type)` pair where
/// `name` is the localized label and `type` is the wire code (e.g.
/// `LIMIT`, `STOP_LIMIT`).
///
/// Schema: `_definitions/OrderType.md`. Both fields are required.
///
/// Renamed from `OrderType` to `AllowedOrderType` to disambiguate from
/// [`crate::models::orders::OrderType`], which is the closed enum used
/// on the request side of `place_order`. Each is a different concept —
/// the tradable's allowed-set is a per-instrument capability discovered
/// at runtime; the request enum is the value the caller sends.
///
/// The wire field `type` is a Rust keyword — exposed as `r#type` with
/// `#[serde(rename = "type")]`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AllowedOrderType {
    /// The translated order type.
    pub name: String,
    /// The order type code. Renamed to `r#type` because `type` is a Rust
    /// keyword; the raw wire name is preserved via `#[serde(rename)]`.
    #[serde(rename = "type")]
    pub r#type: String,
}

/// Trading calendar and allowed trading types for a single tradable.
///
/// Schema: `_definitions/TradableInfo.md`. All fields are required.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TradableInfo {
    /// Allowed days for long term orders.
    pub calendar: Vec<CalendarDay>,
    /// `true` if iceberg orders are allowed.
    pub iceberg: bool,
    /// The Nordnet tradable identifier. The combination of market ID and
    /// tradable ID is unique.
    pub identifier: TradableId,
    /// The Nordnet unique market identifier.
    pub market_id: MarketId,
    /// Allowed order types.
    pub order_types: Vec<AllowedOrderType>,
}

/// One public trade executed on the marketplace.
///
/// Schema: `_definitions/PublicTrade.md`.
///
/// Cannot derive [`Eq`] because `price` is a `Decimal` (which only
/// implements `PartialEq` after the `arbitrary_precision` adapter).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PublicTrade {
    /// Buying participant. Optional per the schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broker_buying: Option<String>,
    /// Selling participant. Optional per the schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broker_selling: Option<String>,
    /// Market ID.
    pub market_id: MarketId,
    /// The price of the trade. `Decimal` (never `f64`) per CONTRACTS.md.
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub price: Decimal,
    /// Tick timestamp. UNIX time in milliseconds (see module doc note).
    pub tick_timestamp: i64,
    /// The trade ID on the exchange.
    pub trade_id: String,
    /// Trade timestamp. UNIX time in milliseconds (see module doc note).
    pub trade_timestamp: i64,
    /// The trade type defined by the exchange. Optional per the schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_type: Option<String>,
    /// The volume of the trade.
    pub volume: i64,
}

/// Public trades for a single tradable.
///
/// Schema: `_definitions/TradablePublicTrades.md`. Cannot derive [`Eq`]
/// because the nested [`PublicTrade::price`] is a `Decimal`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TradablePublicTrades {
    /// The tradable identifier. The combination of market ID and tradable
    /// identifier is unique.
    pub identifier: TradableId,
    /// The Nordnet unique market identifier.
    pub market_id: MarketId,
    /// A list of the public trades.
    pub trades: Vec<PublicTrade>,
}

/// Customer trading eligibility for a single tradable.
///
/// Schema: `_definitions/TradableEligibility.md`. All fields are required.
///
/// Note: `market_id` is documented as `integer(int32)` here while every
/// other `market_id` in the API is `integer(int64)`. We keep the uniform
/// [`MarketId`] (`i64`) newtype — see module doc note.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TradableEligibility {
    /// `true` if the customer is eligible to trade the tradable.
    pub eligible: bool,
    /// The tradable identifier. The combination of market ID and tradable
    /// ID is unique.
    pub identifier: TradableId,
    /// The market identifier.
    pub market_id: MarketId,
}
