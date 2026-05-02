//! Tests for the `tick_sizes` resource group.
//!
//! Two test layers per CONTRACTS.md:
//! 1. Fixture roundtrip — every fixture parses and re-serializes identically.
//! 2. Wiremock integration — every op is called against a mock server.

use nordnet_api::ids::TickSizeId;
use nordnet_api::models::tick_sizes::TicksizeTable;
use nordnet_api::{Client, Error};
use pretty_assertions::assert_eq;
use rust_decimal::Decimal;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn list_fixture() -> &'static str {
    include_str!("../fixtures/tick_sizes/list_tick_sizes.response.json")
}

fn get_fixture() -> &'static str {
    include_str!("../fixtures/tick_sizes/get_tick_size.response.json")
}

// ---------------------------------------------------------------------------
// Layer 1 — Fixture roundtrips
// ---------------------------------------------------------------------------

#[test]
fn list_tick_sizes_fixture_roundtrip() {
    let raw = list_fixture();
    let parsed: Vec<TicksizeTable> = serde_json::from_str(raw)
        .expect("list_tick_sizes fixture must parse as Vec<TicksizeTable>");

    // Must have 2 tables in the list fixture.
    assert_eq!(parsed.len(), 2);

    // First table
    let t0 = &parsed[0];
    assert_eq!(t0.tick_size_id, TickSizeId(1));
    assert_eq!(t0.ticks.len(), 3);
    assert_eq!(t0.ticks[0].decimals, 2);
    assert_eq!(t0.ticks[0].from_price, "0.00".parse::<Decimal>().unwrap());
    assert_eq!(t0.ticks[0].tick, "0.01".parse::<Decimal>().unwrap());
    assert_eq!(t0.ticks[0].to_price, "1.00".parse::<Decimal>().unwrap());

    // Canonical roundtrip: re-serialized form must equal the original fixture
    // when both are parsed as serde_json::Value (catches Decimal-as-string vs
    // Decimal-as-number asymmetry per CONTRACTS.md test rule 1).
    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let re_serialized = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re_serialized).expect("re-serialized must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}

#[test]
fn get_tick_size_fixture_roundtrip() {
    let raw = get_fixture();
    let parsed: Vec<TicksizeTable> =
        serde_json::from_str(raw).expect("get_tick_size fixture must parse as Vec<TicksizeTable>");

    // Single-ID get fixture has one table.
    assert_eq!(parsed.len(), 1);

    let t = &parsed[0];
    assert_eq!(t.tick_size_id, TickSizeId(1));
    assert_eq!(t.ticks.len(), 3);

    // Check last interval
    let last = &t.ticks[2];
    assert_eq!(last.decimals, 1);
    assert_eq!(last.from_price, "10.00".parse::<Decimal>().unwrap());
    assert_eq!(last.tick, "0.10".parse::<Decimal>().unwrap());
    assert_eq!(last.to_price, "100.00".parse::<Decimal>().unwrap());

    // Canonical roundtrip per CONTRACTS.md test rule 1.
    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let re_serialized = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re_serialized).expect("re-serialized must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}
#[test]
fn decimal_precision_survives_roundtrip() {
    // Verify that a Decimal with many significant digits is not corrupted.
    let raw = r#"[
      {
        "tick_size_id": 3,
        "ticks": [
          {
            "decimals": 6,
            "from_price": 0.000001,
            "tick": 0.000001,
            "to_price": 0.999999
          }
        ]
      }
    ]"#;
    let parsed: Vec<TicksizeTable> = serde_json::from_str(raw).unwrap();
    let interval = &parsed[0].ticks[0];
    assert_eq!(interval.from_price, "0.000001".parse::<Decimal>().unwrap());
    assert_eq!(interval.tick, "0.000001".parse::<Decimal>().unwrap());
    assert_eq!(interval.to_price, "0.999999".parse::<Decimal>().unwrap());

    let re = serde_json::to_string(&parsed).unwrap();
    let re_parsed: Vec<TicksizeTable> = serde_json::from_str(&re).unwrap();
    assert_eq!(parsed, re_parsed);
}

// ---------------------------------------------------------------------------
// Layer 2 — Wiremock integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_tick_sizes_integration() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tick_sizes"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let tables = client.list_tick_sizes().await.unwrap();

    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0].tick_size_id, TickSizeId(1));
    assert_eq!(tables[1].tick_size_id, TickSizeId(2));
    assert_eq!(tables[0].ticks.len(), 3);
    assert_eq!(tables[1].ticks.len(), 2);
}

#[tokio::test]
async fn get_tick_size_integration() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tick_sizes/1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let tables = client.get_tick_size(TickSizeId(1)).await.unwrap();

    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].tick_size_id, TickSizeId(1));
    assert_eq!(tables[0].ticks.len(), 3);
    assert_eq!(tables[0].ticks[0].tick, "0.01".parse::<Decimal>().unwrap());
}

#[tokio::test]
async fn get_tick_size_url_uses_id() {
    // Ensure a different ID produces a different URL path.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tick_sizes/42"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let tables = client.get_tick_size(TickSizeId(42)).await.unwrap();
    assert_eq!(tables.len(), 1);
}

#[tokio::test]
async fn get_tick_size_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tick_sizes/0"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_ID","message":"Bad ID"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get_tick_size(TickSizeId(0)).await.unwrap_err();
    assert!(
        matches!(err, Error::BadRequest { .. }),
        "expected BadRequest, got {:?}",
        err
    );
}
