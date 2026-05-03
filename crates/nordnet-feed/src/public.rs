//! Public feed event payload types.
//!
//! Tick-framed payloads (`Price`, `Depth`, `Trade`, `TradingStatus`,
//! `Indicator`) admit absent non-key fields per Nordnet's tick-framing
//! rules: the first message after subscribe is full, subsequent messages
//! carry only changed fields. The keys (`identifier` + `market_id`) are
//! present on every frame.
//!
//! `News` is NOT tick-framed — each news event is published once with
//! its full payload, so non-id fields are required.
//!
//! No `deny_unknown_fields`: Nordnet's docs explicitly state new fields
//! can appear at any time.
//!
//! Decimal vs i64 split: prices and volumes are [`rust_decimal::Decimal`]
//! because the wire is willing to send fractional-as-float
//! (`"volume": 111.0`). Counts (order counts in depth), timestamps, and
//! the news/source ids stay `i64`.

use nordnet_model::ids::{InstrumentId, MarketId, TradableId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Price tick. First message after subscription is full; subsequent
/// messages carry only changed non-key fields. Absent = unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Price {
    #[serde(rename = "i")]
    pub identifier: TradableId,
    #[serde(rename = "m")]
    pub market_id: MarketId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vwap: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ep: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extended_last: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_volume: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_volume: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_volume: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turnover_volume: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paired: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imbalance: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turnover: Option<Decimal>,
}

/// Order book depth tick. Levels 1–5; absent levels = unchanged.
///
/// Note: `bid_orders{n}` / `ask_orders{n}` are `i64` order COUNTS, not
/// volumes (despite the naming pattern matching `bid_volume{n}`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Depth {
    #[serde(rename = "i")]
    pub identifier: TradableId,
    #[serde(rename = "m")]
    pub market_id: MarketId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_timestamp: Option<i64>,
    // Level 1
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid1: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask1: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_volume1: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_volume1: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_orders1: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_orders1: Option<i64>,
    // Level 2
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid2: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask2: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_volume2: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_volume2: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_orders2: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_orders2: Option<i64>,
    // Level 3
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid3: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask3: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_volume3: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_volume3: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_orders3: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_orders3: Option<i64>,
    // Level 4
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid4: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask4: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_volume4: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_volume4: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_orders4: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_orders4: Option<i64>,
    // Level 5
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid5: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask5: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_volume5: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_volume5: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_orders5: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_orders5: Option<i64>,
}

/// Public market trade tick. NOT to be confused with the private
/// feed's own-account trade payload (see private.rs).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Trade {
    #[serde(rename = "i")]
    pub identifier: TradableId,
    #[serde(rename = "m")]
    pub market_id: MarketId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broker_buying: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broker_selling: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_type: Option<String>,
}

/// Trading status tick. `status` is a single character per Nordnet
/// (C/R/D/X/U) but typed as String to admit future codes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TradingStatus {
    #[serde(rename = "i")]
    pub identifier: TradableId,
    #[serde(rename = "m")]
    pub market_id: MarketId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub halted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orderbook_status: Option<String>,
}

/// Indicator tick (e.g. OMXS30). Both `identifier` (the index code,
/// e.g. `"OMXS30"`) and `market` (the source venue, e.g. `"SSE"`) are
/// strings — distinct from the typed `MarketId`/`TradableId` shape used
/// for tradable instruments.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Indicator {
    #[serde(rename = "i")]
    pub identifier: String,
    #[serde(rename = "m")]
    pub market: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delayed: Option<i64>,
}

/// News headline event.
///
/// News frames are NOT tick-framed deltas — Nordnet publishes each news
/// item once with its full payload. All non-id fields are therefore
/// required.
///
/// Wire field `type` → Rust field `kind` (avoids the keyword collision
/// and disambiguates from the envelope `type`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct News {
    pub news_id: i64,
    pub lang: String,
    pub timestamp: i64,
    pub source_id: i64,
    pub headline: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// Tradable IDs the news item references. Always present, may be
    /// empty (per Nordnet news-without-instrument frames).
    pub instruments: Vec<InstrumentId>,
}
