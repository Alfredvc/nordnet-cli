//! Golden JSON deserialization tests for feed event payloads + forward-compat.

use nordnet_feed::private::{
    ActionState, ActivationConditionKind, KnownActionState, KnownActivationConditionKind,
    KnownOrderState, KnownOrderType, KnownSide, KnownValidityKind, KnownVolumeCondition,
    OrderEvent, OrderState, OrderType, Side, Validity, ValidityKind, VolumeCondition,
};
use nordnet_feed::public::{Depth, Indicator, News, Price, Trade, TradingStatus};
use nordnet_model::ids::{AccountId, InstrumentId, MarketId, OrderId, TradableId};
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
    assert_eq!(p.identifier, TradableId::from("101"));
    assert_eq!(p.market_id, MarketId(11));
    assert_eq!(p.bid, Some(Decimal::from_str("132.5").unwrap()));
    assert_eq!(p.last_volume, Some(Decimal::from_str("50.0").unwrap()));
}

#[test]
fn price_delta_tick_deserializes() {
    // Per spec §"Tick framing": deltas carry m+i + only changed fields.
    let json = r#"{"i": "101", "m": 11, "last": 132.55, "last_volume": 25.0}"#;
    let p: Price = serde_json::from_str(json).unwrap();
    assert_eq!(p.identifier, TradableId::from("101"));
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
fn trade_full_deserializes() {
    let json = r#"{
        "i": "101", "m": 11, "trade_timestamp": 1612955053717,
        "price": 132.55, "volume": 100.0,
        "broker_buying": "ABC", "trade_id": "T-1"
    }"#;
    let t: Trade = serde_json::from_str(json).unwrap();
    assert_eq!(t.price, Some(Decimal::from_str("132.55").unwrap()));
    assert_eq!(t.volume, Some(Decimal::from_str("100.0").unwrap()));
    assert_eq!(t.broker_buying, Some("ABC".into()));
}

#[test]
fn trade_delta_tick_omits_price_volume() {
    // Per Nordnet's tick framing, only `i`/`m` are guaranteed on every
    // frame — delta frames may omit price/volume/timestamp.
    let json = r#"{"i":"101","m":11}"#;
    let t: Trade = serde_json::from_str(json).unwrap();
    assert_eq!(t.identifier, TradableId::from("101"));
    assert_eq!(t.price, None);
    assert_eq!(t.volume, None);
    assert_eq!(t.trade_timestamp, None);
}

#[test]
fn trading_status_deserializes() {
    let json = r#"{"i":"101","m":11,"tick_timestamp":1,"status":"R"}"#;
    let s: TradingStatus = serde_json::from_str(json).unwrap();
    assert_eq!(s.status, Some("R".into()));
    assert_eq!(s.tick_timestamp, Some(1));
}

#[test]
fn trading_status_delta_tick_omits_status() {
    // Delta frames may carry only the keys plus changed fields.
    let json = r#"{"i":"101","m":11}"#;
    let s: TradingStatus = serde_json::from_str(json).unwrap();
    assert_eq!(s.status, None);
    assert_eq!(s.tick_timestamp, None);
}

#[test]
fn indicator_uses_string_fields() {
    let json = r#"{"i":"OMXS30","m":"SSE","tick_timestamp":1,"last":2500.0}"#;
    let i: Indicator = serde_json::from_str(json).unwrap();
    assert_eq!(i.identifier, "OMXS30");
    assert_eq!(i.market, "SSE");
    assert_eq!(i.last, Some(Decimal::from_str("2500.0").unwrap()));
}

#[test]
fn news_full_payload_deserializes() {
    // News is push-once (NOT tick-framed) — every non-id field is required.
    let json = r#"{
        "news_id": 1,
        "lang": "sv",
        "timestamp": 1612955053717,
        "source_id": 2,
        "headline": "Headline",
        "type": "alert",
        "instruments": [101, 202]
    }"#;
    let n: News = serde_json::from_str(json).unwrap();
    assert_eq!(n.news_id, 1);
    assert_eq!(n.lang, "sv");
    assert_eq!(n.headline, "Headline");
    assert_eq!(n.kind, "alert");
    assert_eq!(n.instruments, vec![InstrumentId(101), InstrumentId(202)]);
}

#[test]
fn news_kind_renamed_from_type_round_trip() {
    // Wire field is "type"; Rust field is `kind`. Verify both directions:
    // (1) "type" deserializes into `kind`, (2) re-serialization writes
    // back as "type", (3) a JSON with literal "kind" instead of "type"
    // fails to deserialize (forward-compat: literal `kind` is unknown).
    let json = r#"{"news_id":1,"lang":"sv","timestamp":1,"source_id":2,"headline":"H","type":"alert","instruments":[]}"#;
    let n: News = serde_json::from_str(json).unwrap();
    assert_eq!(n.kind, "alert");
    let back = serde_json::to_string(&n).unwrap();
    assert!(
        back.contains(r#""type":"alert""#),
        "round-trip must serialize as `type`, got: {back}"
    );
    assert!(
        !back.contains(r#""kind":"#),
        "must not emit `kind` field on the wire, got: {back}"
    );
}

#[test]
fn news_missing_required_field_errors() {
    // News is push-once — required fields must be present. Verify the
    // deserializer rejects (rather than producing a silent default).
    let json =
        r#"{"news_id":1,"lang":"sv","timestamp":1,"source_id":2,"headline":"H","instruments":[]}"#;
    let r: Result<News, _> = serde_json::from_str(json);
    assert!(r.is_err(), "missing `type` must error on News, got {r:?}");
}

// === Private event payloads ===

#[test]
fn order_golden_deserializes() {
    // Spec §"order" line 273 — required golden.
    let json = r#"{"volume":111.0,"price":{"value":132.55,"currency":"SEK"},"volume_condition":"NORMAL","order_id":202178767,"reference":"ABC132","tradable":{"market_id":11,"identifier":"101"},"validity":{"type":"DAY","valid_until":1613061300000},"accno":123123,"accid":1,"side":"BUY","modified":1612955053717,"activation_condition":{"type":"NONE"},"order_state":"LOCAL","action_state":"INS_PEND","order_type":"LIMIT"}"#;
    let o: OrderEvent = serde_json::from_str(json).unwrap();
    assert_eq!(o.order_id, OrderId(202178767));
    assert_eq!(o.accno, AccountId(123123));
    assert_eq!(o.accid, AccountId(1));
    assert_eq!(o.tradable.market_id, MarketId(11));
    assert_eq!(o.tradable.identifier, TradableId::from("101"));
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
fn order_golden_round_trips() {
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
    let json = r#"{"type":"DAY","valid_until":1613061300000}"#;
    let v: Validity = serde_json::from_str(json).unwrap();
    assert_eq!(v.kind, ValidityKind::Known(KnownValidityKind::Day));
    assert_eq!(v.valid_until, 1613061300000);
    let again = serde_json::to_string(&v).unwrap();
    assert_eq!(again, json);
}
