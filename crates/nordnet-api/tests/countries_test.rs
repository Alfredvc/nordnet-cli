//! Tests for the `countries` resource group.
//!
//! Layer 1 — Fixture roundtrip: every fixture parses under
//! `deny_unknown_fields` and re-serializes to canonical JSON.
//!
//! Layer 2 — Wiremock integration: every operation is exercised against a
//! mock server using the corresponding fixture as the response body.

use nordnet_api::models::countries::Country;
use nordnet_api::Client;
use pretty_assertions::assert_eq;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn list_countries_fixture() -> &'static str {
    include_str!("../fixtures/countries/list_countries.response.json")
}

fn get_country_fixture() -> &'static str {
    include_str!("../fixtures/countries/get_country.response.json")
}

// ---------------------------------------------------------------------------
// Layer 1 — Fixture roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn list_countries_fixture_roundtrip() {
    let raw = list_countries_fixture();
    let parsed: Vec<Country> = serde_json::from_str(raw).expect("fixture should parse");

    assert_eq!(parsed.len(), 4);
    assert_eq!(parsed[0].country, "SE");
    assert_eq!(parsed[0].name, "Sweden");
    assert_eq!(parsed[1].country, "NO");
    assert_eq!(parsed[1].name, "Norway");
    assert_eq!(parsed[2].country, "DK");
    assert_eq!(parsed[2].name, "Denmark");
    assert_eq!(parsed[3].country, "FI");
    assert_eq!(parsed[3].name, "Finland");

    // Re-serialization must produce canonical JSON matching the fixture.
    let canonical: serde_json::Value = serde_json::from_str(raw).unwrap();
    let serialized: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap();
    assert_eq!(serialized, canonical);
}

#[test]
fn get_country_fixture_roundtrip() {
    let raw = get_country_fixture();
    let parsed: Vec<Country> = serde_json::from_str(raw).expect("fixture should parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].country, "SE");
    assert_eq!(parsed[0].name, "Sweden");

    // Re-serialization must produce canonical JSON matching the fixture.
    let canonical: serde_json::Value = serde_json::from_str(raw).unwrap();
    let serialized: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&parsed).unwrap()).unwrap();
    assert_eq!(serialized, canonical);
}

#[test]
fn country_rejects_unknown_fields() {
    let raw = r#"[{"country": "SE", "name": "Sweden", "extra": "oops"}]"#;
    let result: Result<Vec<Country>, _> = serde_json::from_str(raw);
    assert!(
        result.is_err(),
        "deny_unknown_fields should reject extra fields"
    );
}

// ---------------------------------------------------------------------------
// Layer 2 — Wiremock integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_countries_returns_all_countries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/countries"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_countries_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let countries = client.list_countries().await.unwrap();

    assert_eq!(countries.len(), 4);
    assert_eq!(countries[0].country, "SE");
    assert_eq!(countries[0].name, "Sweden");
    assert_eq!(countries[3].country, "FI");
    assert_eq!(countries[3].name, "Finland");
}

#[tokio::test]
async fn get_country_single_code_returns_matching_country() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/countries/SE"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_country_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let countries = client.get_country("SE").await.unwrap();

    assert_eq!(countries.len(), 1);
    assert_eq!(countries[0].country, "SE");
    assert_eq!(countries[0].name, "Sweden");
}

#[tokio::test]
async fn get_country_comma_separated_codes_returns_multiple() {
    let multi_fixture = r#"[{"country":"SE","name":"Sweden"},{"country":"NO","name":"Norway"}]"#;

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/countries/SE,NO"))
        .respond_with(ResponseTemplate::new(200).set_body_string(multi_fixture))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let countries = client.get_country("SE,NO").await.unwrap();

    assert_eq!(countries.len(), 2);
    assert_eq!(countries[0].country, "SE");
    assert_eq!(countries[1].country, "NO");
}

#[tokio::test]
async fn get_country_204_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/countries/XX"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let countries = client.get_country("XX").await.unwrap();

    assert_eq!(countries, vec![], "204 No Content should return empty Vec");
}
