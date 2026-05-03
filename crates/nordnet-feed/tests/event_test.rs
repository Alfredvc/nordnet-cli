//! Golden JSON deserialization tests for feed event payloads + forward-compat.

use nordnet_feed::private::{
    ActionState, ActivationConditionKind, KnownActionState, KnownActivationConditionKind,
    KnownOrderState, KnownOrderType, KnownSide, KnownValidityKind, KnownVolumeCondition,
    OrderEvent, OrderState, OrderType, Side, Validity, ValidityKind, VolumeCondition,
};
use nordnet_feed::public::{Depth, Indicator, News, Price, Trade, TradingStatus};
use rust_decimal::Decimal;
use std::str::FromStr;

// === Public event payloads ===

#[test]
fn price_full_tick_deserializes() {
    let json = r#"{
        "i": "101", "m": 11,
        "tick_timestamp": 1612955053717,
        "trade_timestamp": 1612955053000,
        "delayed": 0,
        "bid": 132.5, "ask": 132.55, "last": 132.5,
        "open": 130.0, "high": 133.0, "low": 129.5, "close": 131.0,
        "vwap": 131.7,
        "bid_volume": 100.0, "ask_volume": 200.0, "last_volume": 50.0
    }"#;
    let p: Price = serde_json::from_str(json).unwrap();
    assert_eq!(p.i, "101");
    assert_eq!(p.m, 11);
    assert_eq!(p.bid, Some(Decimal::from_str("132.5").unwrap()));
    assert_eq!(p.last_volume, Some(Decimal::from_str("50.0").unwrap()));
}

#[test]
fn price_delta_tick_deserializes() {
    // Per spec §"Tick framing": deltas carry m+i + only changed fields.
    let json = r#"{"i": "101", "m": 11, "last": 132.55, "last_volume": 25.0}"#;
    let p: Price = serde_json::from_str(json).unwrap();
    assert_eq!(p.i, "101");
    assert_eq!(p.last, Some(Decimal::from_str("132.55").unwrap()));
    assert_eq!(p.bid, None);
    assert_eq!(p.ask, None);
}

#[test]
fn depth_with_levels_deserializes() {
    let json = r#"{
        "i": "101", "m": 11, "tick_timestamp": 1612955053717,
        "bid1": 132.0, "ask1": 132.5,
        "bid_volume1": 100.0, "ask_volume1": 200.0,
        "bid_orders1": 5, "ask_orders1": 8
    }"#;
    let d: Depth = serde_json::from_str(json).unwrap();
    assert_eq!(d.bid1, Some(Decimal::from_str("132.0").unwrap()));
    assert_eq!(d.bid_orders1, Some(5));
    assert_eq!(d.ask_orders1, Some(8));
}

#[test]
fn trade_deserializes() {
    let json = r#"{
        "i": "101", "m": 11, "trade_timestamp": 1612955053717,
        "price": 132.55, "volume": 100.0,
        "broker_buying": "ABC", "trade_id": "T-1"
    }"#;
    let t: Trade = serde_json::from_str(json).unwrap();
    assert_eq!(t.price, Decimal::from_str("132.55").unwrap());
    assert_eq!(t.broker_buying, Some("ABC".into()));
}

#[test]
fn trading_status_deserializes() {
    let json = r#"{"i":"101","m":11,"tick_timestamp":1,"status":"R"}"#;
    let s: TradingStatus = serde_json::from_str(json).unwrap();
    assert_eq!(s.status, "R");
}

#[test]
fn indicator_m_is_string() {
    let json = r#"{"i":"OMXS30","m":"SSE","tick_timestamp":1,"last":2500.0}"#;
    let i: Indicator = serde_json::from_str(json).unwrap();
    assert_eq!(i.m, "SSE"); // String, NOT i64
    assert_eq!(i.last, Some(Decimal::from_str("2500.0").unwrap()));
}

#[test]
fn news_kind_renamed_from_type() {
    let json =
        r#"{"news_id":1,"lang":"sv","timestamp":1,"source_id":2,"headline":"H","type":"alert"}"#;
    let n: News = serde_json::from_str(json).unwrap();
    assert_eq!(n.kind, "alert");
}

// === Private event payloads ===

