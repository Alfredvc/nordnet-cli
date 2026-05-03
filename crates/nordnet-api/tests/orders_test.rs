//! Tests for the `orders` resource group.
//!
//! Layer 1 — Fixture roundtrip: every request/response fixture parses
//! under `deny_unknown_fields` and re-serialises to canonical JSON.
//!
//! Layer 2 — Wiremock integration: every operation is exercised against
//! a mock server using the corresponding fixture as response body. For
//! the FormData write ops (`place_order`, `modify_order`) the mock
//! additionally asserts the wire request carries
//! `Content-Type: application/x-www-form-urlencoded` and a body matching
//! the urlencoded form derived from the request fixture (constants
//! `PLACE_ORDER_EXPECTED_FORM_BODY` / `MODIFY_ORDER_EXPECTED_FORM_BODY`).
//! Plus error-mapping tests for the documented status codes (400, 401,
//! 403).

use nordnet_api::{Client, Error};
use nordnet_model::ids::{AccountId, OrderId};
use nordnet_model::models::orders::{
    ModifyOrderRequest, Order, OrderReply, OrderSide, OrderType, PlaceOrderRequest,
};
use pretty_assertions::assert_eq;
use wiremock::matchers::{body_string, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Expected wire body for the `place_order` write op. Reflects the
/// fixture in `fixtures/orders/place_order.request.json` re-encoded as
/// `application/x-www-form-urlencoded`. Field order matches the
/// declaration order on [`PlaceOrderRequest`] (and the doc parameter
/// table) — `serde_urlencoded` honors struct-field order. Spaces would
/// be encoded as `+` (no spaces in this fixture); reserved chars are
/// percent-encoded. `valid_until` arrives as `2025-12-31` because `-`
/// is unreserved in form-urlencoded.
const PLACE_ORDER_EXPECTED_FORM_BODY: &str =
    "currency=SEK&identifier=101&market_id=11&order_type=LIMIT&price=101.5&side=BUY&valid_until=2025-12-31&volume=100";

/// Expected wire body for the `modify_order` write op. See
/// [`PLACE_ORDER_EXPECTED_FORM_BODY`].
const MODIFY_ORDER_EXPECTED_FORM_BODY: &str = "currency=SEK&price=102.25&volume=150";

const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";

// ---------------------------------------------------------------------------
// Fixture readers
// ---------------------------------------------------------------------------

fn list_orders_response_fixture() -> &'static str {
    include_str!("../fixtures/orders/list_orders.response.json")
}
fn place_order_request_fixture() -> &'static str {
    include_str!("../fixtures/orders/place_order.request.json")
}
fn place_order_response_fixture() -> &'static str {
    include_str!("../fixtures/orders/place_order.response.json")
}
fn modify_order_request_fixture() -> &'static str {
    include_str!("../fixtures/orders/modify_order.request.json")
}
fn modify_order_response_fixture() -> &'static str {
    include_str!("../fixtures/orders/modify_order.response.json")
}
fn activate_order_response_fixture() -> &'static str {
    include_str!("../fixtures/orders/activate_order.response.json")
}
fn cancel_order_response_fixture() -> &'static str {
    include_str!("../fixtures/orders/cancel_order.response.json")
}

/// Canonical-JSON roundtrip helper.
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
// Layer 1 — Fixture roundtrip
// ---------------------------------------------------------------------------

#[test]
fn list_orders_response_fixture_roundtrip() {
    let raw = list_orders_response_fixture();
    let parsed: Vec<Order> = serde_json::from_str(raw).expect("must parse");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].order_id, OrderId(555));
    assert_eq!(parsed[0].accno, 987654);
    assert_canonical_roundtrip::<Vec<Order>>(raw);
}

#[test]
fn place_order_request_fixture_roundtrip() {
    let raw = place_order_request_fixture();
    let parsed: PlaceOrderRequest = serde_json::from_str(raw).expect("must parse");
    assert!(matches!(parsed.side, OrderSide::Buy));
    assert!(matches!(parsed.order_type, Some(OrderType::Limit)));
    assert_eq!(parsed.volume, 100);
    assert_canonical_roundtrip::<PlaceOrderRequest>(raw);
}

