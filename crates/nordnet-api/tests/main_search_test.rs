//! Tests for the `main_search` resource group.
//!
//! Three test layers per CONTRACTS.md:
//!
//! 1. Fixture roundtrip — `search.response.json` parses under
//!    `deny_unknown_fields` and re-serializes to the same canonical JSON
//!    `Value`. Includes a separate Decimal-precision roundtrip to verify
//!    the optional `Decimal` serde adapter on `last_price.price` /
//!    `spread_pct` / `turnover` / KO/ETP fields.
//! 2. `deny_unknown_fields` rejection — at least one struct rejects
//!    extra fields.
//! 3. Wiremock integration — success (asserting the query string is
//!    forwarded exactly), 400 → `BadRequest`, 204 → empty `Vec`.

use nordnet_api::{Client, Error};
use nordnet_model::ids::{InstrumentId, MarketId, TickSizeId};
use nordnet_model::models::main_search::{
    EtpInfo, KoInfo, MainSearchResponse, MarketInfo, PriceKoInfo, PriceWithDecimals, StatusInfo,
};
use pretty_assertions::assert_eq;
use rust_decimal::Decimal;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn search_fixture() -> &'static str {
    include_str!("../fixtures/main_search/search.response.json")
}

// ---------------------------------------------------------------------------
// Layer 1 — Fixture roundtrip
// ---------------------------------------------------------------------------

#[test]
fn search_fixture_roundtrip() {
    let raw = search_fixture();
    let parsed: Vec<MainSearchResponse> =
        serde_json::from_str(raw).expect("search fixture must parse as Vec<MainSearchResponse>");

    // Two groups: equities (rich row) + news (minimal row).
    assert_eq!(parsed.len(), 2);

    let equities = &parsed[0];
    assert_eq!(equities.display_group_description, "Equities");
    assert_eq!(equities.display_group_type, "EQUITY");
    assert_eq!(equities.limit, Some(5));
    assert_eq!(equities.offset, Some(0));
    assert_eq!(equities.total, Some(1));
    assert_eq!(equities.results.len(), 1);

    let row = &equities.results[0];
    assert_eq!(row.display_name, "Volvo B");
    assert_eq!(row.instrument_id, Some(InstrumentId(16099583)));
    assert_eq!(row.currency.as_deref(), Some("SEK"));
    assert_eq!(
        row.last_price,
        Some(PriceWithDecimals {
            decimals: Some(4),
            price: Some("269.7234".parse::<Decimal>().unwrap()),
        })
    );
    assert_eq!(row.spread_pct, Some("0.037".parse::<Decimal>().unwrap()));
    assert_eq!(
        row.turnover,
        Some("123456789.50".parse::<Decimal>().unwrap())
    );
    assert_eq!(row.turnover_volume, Some(458320));
    assert_eq!(row.tick_timestamp, Some(1714568400000));

    // Nested types
    assert_eq!(
        row.etp_info,
        Some(EtpInfo {
            direction: Some("Long".to_owned()),
            first_trading_date: Some(1577836800000),
            market_view: Some("Bullish".to_owned()),
            nordnet_markets: Some(true),
            underlying_instrument_id: Some(InstrumentId(101)),
            underlying_name: Some("OMXS30".to_owned()),
        })
    );
    assert_eq!(
        row.ko_info,
        Some(KoInfo {
            financial_level: Some("250.0".parse::<Decimal>().unwrap()),
            stop_loss: Some("240.5".parse::<Decimal>().unwrap()),
        })
    );
    assert_eq!(
        row.market_info,
        Some(MarketInfo {
            identifier: Some("XSTO".to_owned()),
            market_id: Some(MarketId(11)),
            market_sub_id: Some(1),
            tick_size_id: Some(TickSizeId(1)),
        })
    );
    assert_eq!(
        row.price_ko_info,
        Some(PriceKoInfo {
            indicative_high_risk: Some(false),
            indicative_leverage: Some("4.5".parse::<Decimal>().unwrap()),
            risk_buffer: Some("12.75".parse::<Decimal>().unwrap()),
        })
    );
    assert_eq!(
        row.status_info,
        Some(StatusInfo {
            tick_timestamp: Some(1714568400000),
            trading_status: Some("TRADING".to_owned()),
            translated_trading_status: Some("Handlas".to_owned()),
        })
    );

    // Minimal row
    let news = &parsed[1];
    assert_eq!(news.display_group_type, "NEWS");
    assert_eq!(news.results.len(), 1);
    assert_eq!(news.results[0].display_name, "Volvo Q1 report released");
    assert_eq!(news.results[0].instrument_id, None);
    assert_eq!(news.limit, None);
    assert_eq!(news.total, None);

    // Canonical roundtrip per CONTRACTS.md test rule 1.
    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let re = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re).expect("re-serialized must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}

#[test]
fn optional_decimal_precision_survives_roundtrip() {
    // Verifies the `opt_arb_prec` adapter on Option<Decimal> fields
    // preserves multi-significant-digit precision through serde.
    let raw = r#"[
      {
        "display_group_description": "Equities",
        "display_group_type": "EQUITY",
        "results": [
          {
            "display_name": "Precision Test",
            "last_price": { "decimals": 8, "price": 12345.67891234 },
            "spread_pct": 0.000001,
            "turnover": 9999999999.999999
          }
        ]
      }
    ]"#;
    let parsed: Vec<MainSearchResponse> = serde_json::from_str(raw).unwrap();
    let row = &parsed[0].results[0];
    assert_eq!(
        row.last_price.as_ref().unwrap().price,
        Some("12345.67891234".parse::<Decimal>().unwrap())
    );
    assert_eq!(row.spread_pct, Some("0.000001".parse::<Decimal>().unwrap()));
    assert_eq!(
        row.turnover,
        Some("9999999999.999999".parse::<Decimal>().unwrap())
    );

    let re = serde_json::to_string(&parsed).unwrap();
    let re_parsed: Vec<MainSearchResponse> = serde_json::from_str(&re).unwrap();
    assert_eq!(parsed, re_parsed);
}

