//! Public feed event payload types.
//!
//! All structs admit absent fields (delta-friendly per Nordnet's
//! tick-framing rules — first message is full, subsequent messages
//! contain only changed fields). No `deny_unknown_fields`: Nordnet's
//! docs explicitly state new fields can appear at any time.
//!
//! Decimal vs i64 split: prices and volumes are [`rust_decimal::Decimal`]
//! because the wire is willing to send fractional-as-float (`"volume": 111.0`).
//! Counts (order counts in depth), ids (market/instrument/source/news),
//! and timestamps stay `i64`.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Price tick. First message after subscription is full; subsequent
/// messages carry only changed fields. Absent = unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct Price {
    pub i: String,
    pub m: i64,
    #[serde(default)]
    pub delayed: Option<i64>,
    #[serde(default)]
    pub trade_timestamp: Option<i64>,
    #[serde(default)]
    pub tick_timestamp: Option<i64>,
    // Bid/ask/last/open/high/low/close/vwap/ep/extended_last
    #[serde(default)]
    pub bid: Option<Decimal>,
    #[serde(default)]
    pub ask: Option<Decimal>,
    #[serde(default)]
    pub last: Option<Decimal>,
    #[serde(default)]
    pub open: Option<Decimal>,
    #[serde(default)]
    pub high: Option<Decimal>,
    #[serde(default)]
    pub low: Option<Decimal>,
    #[serde(default)]
    pub close: Option<Decimal>,
    #[serde(default)]
    pub vwap: Option<Decimal>,
    #[serde(default)]
    pub ep: Option<Decimal>,
    #[serde(default)]
    pub extended_last: Option<Decimal>,
    // Volume fields
    #[serde(default)]
    pub bid_volume: Option<Decimal>,
    #[serde(default)]
    pub ask_volume: Option<Decimal>,
    #[serde(default)]
    pub last_volume: Option<Decimal>,
    #[serde(default)]
    pub turnover_volume: Option<Decimal>,
    #[serde(default)]
    pub paired: Option<Decimal>,
    #[serde(default)]
    pub imbalance: Option<Decimal>,
    #[serde(default)]
    pub turnover: Option<Decimal>,
}

/// Order book depth tick. Levels 1–5; absent levels = unchanged.
///
/// Note: `bid_orders{n}` / `ask_orders{n}` are `i64` order COUNTS, not
/// volumes (despite the naming pattern matching `bid_volume{n}`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, Default)]
pub struct Depth {
    pub i: String,
    pub m: i64,
    pub tick_timestamp: i64,
    // Level 1
    #[serde(default)]
    pub bid1: Option<Decimal>,
    #[serde(default)]
    pub ask1: Option<Decimal>,
    #[serde(default)]
    pub bid_volume1: Option<Decimal>,
    #[serde(default)]
    pub ask_volume1: Option<Decimal>,
    #[serde(default)]
    pub bid_orders1: Option<i64>,
    #[serde(default)]
    pub ask_orders1: Option<i64>,
    // Level 2
    #[serde(default)]
    pub bid2: Option<Decimal>,
    #[serde(default)]
    pub ask2: Option<Decimal>,
    #[serde(default)]
    pub bid_volume2: Option<Decimal>,
    #[serde(default)]
    pub ask_volume2: Option<Decimal>,
    #[serde(default)]
    pub bid_orders2: Option<i64>,
    #[serde(default)]
    pub ask_orders2: Option<i64>,
    // Level 3
    #[serde(default)]
    pub bid3: Option<Decimal>,
    #[serde(default)]
    pub ask3: Option<Decimal>,
    #[serde(default)]
    pub bid_volume3: Option<Decimal>,
    #[serde(default)]
    pub ask_volume3: Option<Decimal>,
    #[serde(default)]
    pub bid_orders3: Option<i64>,
    #[serde(default)]
    pub ask_orders3: Option<i64>,
    // Level 4
    #[serde(default)]
    pub bid4: Option<Decimal>,
    #[serde(default)]
    pub ask4: Option<Decimal>,
    #[serde(default)]
    pub bid_volume4: Option<Decimal>,
    #[serde(default)]
    pub ask_volume4: Option<Decimal>,
    #[serde(default)]
    pub bid_orders4: Option<i64>,
    #[serde(default)]
    pub ask_orders4: Option<i64>,
    // Level 5
    #[serde(default)]
    pub bid5: Option<Decimal>,
    #[serde(default)]
    pub ask5: Option<Decimal>,
    #[serde(default)]
    pub bid_volume5: Option<Decimal>,
    #[serde(default)]
    pub ask_volume5: Option<Decimal>,
    #[serde(default)]
    pub bid_orders5: Option<i64>,
    #[serde(default)]
    pub ask_orders5: Option<i64>,
}

/// Public market trade tick. NOT to be confused with the private
/// feed's own-account trade payload (see private.rs).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Trade {
    pub i: String,
    pub m: i64,
    pub trade_timestamp: i64,
    pub price: Decimal,
    pub volume: Decimal,
    #[serde(default)]
    pub broker_buying: Option<String>,
    #[serde(default)]
    pub broker_selling: Option<String>,
    #[serde(default)]
    pub trade_id: Option<String>,
    #[serde(default)]
    pub trade_type: Option<String>,
}

/// Trading status tick. `status` is a single character per Nordnet
/// (C/R/D/X/U) but typed as String to admit future codes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TradingStatus {
    pub i: String,
    pub m: i64,
    pub tick_timestamp: i64,
    pub status: String,
    #[serde(default)]
    pub source_status: Option<String>,
    #[serde(default)]
    pub halted: Option<String>,
    #[serde(default)]
    pub orderbook_status: Option<String>,
}

/// Indicator tick (e.g. OMXS30). NOTE: `m` is `String` here, NOT `i64`
/// like other event types — per Nordnet's docs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Indicator {
    pub i: String,
    pub m: String,
    pub tick_timestamp: i64,
    #[serde(default)]
    pub last: Option<Decimal>,
    #[serde(default)]
    pub high: Option<Decimal>,
    #[serde(default)]
    pub low: Option<Decimal>,
    #[serde(default)]
    pub close: Option<Decimal>,
    #[serde(default)]
    pub delayed: Option<i64>,
}

/// News headline event.
///
/// Wire field `type` → Rust field `kind` (avoids the keyword
/// collision and disambiguates from the envelope `type`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct News {
    pub news_id: i64,
    pub lang: String,
    pub timestamp: i64,
    pub source_id: i64,
    pub headline: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub instruments: Option<Vec<i64>>,
}
