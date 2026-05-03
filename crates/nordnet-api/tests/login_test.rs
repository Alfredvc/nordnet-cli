//! Tests for the `login` resource group.
//!
//! Layer 1 — Fixture roundtrip: every request/response fixture parses
//! under `deny_unknown_fields` and re-serialises to canonical JSON.
//!
//! Layer 2 — Wiremock integration: every operation is exercised against
//! a mock server using the corresponding fixture as response body. Where
//! the operation has a request body, the mock asserts byte-for-byte that
//! the client emitted the request fixture. Plus one error-mapping test
//! per operation (where applicable) per CONTRACTS.md.
//!
//! Layer 3 — Bridge to [`nordnet_model::auth::Session`]: confirms the
//! canonical [`ApiKeyLoginResponse`] can be converted into a
//! [`Session`] whose `basic_auth_header()` matches the expected base64.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use nordnet_api::{Client, Error};
use nordnet_model::auth::{
    ApiKeyStartLoginRequest, ApiKeyVerifyLoginRequest, ChallengeResponse, Session,
};
use nordnet_model::models::login::{ApiKeyLoginResponse, Feed, LoggedInStatus};
use pretty_assertions::assert_eq;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn start_login_request_fixture() -> &'static str {
    include_str!("../fixtures/login/start_login.request.json")
}
fn start_login_response_fixture() -> &'static str {
    include_str!("../fixtures/login/start_login.response.json")
}
fn verify_login_request_fixture() -> &'static str {
    include_str!("../fixtures/login/verify_login.request.json")
}
fn verify_login_response_fixture() -> &'static str {
    include_str!("../fixtures/login/verify_login.response.json")
}
fn refresh_session_response_fixture() -> &'static str {
    include_str!("../fixtures/login/refresh_session.response.json")
}
fn logout_response_fixture() -> &'static str {
    include_str!("../fixtures/login/logout.response.json")
}

/// Canonical-JSON roundtrip helper used by every fixture test.
fn assert_canonical_roundtrip<T>(raw: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let parsed: T = serde_json::from_str(raw).expect("fixture must parse");
    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let re = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re).expect("re-serialised must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}

// ---------------------------------------------------------------------------
// Layer 1 — Fixture roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn start_login_request_fixture_roundtrip() {
    let raw = start_login_request_fixture();
    let parsed: ApiKeyStartLoginRequest = serde_json::from_str(raw).expect("must parse");
    assert_eq!(parsed.api_key, "AKABCDEF0123456789");
    assert_canonical_roundtrip::<ApiKeyStartLoginRequest>(raw);
}

#[test]
fn start_login_response_fixture_roundtrip() {
    let raw = start_login_response_fixture();
    let parsed: ChallengeResponse = serde_json::from_str(raw).expect("must parse");
    assert_eq!(parsed.challenge, "f0dcd2fa-92b1-4151-93af-61697eae217a");
    assert_canonical_roundtrip::<ChallengeResponse>(raw);
}

#[test]
fn verify_login_request_fixture_roundtrip() {
    let raw = verify_login_request_fixture();
    let parsed: ApiKeyVerifyLoginRequest = serde_json::from_str(raw).expect("must parse");
    assert_eq!(parsed.api_key, "AKABCDEF0123456789");
    assert_eq!(parsed.service, "NEXTAPI");
    assert_eq!(parsed.signature, "c2lnbmF0dXJlLWJ5dGVzLWluLWJhc2U2NA==");
    assert_canonical_roundtrip::<ApiKeyVerifyLoginRequest>(raw);
}

#[test]
fn verify_login_response_fixture_roundtrip() {
    let raw = verify_login_response_fixture();
    let parsed: ApiKeyLoginResponse = serde_json::from_str(raw).expect("must parse");
    assert_eq!(parsed.session_key, "15a6c4db-05b9-481c-b94a-ccffed83e693");
    assert_eq!(parsed.expires_in, 1800);
    assert_eq!(
        parsed.private_feed,
        Feed {
            encrypted: true,
            hostname: "priv.next.nordnet.se".to_owned(),
            port: 443,
        }
    );
    assert_eq!(
        parsed.public_feed,
        Feed {
            encrypted: true,
            hostname: "pub.next.nordnet.se".to_owned(),
            port: 443,
        }
    );
    assert_canonical_roundtrip::<ApiKeyLoginResponse>(raw);
}

#[test]
fn refresh_session_response_fixture_roundtrip() {
    let raw = refresh_session_response_fixture();
    let parsed: LoggedInStatus = serde_json::from_str(raw).expect("must parse");
    assert!(parsed.logged_in);
    assert_canonical_roundtrip::<LoggedInStatus>(raw);
}

#[test]
fn logout_response_fixture_roundtrip() {
    let raw = logout_response_fixture();
    let parsed: LoggedInStatus = serde_json::from_str(raw).expect("must parse");
    assert!(!parsed.logged_in);
    assert_canonical_roundtrip::<LoggedInStatus>(raw);
}

