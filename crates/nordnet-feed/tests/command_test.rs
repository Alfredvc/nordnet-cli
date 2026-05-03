//! Wire-byte assertions for outbound feed command frames.
//!
//! These tests pin the exact JSON bytes the encoder emits so future
//! changes can't silently shift wire format.
//!
//! `encode_login_frame` and `encode_subscribe_frame` are `pub(crate)`, so
//! they are not reachable from cross-crate integration tests. Instead, this
//! file exercises serialization through the public `Serialize` impls of
//! `LoginCommand` and `SubscribeArgs` directly:
//!
//! - `LoginCommand` serializes its own full `{"cmd":"login","args":{...}}`
//!   envelope — tested via `serde_json::to_string`.
//! - `SubscribeArgs` serializes only the args object — the outer envelope
//!   is reconstructed with string concatenation to guarantee `cmd` comes
//!   before `args` (serde_json without `preserve_order` uses BTreeMap and
//!   would sort alphabetically, emitting `args` before `cmd`).

use nordnet_feed::command::{LoginCommand, MarketDataKind, SubscribeArgs};

/// Build the full `{"cmd":"<cmd>","args":<args_json>}` envelope by
/// serializing `args` first, then concatenating. This mirrors what
/// `encode_subscribe_frame` does internally (via `SerializeMap`) and
/// guarantees `cmd` precedes `args` regardless of serde_json's internal
/// map ordering.
fn frame(cmd: &str, args: &SubscribeArgs) -> String {
    let inner = serde_json::to_string(args).unwrap();
    format!(r#"{{"cmd":"{cmd}","args":{inner}}}"#)
}

// ── Login ─────────────────────────────────────────────────────────────────

#[test]
fn login_frame_wire_bytes() {
    // LoginCommand<'a> serializes its own full envelope via a manual
    // Serialize impl that writes cmd → args in insertion order.
    let cmd = LoginCommand { session_key: "K" };
    let bytes = serde_json::to_string(&cmd).unwrap();
    assert_eq!(
        bytes,
        r#"{"cmd":"login","args":{"session_key":"K","service":"NEXTAPI"}}"#
    );
}

// ── MarketData ────────────────────────────────────────────────────────────

#[test]
fn subscribe_market_data_price() {
    let args = SubscribeArgs::MarketData {
        kind: MarketDataKind::Price,
        market: 11,
        identifier: "101".into(),
    };
    let bytes = frame("subscribe", &args);
    assert_eq!(
        bytes,
        r#"{"cmd":"subscribe","args":{"t":"price","m":11,"i":"101"}}"#
    );
}

#[test]
fn subscribe_market_data_depth() {
    let args = SubscribeArgs::MarketData {
        kind: MarketDataKind::Depth,
        market: 11,
        identifier: "101".into(),
    };
    let bytes = frame("subscribe", &args);
    assert_eq!(
        bytes,
        r#"{"cmd":"subscribe","args":{"t":"depth","m":11,"i":"101"}}"#
    );
}

#[test]
fn subscribe_market_data_trade() {
    let args = SubscribeArgs::MarketData {
        kind: MarketDataKind::Trade,
        market: 11,
        identifier: "101".into(),
    };
    let bytes = frame("subscribe", &args);
    assert_eq!(
        bytes,
        r#"{"cmd":"subscribe","args":{"t":"trade","m":11,"i":"101"}}"#
    );
}

#[test]
fn subscribe_market_data_trading_status() {
    let args = SubscribeArgs::MarketData {
        kind: MarketDataKind::TradingStatus,
        market: 11,
        identifier: "101".into(),
    };
    let bytes = frame("subscribe", &args);
    assert_eq!(
        bytes,
        r#"{"cmd":"subscribe","args":{"t":"trading_status","m":11,"i":"101"}}"#
    );
}

// ── Indicator ─────────────────────────────────────────────────────────────

#[test]
fn subscribe_indicator() {
    // Indicator uses a string `m` ("SSE"), not an integer market id.
    let args = SubscribeArgs::Indicator {
        market: "SSE".into(),
        identifier: "OMXS30".into(),
    };
    let bytes = frame("subscribe", &args);
    assert_eq!(
        bytes,
        r#"{"cmd":"subscribe","args":{"t":"indicator","m":"SSE","i":"OMXS30"}}"#
    );
}

// ── News ──────────────────────────────────────────────────────────────────

#[test]
fn subscribe_news_no_delay_omits_field() {
    // delay: None → field omitted entirely (no "null").
    let args = SubscribeArgs::News {
        source_id: 2,
        delay: None,
    };
    let bytes = frame("subscribe", &args);
    assert_eq!(bytes, r#"{"cmd":"subscribe","args":{"t":"news","s":2}}"#);
}

#[test]
fn subscribe_news_explicit_false_emits_field() {
    // delay: Some(false) → "delay":false emitted (NOT omitted).
    // This is the key distinction — absence vs explicit false are different
    // on the wire.
    let args = SubscribeArgs::News {
        source_id: 2,
        delay: Some(false),
    };
    let bytes = frame("subscribe", &args);
    assert_eq!(
        bytes,
        r#"{"cmd":"subscribe","args":{"t":"news","s":2,"delay":false}}"#
    );
}

#[test]
fn subscribe_news_explicit_true() {
    let args = SubscribeArgs::News {
        source_id: 2,
        delay: Some(true),
    };
    let bytes = frame("subscribe", &args);
    assert_eq!(
        bytes,
        r#"{"cmd":"subscribe","args":{"t":"news","s":2,"delay":true}}"#
    );
}

// ── Unsubscribe ───────────────────────────────────────────────────────────

#[test]
fn unsubscribe_mirrors_subscribe() {
    // Same args, just the cmd verb changes — args shape is identical.
    let args = SubscribeArgs::MarketData {
        kind: MarketDataKind::Price,
        market: 11,
        identifier: "101".into(),
    };
    let bytes = frame("unsubscribe", &args);
    assert_eq!(
        bytes,
        r#"{"cmd":"unsubscribe","args":{"t":"price","m":11,"i":"101"}}"#
    );
}

// ── Round-trip ────────────────────────────────────────────────────────────

#[test]
fn subscribe_args_round_trip_for_unsubscribe_symmetry() {
    // SubscribeArgs derives Clone + PartialEq + Eq + Hash so callers can
    // stash the same value and hand it back to unsubscribe() later.
    let a = SubscribeArgs::MarketData {
        kind: MarketDataKind::Price,
        market: 11,
        identifier: "101".into(),
    };
    let b = a.clone();
    assert_eq!(a, b);

    let mut set = std::collections::HashSet::new();
    set.insert(a);
    assert!(set.contains(&b));
}
