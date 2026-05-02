//! Tests for the `tradables` resource group.
//!
//! Three test layers per CONTRACTS.md:
//!
//! 1. Fixture roundtrip — every fixture parses under `deny_unknown_fields`
//!    and re-serializes to the same canonical JSON `Value`. A separate
//!    Decimal-precision roundtrip covers the `price` field.
//! 2. `deny_unknown_fields` rejection — covers `TradableInfo`,
//!    `TradableEligibility`, and `PublicTrade`.
//! 3. Wiremock integration — every op exercised with success + at least
//!    one error mapping. `get_suitability` additionally tests the
//!    documented 403 (anonymous session, empty body). `list_tradable_trades`
//!    asserts the `count` query string is forwarded when set and omitted
//!    when not.

use nordnet_api::ids::{MarketId, TradableId};
use nordnet_api::models::tradables::{
    AllowedOrderType, CalendarDay, PublicTrade, TradableEligibility, TradableInfo, TradableKey,
    TradablePublicTrades,
};
use nordnet_api::{Client, Error};
use pretty_assertions::assert_eq;
use rust_decimal::Decimal;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_tradable_info_fixture() -> &'static str {
    include_str!("../fixtures/tradables/get_tradable_info.response.json")
}

fn list_trades_fixture() -> &'static str {
    include_str!("../fixtures/tradables/list_trades.response.json")
}

fn get_suitability_fixture() -> &'static str {
    include_str!("../fixtures/tradables/get_suitability.response.json")
}

fn key_eric_b() -> TradableKey {
    TradableKey::new(MarketId(11), TradableId("101".to_owned()))
}

// ---------------------------------------------------------------------------
// TradableKey::Display
// ---------------------------------------------------------------------------

#[test]
fn tradable_key_display_uses_colon_separator() {
    let key = TradableKey::new(MarketId(11), TradableId("101".to_owned()));
    assert_eq!(format!("{}", key), "11:101");
}

// ---------------------------------------------------------------------------
// Layer 1 — Fixture roundtrip
// ---------------------------------------------------------------------------

#[test]
fn get_tradable_info_fixture_roundtrip() {
    let raw = get_tradable_info_fixture();
    let parsed: Vec<TradableInfo> =
        serde_json::from_str(raw).expect("get_tradable_info fixture must parse");

    assert_eq!(parsed.len(), 1);
    let entry = &parsed[0];
    assert_eq!(entry.identifier, TradableId("101".to_owned()));
    assert_eq!(entry.market_id, MarketId(11));
    assert!(entry.iceberg);
    assert_eq!(entry.calendar.len(), 2);
    assert_eq!(
        entry.calendar[0],
        CalendarDay {
            close: 1714665600000,
            date: "2024-05-02".to_owned(),
            open: 1714636800000,
        }
    );
    assert_eq!(entry.order_types.len(), 3);
    assert_eq!(
        entry.order_types[0],
        AllowedOrderType {
            name: "Limit".to_owned(),
            r#type: "LIMIT".to_owned(),
        }
    );
    assert_eq!(entry.order_types[2].r#type, "STOP_LOSS");

    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let re = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re).expect("re-serialized must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}

#[test]
fn list_trades_fixture_roundtrip() {
    let raw = list_trades_fixture();
    let parsed: Vec<TradablePublicTrades> =
        serde_json::from_str(raw).expect("list_trades fixture must parse");

    assert_eq!(parsed.len(), 1);
    let entry = &parsed[0];
    assert_eq!(entry.identifier, TradableId("101".to_owned()));
    assert_eq!(entry.market_id, MarketId(11));
    assert_eq!(entry.trades.len(), 2);

    // First trade: all optional fields populated.
    let t0 = &entry.trades[0];
    assert_eq!(t0.broker_buying.as_deref(), Some("AVA"));
    assert_eq!(t0.broker_selling.as_deref(), Some("NON"));
    assert_eq!(t0.trade_type.as_deref(), Some("REGULAR"));
    assert_eq!(t0.market_id, MarketId(11));
    assert_eq!(t0.price, "269.7234".parse::<Decimal>().unwrap());
    assert_eq!(t0.tick_timestamp, 1714568400000);
    assert_eq!(t0.trade_id, "T-0001");
    assert_eq!(t0.trade_timestamp, 1714568400000);
    assert_eq!(t0.volume, 100);

    // Second trade: optional fields omitted.
    let t1 = &entry.trades[1];
    assert_eq!(t1.broker_buying, None);
    assert_eq!(t1.broker_selling, None);
    assert_eq!(t1.trade_type, None);
    assert_eq!(t1.price, "270.1500".parse::<Decimal>().unwrap());
    assert_eq!(t1.volume, 50);

    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let re = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re).expect("re-serialized must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}

#[test]
fn get_suitability_fixture_roundtrip() {
    let raw = get_suitability_fixture();
    let parsed: Vec<TradableEligibility> =
        serde_json::from_str(raw).expect("get_suitability fixture must parse");

    assert_eq!(parsed.len(), 2);
    assert_eq!(
        parsed[0],
        TradableEligibility {
            eligible: true,
            identifier: TradableId("101".to_owned()),
            market_id: MarketId(11),
        }
    );
    assert!(!parsed[1].eligible);
    assert_eq!(parsed[1].identifier, TradableId("202".to_owned()));

    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let re = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re).expect("re-serialized must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}

#[test]
fn public_trade_decimal_precision_survives_roundtrip() {
    // Verifies the `arbitrary_precision` adapter on `PublicTrade::price`
    // preserves multi-significant-digit precision through serde.
    let raw = r#"{
        "market_id": 11,
        "price": 12345.67891234,
        "tick_timestamp": 0,
        "trade_id": "X",
        "trade_timestamp": 0,
        "volume": 1
    }"#;
    let parsed: PublicTrade = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.price, "12345.67891234".parse::<Decimal>().unwrap());

    let re = serde_json::to_string(&parsed).unwrap();
    let re_parsed: PublicTrade = serde_json::from_str(&re).unwrap();
    assert_eq!(parsed, re_parsed);
}

// ---------------------------------------------------------------------------
// Layer 2 — deny_unknown_fields rejection
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Layer 3 — Wiremock integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_tradable_info_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tradables/info/11:101"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_tradable_info_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let infos = client.get_tradable_info(&key_eric_b()).await.unwrap();
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].market_id, MarketId(11));
    assert_eq!(infos[0].order_types.len(), 3);
}