#[test]
fn place_order_response_fixture_roundtrip() {
    let raw = place_order_response_fixture();
    let parsed: OrderReply = serde_json::from_str(raw).expect("must parse");
    assert_eq!(parsed.order_id, OrderId(555));
    assert_eq!(parsed.result_code, "OK");
    assert_canonical_roundtrip::<OrderReply>(raw);
}

#[test]
fn modify_order_request_fixture_roundtrip() {
    let raw = modify_order_request_fixture();
    let parsed: ModifyOrderRequest = serde_json::from_str(raw).expect("must parse");
    assert_eq!(parsed.volume, Some(150));
    assert_canonical_roundtrip::<ModifyOrderRequest>(raw);
}

#[test]
fn modify_order_response_fixture_roundtrip() {
    let raw = modify_order_response_fixture();
    let parsed: OrderReply = serde_json::from_str(raw).expect("must parse");
    assert_eq!(parsed.order_id, OrderId(555));
    assert_canonical_roundtrip::<OrderReply>(raw);
}

#[test]
fn activate_order_response_fixture_roundtrip() {
    let raw = activate_order_response_fixture();
    let parsed: Vec<OrderReply> = serde_json::from_str(raw).expect("must parse");
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].order_id, OrderId(555));
    assert_canonical_roundtrip::<Vec<OrderReply>>(raw);
}

#[test]
fn cancel_order_response_fixture_roundtrip() {
    let raw = cancel_order_response_fixture();
    let parsed: OrderReply = serde_json::from_str(raw).expect("must parse");
    assert_eq!(parsed.order_id, OrderId(555));
    assert_eq!(parsed.result_code, "OK");
    assert_canonical_roundtrip::<OrderReply>(raw);
}

// ---------------------------------------------------------------------------
// Layer 1b — deny_unknown_fields rejection
// ---------------------------------------------------------------------------
#[test]
fn place_order_request_rejects_unknown_fields() {
    let raw = r#"{"market_id":11,"side":"BUY","volume":1,"oops":true}"#;
    let r: Result<PlaceOrderRequest, _> = serde_json::from_str(raw);
    assert!(r.is_err(), "PlaceOrderRequest must deny unknown fields");
}

// ---------------------------------------------------------------------------
// Layer 2 — Wiremock integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_orders_returns_orders() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1234/orders"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_orders_response_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let orders = client.list_orders(AccountId(1234), None).await.unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].order_id, OrderId(555));
}

#[tokio::test]
async fn list_orders_passes_deleted_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1234/orders"))
        .and(wiremock::matchers::query_param("deleted", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_orders_response_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let orders = client
        .list_orders(AccountId(1234), Some(true))
        .await
        .unwrap();
    assert_eq!(orders.len(), 1);
}

#[tokio::test]
async fn list_orders_204_maps_to_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1234/orders"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let orders = client.list_orders(AccountId(1234), None).await.unwrap();
    assert!(orders.is_empty(), "204 must map to empty Vec");
}

#[tokio::test]
async fn list_orders_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1234/orders"))
        .respond_with(
            ResponseTemplate::new(401).set_body_string(r#"{"code":"AUTH","message":"nope"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.list_orders(AccountId(1234), None).await.unwrap_err();
    assert!(
        matches!(err, Error::Unauthorized { .. }),
        "expected Unauthorized, got {:?}",
        err
    );
}

#[tokio::test]
async fn list_orders_403_maps_to_forbidden() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1234/orders"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.list_orders(AccountId(1234), None).await.unwrap_err();
    assert!(
        matches!(err, Error::Forbidden { .. }),
        "expected Forbidden, got {:?}",
        err
    );
}