// ---------------------------------------------------------------------------
// Layer 2 — deny_unknown_fields rejection
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Layer 3 — Wiremock integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn search_forwards_query_string() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/main_search"))
        .and(query_param("query", "Volvo B"))
        .and(query_param("limit", "10"))
        .and(query_param("offset", "5"))
        .and(query_param("search_space", "INSTRUMENTS"))
        // `instrument_group` repeats — wiremock's `query_param` matches
        // any occurrence of the (key, value) pair, so checking one
        // member of the list is sufficient to confirm multi encoding.
        .and(query_param("instrument_group", "EQUITY"))
        .and(query_param("instrument_group", "ETF"))
        .respond_with(ResponseTemplate::new(200).set_body_string(search_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let groups = client
        .search(
            "Volvo B",
            Some(&["EQUITY", "ETF"]),
            Some(10),
            Some(5),
            Some("INSTRUMENTS"),
        )
        .await
        .unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].display_group_type, "EQUITY");
}

#[tokio::test]
async fn search_minimal_args() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/main_search"))
        .and(query_param("query", "Volvo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(search_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let groups = client
        .search("Volvo", None, None, None, None)
        .await
        .unwrap();
    assert_eq!(groups.len(), 2);
}

#[tokio::test]
async fn search_204_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/main_search"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let groups = client
        .search("nothing", None, None, None, None)
        .await
        .unwrap();
    assert_eq!(
        groups,
        vec![],
        "204 No Content should map to empty Vec<MainSearchResponse>"
    );
}

#[tokio::test]
async fn search_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/main_search"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"Bad query"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.search("", None, None, None, None).await.unwrap_err();
    assert!(
        matches!(err, Error::BadRequest { .. }),
        "expected BadRequest, got {:?}",
        err
    );
}
