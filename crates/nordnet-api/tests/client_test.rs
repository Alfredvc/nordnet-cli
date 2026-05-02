//! Wiremock-driven integration tests for the foundation HTTP client.
//!
//! Verifies:
//! - Successful GET deserializes a typed response.
//! - 400 maps to [`Error::BadRequest`] with the body preserved.
//! - 401 maps to [`Error::Unauthorized`].
//! - 429 triggers a single retry. To keep wall-clock time reasonable, we
//!   pause Tokio time so the documented 10s wait is virtual.
//! - 503 honors the `Retry-After` header and retries once.
//! - The `Authorization` header carries `Basic <base64(key:key)>`.
//! - The full `POST /login/start` -> `POST /login/verify` flow works
//!   end-to-end against wiremock and produces a usable [`Session`].

use nordnet_api::auth::{
    sign_challenge, ApiKeyLoginResponse, ApiKeyStartLoginRequest, ApiKeyVerifyLoginRequest,
    ChallengeResponse, Session,
};
use nordnet_api::{Client, Error};
use serde::{Deserialize, Serialize};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Echo {
    value: i64,
}

#[tokio::test]
async fn get_success_decodes_typed_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/echo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(Echo { value: 42 }))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let got: Echo = client.get("/echo").await.unwrap();
    assert_eq!(got, Echo { value: 42 });
}

#[tokio::test]
async fn http_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/x"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad"))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get::<Echo>("/x").await.unwrap_err();
    match err {
        Error::BadRequest { body } => assert_eq!(body, "bad"),
        other => panic!("expected BadRequest, got {other:?}"),
    }
}

#[tokio::test]
async fn http_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/x"))
        .respond_with(ResponseTemplate::new(401).set_body_string("nope"))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get::<Echo>("/x").await.unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}

#[tokio::test(start_paused = true)]
async fn http_429_retries_once_then_succeeds() {
    let server = MockServer::start().await;

    // First call: 429. Second call: 200.
    Mock::given(method("GET"))
        .and(path("/x"))
        .respond_with(ResponseTemplate::new(429).set_body_string("slow down"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/x"))
        .respond_with(ResponseTemplate::new(200).set_body_json(Echo { value: 7 }))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let got: Echo = client.get("/x").await.unwrap();
    assert_eq!(got, Echo { value: 7 });
}

#[tokio::test(start_paused = true)]
async fn http_503_honors_retry_after_then_succeeds() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/x"))
        .respond_with(
            ResponseTemplate::new(503)
                .insert_header("Retry-After", "1")
                .set_body_string("down"),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/x"))
        .respond_with(ResponseTemplate::new(200).set_body_json(Echo { value: 11 }))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let got: Echo = client.get("/x").await.unwrap();
    assert_eq!(got, Echo { value: 11 });
}

#[tokio::test]
async fn authorization_header_uses_session_key_basic() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .and(header("authorization", "Basic c2VzczpzZXNz")) // base64("sess:sess")
        .respond_with(ResponseTemplate::new(200).set_body_json(Echo { value: 1 }))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap().with_session(Session {
        session_key: "sess".into(),
        expires_in: 300,
    });
    let _: Echo = client.get("/me").await.unwrap();
}

#[tokio::test]
async fn login_flow_start_sign_verify_against_mock() {
    use nordnet_api::auth::parse_private_key_pem;
    use rsa::pkcs8::{EncodePrivateKey, LineEnding};

    // Build a deterministic key + serialize PEM so the test mirrors how
    // a real caller would load credentials from disk.
    use rand_chacha::rand_core::SeedableRng;
    let mut rng = rand_chacha::ChaCha20Rng::from_seed([3u8; 32]);
    let key = rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
    let pem = key.to_pkcs8_pem(LineEnding::LF).unwrap();
    let parsed = parse_private_key_pem(&pem).unwrap();
    let api_key = "demo-api-key";
    let challenge = "ch4ll3ng3";
    let signature = sign_challenge(&parsed, challenge).unwrap();

    let server = MockServer::start().await;

    // /login/start: assert request body, return challenge.
    Mock::given(method("POST"))
        .and(path("/login/start"))
        .and(body_json(ApiKeyStartLoginRequest {
            api_key: api_key.into(),
        }))
        .respond_with(ResponseTemplate::new(200).set_body_json(ChallengeResponse {
            challenge: challenge.into(),
        }))
        .expect(1)
        .mount(&server)
        .await;

    // /login/verify: assert request body has the computed signature, return session.
    Mock::given(method("POST"))
        .and(path("/login/verify"))
        .and(body_json(ApiKeyVerifyLoginRequest {
            api_key: api_key.into(),
            service: "NEXTAPI".into(),
            signature: signature.clone(),
        }))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ApiKeyLoginResponse {
                session_key: "sk-xyz".into(),
                expires_in: 600,
                private_feed: None,
                public_feed: None,
            }),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let challenge_resp: ChallengeResponse = client
        .post(
            "/login/start",
            &ApiKeyStartLoginRequest {
                api_key: api_key.into(),
            },
        )
        .await
        .unwrap();
    let sig = sign_challenge(&parsed, &challenge_resp.challenge).unwrap();
    let login: ApiKeyLoginResponse = client
        .post(
            "/login/verify",
            &ApiKeyVerifyLoginRequest {
                api_key: api_key.into(),
                service: "NEXTAPI".into(),
                signature: sig,
            },
        )
        .await
        .unwrap();
    assert_eq!(login.session_key, "sk-xyz");
    assert_eq!(login.expires_in, 600);

    // The session can be attached to the client and used downstream.
    let auth_client = client.with_session(Session {
        session_key: login.session_key,
        expires_in: login.expires_in,
    });
    assert!(auth_client.session().is_some());
}