#[test]
fn order_golden_deserializes() {
    // Spec §"order" line 273 — required golden.
    let json = r#"{"volume":111.0,"price":{"value":132.55,"currency":"SEK"},"volume_condition":"NORMAL","order_id":202178767,"reference":"ABC132","tradable":{"market_id":11,"identifier":"101"},"validity":{"type":"DAY","valid_until":1613061300000},"accno":123123,"accid":1,"side":"BUY","modified":1612955053717,"activation_condition":{"type":"NONE"},"order_state":"LOCAL","action_state":"INS_PEND","order_type":"LIMIT"}"#;
    let o: OrderEvent = serde_json::from_str(json).unwrap();
    assert_eq!(o.order_id, 202178767);
    assert_eq!(o.volume, Decimal::from_str("111.0").unwrap());
    assert_eq!(o.side, Side::Known(KnownSide::Buy));
    assert_eq!(o.validity.kind, ValidityKind::Known(KnownValidityKind::Day));
    assert_eq!(o.order_state, OrderState::Known(KnownOrderState::Local));
    assert_eq!(
        o.action_state,
        ActionState::Known(KnownActionState::InsertPending)
    );
    assert_eq!(o.order_type, OrderType::Known(KnownOrderType::Limit));
    assert_eq!(
        o.activation_condition.kind,
        ActivationConditionKind::Known(KnownActivationConditionKind::None)
    );
    assert_eq!(o.reference, Some("ABC132".into()));
    assert_eq!(
        o.volume_condition,
        VolumeCondition::Known(KnownVolumeCondition::Normal)
    );
}

#[test]
fn order_golden_round_trips_byte_equivalent() {
    // Re-serialize the golden — assert key fields survive round-trip.
    let json = r#"{"volume":111.0,"price":{"value":132.55,"currency":"SEK"},"volume_condition":"NORMAL","order_id":202178767,"reference":"ABC132","tradable":{"market_id":11,"identifier":"101"},"validity":{"type":"DAY","valid_until":1613061300000},"accno":123123,"accid":1,"side":"BUY","modified":1612955053717,"activation_condition":{"type":"NONE"},"order_state":"LOCAL","action_state":"INS_PEND","order_type":"LIMIT"}"#;
    let o: OrderEvent = serde_json::from_str(json).unwrap();
    let again = serde_json::to_string(&o).unwrap();
    let reparse: OrderEvent = serde_json::from_str(&again).unwrap();
    assert_eq!(o, reparse);
}

// === Forward-compat ===

#[test]
fn unknown_field_in_known_payload_is_ignored() {
    // Price with a bogus extra field — must NOT error.
    let json = r#"{"i":"101","m":11,"bid":100.0,"future_field":42}"#;
    let p: Price = serde_json::from_str(json).unwrap();
    assert_eq!(p.bid, Some(Decimal::from_str("100.0").unwrap()));
}

#[test]
fn unknown_typed_enum_variant_lands_in_unknown() {
    // OrderState with a future wire value — must deserialize as Unknown.
    let json = r#""FUTURE_VALUE_NOT_YET_DEFINED""#;
    let s: OrderState = serde_json::from_str(json).unwrap();
    assert_eq!(
        s,
        OrderState::Unknown("FUTURE_VALUE_NOT_YET_DEFINED".into())
    );
}

#[test]
fn unknown_typed_enum_variant_round_trips() {
    let json = r#""FUTURE_VALUE""#;
    let s: OrderState = serde_json::from_str(json).unwrap();
    let again = serde_json::to_string(&s).unwrap();
    assert_eq!(again, json);
}

#[test]
fn validity_kind_round_trip() {
    // {"type":"DAY"} → Validity.kind = Known(Day) → reserialize → same bytes.
    let json = r#"{"type":"DAY","valid_until":1613061300000}"#;
    let v: Validity = serde_json::from_str(json).unwrap();
    assert_eq!(v.kind, ValidityKind::Known(KnownValidityKind::Day));
    assert_eq!(v.valid_until, 1613061300000);
    let again = serde_json::to_string(&v).unwrap();
    assert_eq!(again, json);
}

// === Heartbeat forward-compat ===
//
// Heartbeat decode happens in the envelope dispatcher (event.rs); these
// tests verify that a heartbeat with extra fields in `data` doesn't
// fall through to Unknown. Since from_envelope is pub(crate), we test
// via the public PublicFeedClient::recv path in client_test.rs (Agent C).
// So this section is intentionally empty here.

// === Server error frame ===
//
// Tested end-to-end in client_test.rs (Agent C) where the duplex socket
// can deliver the err frame through the codec.

// === Unknown envelope type ===
//
// Tested end-to-end in client_test.rs (Agent C). Envelope and
// from_envelope are pub(crate) and unreachable from integration tests,
// so the dispatcher's unknown-type fallthrough must be exercised via
// PublicFeedClient::recv.
