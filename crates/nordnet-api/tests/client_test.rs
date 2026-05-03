//! Wiremock-driven integration tests for the foundation HTTP client.
//!
//! Verifies:
//! - Successful GET deserializes a typed response.
//! - 400 maps to [`Error::BadRequest`] with the body preserved.
//! - 401 maps to [`Error::Unauthorized`].
//! - 429 surfaces as [`Error::TooManyRequests`] without any client-side
//!   retry (caller decides backoff policy).
//! - 503 surfaces as [`Error::ServiceUnavailable`] without any
//!   client-side retry (non-idempotent POSTs would be unsafe to retry).
//! - The `Authorization` header carries `Basic <base64(key:key)>`.
//! - The full `POST /login/start` -> `POST /login/verify` flow works
//!   end-to-end against wiremock and produces a usable [`Session`].

use nordnet_api::{Client, Error};
use nordnet_model::auth::{
    sign_challenge, ApiKeyStartLoginRequest, ApiKeyVerifyLoginRequest, ChallengeResponse, Session,
};
use nordnet_model::models::login::{ApiKeyLoginResponse, Feed};
use serde::{Deserialize, Serialize};
use wiremock::matchers::{body_json, body_string, header, method, path};
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

#[tokio::test]
async fn http_429_surfaces_without_retry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/x"))
        .respond_with(ResponseTemplate::new(429).set_body_string("slow down"))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get::<Echo>("/x").await.unwrap_err();
    match err {
        Error::TooManyRequests { body } => assert_eq!(body, "slow down"),
        other => panic!("expected TooManyRequests, got {other:?}"),
    }
}

#[tokio::test]
async fn http_503_surfaces_without_retry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/x"))
        .respond_with(
            ResponseTemplate::new(503)
                .insert_header("Retry-After", "1")
                .set_body_string("down"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get::<Echo>("/x").await.unwrap_err();
    match err {
        Error::ServiceUnavailable { body } => assert_eq!(body, "down"),
        other => panic!("expected ServiceUnavailable, got {other:?}"),
    }
}

#[tokio::test]
async fn http_503_on_post_does_not_retry() {
    // Critical: a hidden retry on a non-idempotent POST could double-place
    // an order. Assert exactly one request is sent.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/x"))
        .respond_with(ResponseTemplate::new(503).set_body_string("down"))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .post::<Echo, _>("/x", &Echo { value: 1 })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::ServiceUnavailable { .. }));
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
    use nordnet_model::auth::parse_private_key_openssh;
    use ssh_key::{private::Ed25519Keypair, LineEnding, PrivateKey};

    // Build a deterministic Ed25519 key from a fixed seed and serialize
    // it as OpenSSH PEM so the test mirrors how a real caller would
    // load credentials from disk (a file produced by `ssh-keygen`).
    let kp = Ed25519Keypair::from_seed(&[3u8; 32]);
    let pk = PrivateKey::from(kp);
    let pem = pk.to_openssh(LineEnding::LF).unwrap();
    let parsed = parse_private_key_openssh(&pem).unwrap();
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
                private_feed: Feed {
                    encrypted: true,
                    hostname: "priv.next.nordnet.se".into(),
                    port: 443,
                },
                public_feed: Feed {
                    encrypted: true,
                    hostname: "pub.next.nordnet.se".into(),
                    port: 443,
                },
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

/// Body sent by the `post_form` test below. Mirrors a Nordnet `FormData`
/// payload — note that scalar fields are urlencoded by `serde_urlencoded`
/// using `Display`, so `i64` becomes its decimal string and `&str` is
/// percent-encoded as needed.
#[derive(Debug, Serialize)]
struct FormBody<'a> {
    side: &'a str,
    market_id: i64,
    volume: i64,
    reference: &'a str,
}

#[tokio::test]
async fn post_form_sends_application_x_www_form_urlencoded() {
    let server = MockServer::start().await;
    // Catch-all so an unmatched request doesn't 404 — lets us read the
    // actual recorded request when assertions fail.
    Mock::given(method("POST"))
        .and(path("/orders"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string(
            "side=BUY&market_id=11&volume=10&reference=hello+world",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(Echo { value: 1 }))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let got: Echo = client
        .post_form(
            "/orders",
            &FormBody {
                side: "BUY",
                market_id: 11,
                volume: 10,
                reference: "hello world",
            },
        )
        .await
        .unwrap();
    assert_eq!(got, Echo { value: 1 });
}

#[tokio::test]
async fn put_form_sends_application_x_www_form_urlencoded() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/orders/42"))
        .and(header("content-type", "application/x-www-form-urlencoded"))
        .and(body_string("side=SELL&market_id=11&volume=5&reference=ok"))
        .respond_with(ResponseTemplate::new(200).set_body_json(Echo { value: 2 }))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let got: Echo = client
        .put_form(
            "/orders/42",
            &FormBody {
                side: "SELL",
                market_id: 11,
                volume: 5,
                reference: "ok",
            },
        )
        .await
        .unwrap();
    assert_eq!(got, Echo { value: 2 });
}
