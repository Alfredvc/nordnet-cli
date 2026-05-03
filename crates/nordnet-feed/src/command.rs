//! Outbound feed commands: login, subscribe, unsubscribe.
//!
//! Each command serializes to a single-line JSON object terminated by
//! `\n`. The terminator is added by the codec at write time; this
//! module only emits the JSON object body.
//!
//! # Field ordering
//!
//! `serde_json` without the `preserve_order` feature uses `BTreeMap`
//! internally, so both the `json!` macro and `serde_json::Map` sort keys
//! alphabetically. All serialization in this module uses `SerializeMap`
//! directly, which writes fields in the order `serialize_entry` is called
//! and never passes through an intermediate `Map`. This guarantees the
//! exact wire-byte ordering required by the Nordnet feed protocol.

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

/// Login frame body. Always `{"cmd":"login","args":{"session_key":"...","service":"NEXTAPI"}}`.
///
/// Per the official Python `next-api-v2-examples` repo, `service` is
/// always the literal string `"NEXTAPI"` (the public docs page omits
/// the field but the reference impl always sends it).
///
/// Not in the prelude (`crate::*`) — consumers call
/// `client.login(session_key)` rather than constructing this directly.
/// Kept `pub` for the wire-byte integration tests in `tests/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginCommand<'a> {
    pub session_key: &'a str,
}

/// Helper that serializes the login args sub-object in insertion order.
struct LoginArgs<'a> {
    session_key: &'a str,
}

impl Serialize for LoginArgs<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(2))?;
        m.serialize_entry("session_key", self.session_key)?;
        m.serialize_entry("service", "NEXTAPI")?;
        m.end()
    }
}

impl Serialize for LoginCommand<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut top = s.serialize_map(Some(2))?;
        top.serialize_entry("cmd", "login")?;
        top.serialize_entry(
            "args",
            &LoginArgs {
                session_key: self.session_key,
            },
        )?;
        top.end()
    }
}

/// Compile-time-distinct subscribe variants — prevents constructing
/// `Indicator` with an integer market or `News` with `m`/`i` fields.
///
/// Derives `Clone + Eq + Hash` so callers can stash a value and hand it
/// back to `unsubscribe()` later (round-trip symmetry). The `Hash`
/// derive depends on `rust_decimal::Decimal: Hash` indirectly (no
/// `Decimal` field today, but if one is ever added the derive must keep
/// compiling — load-bearing for stash-and-reuse).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubscribeArgs {
    /// Standard market data: price, depth, trade, trading_status.
    MarketData {
        kind: MarketDataKind,
        market: i64,
        identifier: String,
    },
    /// Indicator subscriptions use a string `m` per Nordnet's docs.
    Indicator { market: String, identifier: String },
    /// News uses `s` (source id) instead of `m`/`i`. `delay` is news-only
    /// per Nordnet (deprecated even there — kept for completeness).
    News { source_id: i64, delay: Option<bool> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketDataKind {
    Price,
    Depth,
    Trade,
    TradingStatus,
}

impl MarketDataKind {
    fn wire_t(self) -> &'static str {
        match self {
            Self::Price => "price",
            Self::Depth => "depth",
            Self::Trade => "trade",
            Self::TradingStatus => "trading_status",
        }
    }
}

impl Serialize for SubscribeArgs {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::MarketData {
                kind,
                market,
                identifier,
            } => {
                let mut m = s.serialize_map(Some(3))?;
                m.serialize_entry("t", kind.wire_t())?;
                m.serialize_entry("m", market)?;
                m.serialize_entry("i", identifier)?;
                m.end()
            }
            Self::Indicator { market, identifier } => {
                let mut m = s.serialize_map(Some(3))?;
                m.serialize_entry("t", "indicator")?;
                m.serialize_entry("m", market)?;
                m.serialize_entry("i", identifier)?;
                m.end()
            }
            Self::News { source_id, delay } => {
                // delay: None -> field omitted entirely (no `null`).
                // delay: Some(false) -> `"delay":false` (NOT omitted).
                let len = if delay.is_some() { 3 } else { 2 };
                let mut m = s.serialize_map(Some(len))?;
                m.serialize_entry("t", "news")?;
                m.serialize_entry("s", source_id)?;
                if let Some(d) = delay {
                    m.serialize_entry("delay", d)?;
                }
                m.end()
            }
        }
    }
}