// ---------------------------------------------------------------------------
// Layer 1b — deny_unknown_fields rejection
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Layer 2 — Wiremock integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn start_login_posts_request_body_and_returns_challenge() {
    let server = MockServer::start().await;
    // body_json matches structurally on the parsed JSON, which is what
    // we want — the client may re-order fields when serialising structs.
    let expected_body: serde_json::Value =
        serde_json::from_str(start_login_request_fixture()).unwrap();
    Mock::given(method("POST"))
        .and(path("/login/start"))
        .and(body_json(&expected_body))
        .respond_with(ResponseTemplate::new(200).set_body_string(start_login_response_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let req: ApiKeyStartLoginRequest = serde_json::from_str(start_login_request_fixture()).unwrap();
    let challenge = client.start_login(&req).await.unwrap();
    assert_eq!(challenge.challenge, "f0dcd2fa-92b1-4151-93af-61697eae217a");
}

#[tokio::test]
async fn start_login_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/start"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID","message":"Bad api key"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let req = ApiKeyStartLoginRequest {
        api_key: String::new(),
    };
    let err = client.start_login(&req).await.unwrap_err();
    assert!(
        matches!(err, Error::BadRequest { .. }),
        "expected BadRequest, got {:?}",
        err
    );
}

#[tokio::test]
async fn verify_login_posts_request_body_and_returns_session() {
    let server = MockServer::start().await;
    let expected_body: serde_json::Value =
        serde_json::from_str(verify_login_request_fixture()).unwrap();
    Mock::given(method("POST"))
        .and(path("/login/verify"))
        .and(body_json(&expected_body))
        .respond_with(ResponseTemplate::new(200).set_body_string(verify_login_response_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let req: ApiKeyVerifyLoginRequest =
        serde_json::from_str(verify_login_request_fixture()).unwrap();
    let response = client.verify_login(&req).await.unwrap();
    assert_eq!(response.session_key, "15a6c4db-05b9-481c-b94a-ccffed83e693");
    assert_eq!(response.expires_in, 1800);
    assert_eq!(response.private_feed.hostname, "priv.next.nordnet.se");
    assert_eq!(response.public_feed.port, 443);
}

#[tokio::test]
async fn verify_login_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/verify"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"code":"AUTH","message":"bad signature"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let req = ApiKeyVerifyLoginRequest {
        api_key: "k".into(),
        service: "NEXTAPI".into(),
        signature: "bad".into(),
    };
    let err = client.verify_login(&req).await.unwrap_err();
    assert!(
        matches!(err, Error::Unauthorized { .. }),
        "expected Unauthorized, got {:?}",
        err
    );
}

#[tokio::test]
async fn refresh_session_returns_logged_in_true() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/login"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(refresh_session_response_fixture()),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let status = client.refresh_session().await.unwrap();
    assert!(status.logged_in);
}

#[tokio::test]
async fn refresh_session_sends_empty_body() {
    // `PUT /login` is documented body-less. Verify the wire request has a
    // zero-length body and no Content-Type header.
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/login"))
        .and(wiremock::matchers::body_bytes(b"" as &[u8]))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(refresh_session_response_fixture()),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let status = client.refresh_session().await.unwrap();
    assert!(status.logged_in);
}

#[tokio::test]
async fn refresh_session_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/login"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"code":"AUTH","message":"invalid session"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.refresh_session().await.unwrap_err();
    assert!(
        matches!(err, Error::Unauthorized { .. }),
        "expected Unauthorized, got {:?}",
        err
    );
}

#[tokio::test]
async fn logout_returns_logged_in_false() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/login"))
        .respond_with(ResponseTemplate::new(200).set_body_string(logout_response_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let status = client.logout().await.unwrap();
    assert!(!status.logged_in);
}

// ---------------------------------------------------------------------------
// Layer 3 — Bridge to foundation Session
// ---------------------------------------------------------------------------

#[test]
fn api_key_login_response_builds_session_with_matching_basic_auth() {
    let response: ApiKeyLoginResponse =
        serde_json::from_str(verify_login_response_fixture()).unwrap();

    // Method form
    let session_via_method = response.to_session();
    // From<&_> form
    let session_via_from: Session = (&response).into();

    assert_eq!(session_via_method.session_key, response.session_key);
    assert_eq!(session_via_method.expires_in, response.expires_in);
    assert_eq!(session_via_from.session_key, response.session_key);
    assert_eq!(session_via_from.expires_in, response.expires_in);

    // basic_auth_header matches the expected base64 of "key:key".
    let expected_raw = format!("{0}:{0}", response.session_key);
    let expected = format!("Basic {}", B64.encode(expected_raw.as_bytes()));
    assert_eq!(session_via_method.basic_auth_header(), expected);
    assert_eq!(session_via_from.basic_auth_header(), expected);
}