#[tokio::test]
async fn place_order_posts_request_body_and_returns_reply() {
    // Verifies both:
    //   - Content-Type header is application/x-www-form-urlencoded
    //     (Swagger 2.0 `FormData` per the doc parameter table).
    //   - Wire body matches the urlencoded form derived from the
    //     request fixture (with declaration-order field ordering).
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/1234/orders"))
        .and(header("content-type", FORM_CONTENT_TYPE))
        .and(body_string(PLACE_ORDER_EXPECTED_FORM_BODY))
        .respond_with(ResponseTemplate::new(200).set_body_string(place_order_response_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let req: PlaceOrderRequest = serde_json::from_str(place_order_request_fixture()).unwrap();
    let reply = client.place_order(AccountId(1234), &req).await.unwrap();
    assert_eq!(reply.order_id, OrderId(555));
    assert_eq!(reply.result_code, "OK");
}

#[tokio::test]
async fn place_order_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/1234/orders"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"BAD","message":"invalid params"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let req: PlaceOrderRequest = serde_json::from_str(place_order_request_fixture()).unwrap();
    let err = client.place_order(AccountId(1234), &req).await.unwrap_err();
    assert!(
        matches!(err, Error::BadRequest { .. }),
        "expected BadRequest, got {:?}",
        err
    );
}

#[tokio::test]
async fn modify_order_puts_request_body_and_returns_reply() {
    // See `place_order_posts_request_body_and_returns_reply` — same
    // FormData wire format applies.
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/accounts/1234/orders/555"))
        .and(header("content-type", FORM_CONTENT_TYPE))
        .and(body_string(MODIFY_ORDER_EXPECTED_FORM_BODY))
        .respond_with(ResponseTemplate::new(200).set_body_string(modify_order_response_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let req: ModifyOrderRequest = serde_json::from_str(modify_order_request_fixture()).unwrap();
    let reply = client
        .modify_order(AccountId(1234), OrderId(555), &req)
        .await
        .unwrap();
    assert_eq!(reply.order_id, OrderId(555));
    assert_eq!(reply.result_code, "OK");
}

#[tokio::test]
async fn modify_order_403_maps_to_forbidden() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/accounts/1234/orders/555"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let req: ModifyOrderRequest = serde_json::from_str(modify_order_request_fixture()).unwrap();
    let err = client
        .modify_order(AccountId(1234), OrderId(555), &req)
        .await
        .unwrap_err();
    assert!(
        matches!(err, Error::Forbidden { .. }),
        "expected Forbidden, got {:?}",
        err
    );
}

#[tokio::test]
async fn activate_order_returns_reply_array() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/accounts/1234/orders/555/activate"))
        .respond_with(ResponseTemplate::new(200).set_body_string(activate_order_response_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let replies = client
        .activate_order(AccountId(1234), OrderId(555))
        .await
        .unwrap();
    assert_eq!(replies.len(), 1);
    assert_eq!(replies[0].order_id, OrderId(555));
}

#[tokio::test]
async fn activate_order_sends_empty_body() {
    // `PUT .../activate` is documented body-less. Verify the wire request
    // has a zero-length body.
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/accounts/1234/orders/555/activate"))
        .and(wiremock::matchers::body_bytes(b"" as &[u8]))
        .respond_with(ResponseTemplate::new(200).set_body_string(activate_order_response_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let replies = client
        .activate_order(AccountId(1234), OrderId(555))
        .await
        .unwrap();
    assert_eq!(replies.len(), 1);
}

#[tokio::test]
async fn activate_order_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/accounts/1234/orders/555/activate"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"BAD","message":"already active"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .activate_order(AccountId(1234), OrderId(555))
        .await
        .unwrap_err();
    assert!(
        matches!(err, Error::BadRequest { .. }),
        "expected BadRequest, got {:?}",
        err
    );
}

#[tokio::test]
async fn cancel_order_returns_reply() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/accounts/1234/orders/555"))
        .respond_with(ResponseTemplate::new(200).set_body_string(cancel_order_response_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let reply = client
        .cancel_order(AccountId(1234), OrderId(555))
        .await
        .unwrap();
    assert_eq!(reply.order_id, OrderId(555));
    assert_eq!(reply.result_code, "OK");
}

#[tokio::test]
async fn cancel_order_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/accounts/1234/orders/555"))
        .respond_with(
            ResponseTemplate::new(401).set_body_string(r#"{"code":"AUTH","message":"no"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .cancel_order(AccountId(1234), OrderId(555))
        .await
        .unwrap_err();
    assert!(
        matches!(err, Error::Unauthorized { .. }),
        "expected Unauthorized, got {:?}",
        err
    );
}