/// Full subscribe/unsubscribe frame: wraps cmd_name + args into the outer
/// envelope `{"cmd":"...","args":{...}}`.
///
/// Uses `SerializeMap` directly (not `serde_json::Map` or `json!`) so field
/// order is preserved: `cmd` first, then `args`. Without the `preserve_order`
/// feature, both `serde_json::Map` and `json!` use `BTreeMap` internally and
/// sort keys alphabetically, which would emit `args` before `cmd`.
// Used by encode_subscribe_frame, which is called from the client layer (phase 2.3).
#[allow(dead_code)]
struct SubscribeFrame<'a> {
    cmd_name: &'static str,
    args: &'a SubscribeArgs,
}

impl Serialize for SubscribeFrame<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut top = s.serialize_map(Some(2))?;
        top.serialize_entry("cmd", self.cmd_name)?;
        top.serialize_entry("args", self.args)?;
        top.end()
    }
}

/// Wraps subscribe/unsubscribe args into the full command frame.
///
/// `cmd_name` is `"subscribe"` or `"unsubscribe"` — the same args
/// shape works for both calls.
// Called from the client layer (phase 2.3).
#[allow(dead_code)]
pub(crate) fn encode_subscribe_frame(
    cmd_name: &'static str,
    args: &SubscribeArgs,
) -> serde_json::Result<String> {
    serde_json::to_string(&SubscribeFrame { cmd_name, args })
}

/// Encodes a login frame.
// Called from the client layer (phase 2.3).
#[allow(dead_code)]
pub(crate) fn encode_login_frame(cmd: &LoginCommand<'_>) -> serde_json::Result<String> {
    serde_json::to_string(cmd)
}

#[cfg(test)]
mod wire_byte_tests {
    use super::*;

    #[test]
    fn login_wire_bytes() {
        let s = encode_login_frame(&LoginCommand { session_key: "K" }).unwrap();
        assert_eq!(
            s,
            r#"{"cmd":"login","args":{"session_key":"K","service":"NEXTAPI"}}"#
        );
    }

    #[test]
    fn subscribe_market_data_price() {
        let args = SubscribeArgs::MarketData {
            kind: MarketDataKind::Price,
            market: 11,
            identifier: "101".to_string(),
        };
        let s = encode_subscribe_frame("subscribe", &args).unwrap();
        assert_eq!(
            s,
            r#"{"cmd":"subscribe","args":{"t":"price","m":11,"i":"101"}}"#
        );
    }

    #[test]
    fn subscribe_indicator() {
        let args = SubscribeArgs::Indicator {
            market: "SSE".to_string(),
            identifier: "OMXS30".to_string(),
        };
        let s = encode_subscribe_frame("subscribe", &args).unwrap();
        assert_eq!(
            s,
            r#"{"cmd":"subscribe","args":{"t":"indicator","m":"SSE","i":"OMXS30"}}"#
        );
    }

    #[test]
    fn subscribe_news_no_delay() {
        let args = SubscribeArgs::News {
            source_id: 2,
            delay: None,
        };
        let s = encode_subscribe_frame("subscribe", &args).unwrap();
        assert_eq!(s, r#"{"cmd":"subscribe","args":{"t":"news","s":2}}"#);
    }

    #[test]
    fn subscribe_news_delay_false() {
        let args = SubscribeArgs::News {
            source_id: 2,
            delay: Some(false),
        };
        let s = encode_subscribe_frame("subscribe", &args).unwrap();
        assert_eq!(
            s,
            r#"{"cmd":"subscribe","args":{"t":"news","s":2,"delay":false}}"#
        );
    }

    #[test]
    fn subscribe_news_delay_true() {
        let args = SubscribeArgs::News {
            source_id: 2,
            delay: Some(true),
        };
        let s = encode_subscribe_frame("subscribe", &args).unwrap();
        assert_eq!(
            s,
            r#"{"cmd":"subscribe","args":{"t":"news","s":2,"delay":true}}"#
        );
    }

    #[test]
    fn unsubscribe_mirrors_subscribe_shape() {
        let args = SubscribeArgs::MarketData {
            kind: MarketDataKind::Depth,
            market: 11,
            identifier: "101".to_string(),
        };
        let s = encode_subscribe_frame("unsubscribe", &args).unwrap();
        assert_eq!(
            s,
            r#"{"cmd":"unsubscribe","args":{"t":"depth","m":11,"i":"101"}}"#
        );
    }

    #[test]
    fn cmd_precedes_args() {
        // Regression: without preserve_order, serde_json::Map/json! would emit
        // args before cmd (BTreeMap sorts keys alphabetically).
        let args = SubscribeArgs::Indicator {
            market: "SSE".to_string(),
            identifier: "OMXS30".to_string(),
        };
        let s = encode_subscribe_frame("subscribe", &args).unwrap();
        assert!(s.starts_with(r#"{"cmd":"#), "cmd must come first, got: {s}");
    }
}
