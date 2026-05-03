//! Tests for the `root` resource group.
//!
//! Covers:
//! - Fixture round-trip: `serde_json::from_str` succeeds and re-serializes
//!   byte-equivalent.
//! - Wiremock integration: mock `GET /` returns the fixture; `get_system_status`
//!   deserializes the response and returns the expected [`Status`].

use nordnet_api::Client;
use nordnet_model::models::root::Status;
use pretty_assertions::assert_eq;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FIXTURE: &str = include_str!("../fixtures/root/get_system_status.response.json");

// ---------------------------------------------------------------------------
// Fixture round-trip
// ---------------------------------------------------------------------------

#[test]
fn fixture_round_trip() {
    let parsed: Status = serde_json::from_str(FIXTURE).expect("fixture should parse into Status");

    assert_eq!(parsed.message, "System is up and running.");
    assert!(parsed.system_running);
    assert_eq!(parsed.timestamp, 1_746_172_800_000_i64);
    assert!(parsed.valid_version);

    // Re-serialize and compare JSON values (not raw strings) for canonical
    // equivalence regardless of key order.
    let re_serialized = serde_json::to_string(&parsed).expect("Status should serialize");
    let original_val: serde_json::Value =
        serde_json::from_str(FIXTURE).expect("fixture is valid JSON");
    let round_trip_val: serde_json::Value =
        serde_json::from_str(&re_serialized).expect("re-serialized value is valid JSON");
    assert_eq!(original_val, round_trip_val);
}
// ---------------------------------------------------------------------------
// Wiremock integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_system_status_returns_status() {
    let server = MockServer::start().await;

    // The client is constructed with server.uri() as base URL (no path suffix).
    // Client::get("") calls `base_url + "/" + "" = base_url/`, which matches
    // wiremock path "/".
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIXTURE))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let status = client.get_system_status().await.unwrap();

    assert_eq!(status.message, "System is up and running.");
    assert!(status.system_running);
    assert_eq!(status.timestamp, 1_746_172_800_000_i64);
    assert!(status.valid_version);
}
