//! Tests for the `markets` resource group.
//! Layer 1 — Fixture roundtrip: every fixture parses under
//! `deny_unknown_fields` and re-serializes to canonical JSON.
//! Layer 2 — Wiremock integration: every operation is exercised against a
//! mock server using the corresponding fixture as the response body, plus
//! one error-mapping test.

use nordnet_api::{Client, Error};
use nordnet_model::ids::MarketId;
use nordnet_model::models::markets::Market;
use pretty_assertions::assert_eq;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn list_markets_fixture() -> &'static str {
    include_str!("../fixtures/markets/list_markets.response.json")
}

fn get_market_fixture() -> &'static str {
    include_str!("../fixtures/markets/get_market.response.json")
}

// ---------------------------------------------------------------------------
// Layer 1 — Fixture roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn list_markets_fixture_roundtrip() {
    let raw = list_markets_fixture();
    let parsed: Vec<Market> =
        serde_json::from_str(raw).expect("list_markets fixture must parse as Vec<Market>");

    // Structural assertions: at least one market with country present, one
    // with country omitted (exercises the optional field).
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].market_id, MarketId(11));
    assert_eq!(parsed[0].name, "Stockholmsbörsen");
    assert_eq!(parsed[0].country.as_deref(), Some("SE"));
    assert_eq!(parsed[2].market_id, MarketId(80));
    assert_eq!(parsed[2].country, None);

    // Canonical roundtrip test rule 1.
    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let re = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re).expect("re-serialized must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}

#[test]
fn get_market_fixture_roundtrip() {
    let raw = get_market_fixture();
    let parsed: Vec<Market> =
        serde_json::from_str(raw).expect("get_market fixture must parse as Vec<Market>");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].market_id, MarketId(11));
    assert_eq!(parsed[0].name, "Stockholmsbörsen");
    assert_eq!(parsed[0].country.as_deref(), Some("SE"));

    // Canonical roundtrip test rule 1.
    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let re = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re).expect("re-serialized must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}
// ---------------------------------------------------------------------------
// Layer 2 — Wiremock integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_markets_returns_all_markets() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/markets"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_markets_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let markets = client.list_markets().await.unwrap();

    assert_eq!(markets.len(), 3);
    assert_eq!(markets[0].market_id, MarketId(11));
    assert_eq!(markets[0].country.as_deref(), Some("SE"));
    assert_eq!(markets[2].market_id, MarketId(80));
    assert_eq!(markets[2].country, None);
}

#[tokio::test]
async fn get_market_returns_matching_market() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/markets/11"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_market_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let markets = client.get_market(MarketId(11)).await.unwrap();

    assert_eq!(markets.len(), 1);
    assert_eq!(markets[0].market_id, MarketId(11));
    assert_eq!(markets[0].name, "Stockholmsbörsen");
}

#[tokio::test]
async fn get_market_url_uses_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/markets/42"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_market_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let markets = client.get_market(MarketId(42)).await.unwrap();
    assert_eq!(markets.len(), 1);
}

#[tokio::test]
async fn get_market_204_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/markets/9999"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let markets = client.get_market(MarketId(9999)).await.unwrap();
    assert_eq!(markets, vec![], "204 No Content should return empty Vec");
}

#[tokio::test]
async fn get_market_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/markets/0"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_ID","message":"Bad ID"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get_market(MarketId(0)).await.unwrap_err();
    assert!(
        matches!(err, Error::BadRequest { .. }),
        "expected BadRequest, got {:?}",
        err
    );
}
