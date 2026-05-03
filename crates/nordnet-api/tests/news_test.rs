//! Tests for the `news` resource group.
//!
//! Layer 1 — Fixture roundtrip: every fixture parses under
//! `deny_unknown_fields` and re-serializes to canonical JSON.
//!
//! Layer 2 — Wiremock integration: every operation is exercised against a
//! mock server using the corresponding fixture as the response body, plus
//! at least one error-mapping test per CONTRACTS.md.

use nordnet_api::{Client, Error};
use nordnet_model::ids::{InstrumentId, MarketId};
use nordnet_model::models::news::{NewsArticle, NewsId, NewsSource, NewsSourceId};
use pretty_assertions::assert_eq;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_news_item_fixture() -> &'static str {
    include_str!("../fixtures/news/get_news_item.response.json")
}

fn list_news_sources_fixture() -> &'static str {
    include_str!("../fixtures/news/list_news_sources.response.json")
}

// ---------------------------------------------------------------------------
// Layer 1 — Fixture roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn get_news_item_fixture_roundtrip() {
    let raw = get_news_item_fixture();
    let parsed: Vec<NewsArticle> =
        serde_json::from_str(raw).expect("get_news_item fixture must parse as Vec<NewsArticle>");

    // Two entries: first fully populated, second with optionals omitted.
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].headline, "Volvo reports Q3 earnings");
    assert_eq!(parsed[0].news_id, NewsId(4567890123));
    assert_eq!(parsed[0].source_id, NewsSourceId(7));
    assert_eq!(parsed[0].news_type, "NEWS");
    assert_eq!(parsed[0].r#type, "NEWS");
    assert_eq!(parsed[0].lang, "sv");
    assert_eq!(parsed[0].markdown_format, false);
    assert_eq!(parsed[0].timestamp, 1714632000000);
    assert_eq!(parsed[0].version, 1);
    assert_eq!(
        parsed[0].instruments.as_deref(),
        Some(&[InstrumentId(101), InstrumentId(202)][..])
    );
    assert_eq!(
        parsed[0].markets.as_deref(),
        Some(&[MarketId(11), MarketId(15)][..])
    );
    assert_eq!(
        parsed[0].isin_codes.as_deref(),
        Some(&["SE0000115446".to_string(), "SE0000108656".to_string()][..])
    );
    assert_eq!(
        parsed[0].sectors.as_deref(),
        Some(&["AUTOMOTIVE".to_string(), "INDUSTRIALS".to_string()][..])
    );
    assert_eq!(parsed[0].byline.as_deref(), Some("Nordnet Newsroom"));
    assert!(parsed[0].body.is_some());
    assert!(parsed[0].summary.is_some());

    // Second entry has all optional fields omitted.
    assert_eq!(parsed[1].news_id, NewsId(4567890124));
    assert_eq!(parsed[1].news_type, "TRADING_HALT");
    assert_eq!(parsed[1].markdown_format, true);
    assert_eq!(parsed[1].body, None);
    assert_eq!(parsed[1].byline, None);
    assert_eq!(parsed[1].summary, None);
    assert_eq!(parsed[1].instruments, None);
    assert_eq!(parsed[1].markets, None);
    assert_eq!(parsed[1].isin_codes, None);
    assert_eq!(parsed[1].sectors, None);

    // Canonical roundtrip per CONTRACTS.md test rule 1.
    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let re = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re).expect("re-serialized must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}

#[test]
fn list_news_sources_fixture_roundtrip() {
    let raw = list_news_sources_fixture();
    let parsed: Vec<NewsSource> =
        serde_json::from_str(raw).expect("list_news_sources fixture must parse as Vec<NewsSource>");

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].source_id, NewsSourceId(7));
    assert_eq!(parsed[0].name, "Direkt");
    assert_eq!(parsed[0].level, "REALTIME");
    assert_eq!(
        parsed[0].countries.as_deref(),
        Some(&["SE".to_string(), "NO".to_string(), "FI".to_string()][..])
    );

    assert_eq!(parsed[1].source_id, NewsSourceId(8));
    assert_eq!(parsed[1].level, "DELAYED");
    assert_eq!(parsed[1].countries, None);

    // Canonical roundtrip per CONTRACTS.md test rule 1.
    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let re = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re).expect("re-serialized must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}
// ---------------------------------------------------------------------------
// Newtype transparency tests
// ---------------------------------------------------------------------------

#[test]
fn news_id_is_serde_transparent() {
    let v = NewsId(42);
    let s = serde_json::to_string(&v).unwrap();
    assert_eq!(s, "42");
    let back: NewsId = serde_json::from_str("42").unwrap();
    assert_eq!(back, v);
    assert_eq!(format!("{}", v), "42");
}

#[test]
fn news_source_id_is_serde_transparent() {
    let v = NewsSourceId(7);
    let s = serde_json::to_string(&v).unwrap();
    assert_eq!(s, "7");
    let back: NewsSourceId = serde_json::from_str("7").unwrap();
    assert_eq!(back, v);
    assert_eq!(format!("{}", v), "7");
}

// ---------------------------------------------------------------------------
// Layer 2 — Wiremock integration tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_news_item_returns_articles() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/news/4567890123"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_news_item_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let articles = client.get_news_item(NewsId(4567890123)).await.unwrap();

    assert_eq!(articles.len(), 2);
    assert_eq!(articles[0].headline, "Volvo reports Q3 earnings");
    assert_eq!(articles[0].news_id, NewsId(4567890123));
}

#[tokio::test]
async fn get_news_item_204_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/news/9999"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let articles = client.get_news_item(NewsId(9999)).await.unwrap();
    assert_eq!(articles, vec![], "204 No Content should return empty Vec");
}

#[tokio::test]
async fn get_news_item_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/news/0"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_ID","message":"Bad ID"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get_news_item(NewsId(0)).await.unwrap_err();
    assert!(
        matches!(err, Error::BadRequest { .. }),
        "expected BadRequest, got {:?}",
        err
    );
}

#[tokio::test]
async fn list_news_sources_returns_sources() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/news_sources"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_news_sources_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let sources = client.list_news_sources().await.unwrap();

    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0].name, "Direkt");
    assert_eq!(sources[0].level, "REALTIME");
    assert_eq!(sources[1].level, "DELAYED");
    assert_eq!(sources[1].countries, None);
}

#[tokio::test]
async fn list_news_sources_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/news_sources"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"code":"NEXT_INVALID_SESSION","message":"nope"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.list_news_sources().await.unwrap_err();
    assert!(
        matches!(err, Error::Unauthorized { .. }),
        "expected Unauthorized, got {:?}",
        err
    );
}