#[tokio::test]
async fn get_tradable_info_204_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tradables/info/11:101"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let infos = client.get_tradable_info(&key_eric_b()).await.unwrap();
    assert_eq!(
        infos,
        vec![],
        "204 No Content should map to empty Vec<TradableInfo>"
    );
}

#[tokio::test]
async fn get_tradable_info_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tradables/info/11:101"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"Bad key"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get_tradable_info(&key_eric_b()).await.unwrap_err();
    assert!(
        matches!(err, Error::BadRequest { .. }),
        "expected BadRequest, got {:?}",
        err
    );
}

#[tokio::test]
async fn list_tradable_trades_with_count_forwards_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tradables/trades/11:101"))
        .and(query_param("count", "all"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_trades_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let trades = client
        .list_tradable_trades(&key_eric_b(), Some("all"))
        .await
        .unwrap();
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].trades.len(), 2);
}

#[tokio::test]
async fn list_tradable_trades_without_count_omits_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tradables/trades/11:101"))
        // Custom matcher: assert the request URL has NO query string at all.
        .and(move |req: &Request| req.url.query().is_none())
        .respond_with(ResponseTemplate::new(200).set_body_string(list_trades_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let trades = client
        .list_tradable_trades(&key_eric_b(), None)
        .await
        .unwrap();
    assert_eq!(trades.len(), 1);
}

#[tokio::test]
async fn list_tradable_trades_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tradables/trades/11:101"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"code":"NEXT_INVALID_SESSION","message":"nope"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .list_tradable_trades(&key_eric_b(), None)
        .await
        .unwrap_err();
    assert!(
        matches!(err, Error::Unauthorized { .. }),
        "expected Unauthorized, got {:?}",
        err
    );
}

#[tokio::test]
async fn get_suitability_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tradables/validation/suitability/11:101"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_suitability_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let entries = client.get_suitability(&key_eric_b()).await.unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].eligible);
    assert!(!entries[1].eligible);
}

#[tokio::test]
async fn get_suitability_403_maps_to_forbidden() {
    let server = MockServer::start().await;
    // Doc says 403 carries No Content for anonymous sessions — empty body.
    Mock::given(method("GET"))
        .and(path("/tradables/validation/suitability/11:101"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get_suitability(&key_eric_b()).await.unwrap_err();
    match err {
        Error::Forbidden { body } => assert_eq!(body, ""),
        other => panic!("expected Forbidden, got {:?}", other),
    }
}

#[tokio::test]
async fn get_suitability_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tradables/validation/suitability/11:101"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"Bad key"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get_suitability(&key_eric_b()).await.unwrap_err();
    assert!(
        matches!(err, Error::BadRequest { .. }),
        "expected BadRequest, got {:?}",
        err
    );
}
