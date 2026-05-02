//! Tests for the `instruments` resource group.
//!
//! Three test layers per CONTRACTS.md:
//!
//! 1. Fixture roundtrip — every fixture parses under `deny_unknown_fields`
//!    and re-serializes to the same canonical JSON `Value`. Plus a Decimal
//!    precision survival test on `Instrument::leverage_percentage`, an
//!    `IssuerId` (now in `crate::ids`) transparency test, and a roundtrip that verifies the
//!    misspelled legacy `instrumment_id` field is preserved on the wire.
//! 2. `deny_unknown_fields` rejection — covers `Instrument`,
//!    `InstrumentType`, `InstrumentEligibility`, and `LeverageFilter`.
//! 3. Wiremock integration — every op exercised with success + at least
//!    one error mapping. `get_instrument_suitability` additionally tests
//!    the documented 403 (anonymous session, empty body).
//!    `list_leverages` asserts the `LeveragesQuery` is forwarded as a
//!    query string both when populated and when default.
//!    `get_instrument` covers the 204 No Content -> empty Vec mapping.

use nordnet_api::ids::{InstrumentId, IssuerId, MarketId, TickSizeId, TradableId};
use nordnet_api::models::instruments::{
    Instrument, InstrumentEligibility, InstrumentPublicTrades, InstrumentType,
    KeyInformationDocuments, LeverageFilter, Tradable, UnderlyingInfo,
};
use nordnet_api::resources::instruments::LeveragesQuery;
use nordnet_api::{Client, Error};
use pretty_assertions::assert_eq;
use rust_decimal::Decimal;
use time::macros::date;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn lookup_fixture() -> &'static str {
    include_str!("../fixtures/instruments/lookup.response.json")
}

fn list_types_fixture() -> &'static str {
    include_str!("../fixtures/instruments/list_types.response.json")
}

fn get_type_fixture() -> &'static str {
    include_str!("../fixtures/instruments/get_type.response.json")
}

fn list_underlyings_fixture() -> &'static str {
    include_str!("../fixtures/instruments/list_underlyings.response.json")
}

fn get_instrument_suitability_fixture() -> &'static str {
    include_str!("../fixtures/instruments/get_instrument_suitability.response.json")
}

fn get_instrument_fixture() -> &'static str {
    include_str!("../fixtures/instruments/get_instrument.response.json")
}

fn list_leverages_fixture() -> &'static str {
    include_str!("../fixtures/instruments/list_leverages.response.json")
}

fn get_leverage_filters_fixture() -> &'static str {
    include_str!("../fixtures/instruments/get_leverage_filters.response.json")
}

fn list_instrument_trades_fixture() -> &'static str {
    include_str!("../fixtures/instruments/list_instrument_trades.response.json")
}

const ERIC_B: InstrumentId = InstrumentId(16099583);

// ---------------------------------------------------------------------------
// Helper: assert a fixture re-serializes to the same canonical JSON Value.
// ---------------------------------------------------------------------------

fn assert_canonical_roundtrip<T>(raw: &str)
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let canonical: serde_json::Value =
        serde_json::from_str(raw).expect("fixture must parse as Value");
    let parsed: T = serde_json::from_str(raw).expect("fixture must parse as typed T");
    let re = serde_json::to_string(&parsed).expect("must re-serialize");
    let re_canonical: serde_json::Value =
        serde_json::from_str(&re).expect("re-serialized must parse as Value");
    assert_eq!(canonical, re_canonical, "canonical roundtrip mismatch");
}

// ---------------------------------------------------------------------------
// Layer 1 — Fixture roundtrip
// ---------------------------------------------------------------------------

#[test]
fn lookup_fixture_roundtrip() {
    let raw = lookup_fixture();
    let parsed: Vec<Instrument> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].instrument_id, ERIC_B);
    assert_eq!(parsed[0].symbol, "ERIC B");
    assert_canonical_roundtrip::<Vec<Instrument>>(raw);
}

#[test]
fn list_types_fixture_roundtrip() {
    let raw = list_types_fixture();
    let parsed: Vec<InstrumentType> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0].instrument_type, "ESH");
    assert_eq!(parsed[0].name, "Equity");
    assert_canonical_roundtrip::<Vec<InstrumentType>>(raw);
}

#[test]
fn get_type_fixture_roundtrip() {
    let raw = get_type_fixture();
    let parsed: Vec<InstrumentType> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].instrument_type, "ESH");
    assert_canonical_roundtrip::<Vec<InstrumentType>>(raw);
}

#[test]
fn list_underlyings_fixture_roundtrip() {
    let raw = list_underlyings_fixture();
    let parsed: Vec<Instrument> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].instrument_id, ERIC_B);
    assert_eq!(parsed[1].symbol, "VOLV B");
    assert_canonical_roundtrip::<Vec<Instrument>>(raw);
}

#[test]
fn get_instrument_suitability_fixture_roundtrip() {
    let raw = get_instrument_suitability_fixture();
    let parsed: Vec<InstrumentEligibility> = serde_json::from_str(raw).unwrap();
    assert_eq!(
        parsed,
        vec![
            InstrumentEligibility {
                eligible: true,
                instrument_id: ERIC_B,
            },
            InstrumentEligibility {
                eligible: false,
                instrument_id: InstrumentId(101),
            },
        ]
    );
    assert_canonical_roundtrip::<Vec<InstrumentEligibility>>(raw);
}

#[test]
fn get_instrument_fixture_roundtrip() {
    let raw = get_instrument_fixture();
    let parsed: Vec<Instrument> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 2);

    // First entry: fully populated.
    let full = &parsed[0];
    assert_eq!(full.instrument_id, ERIC_B);
    assert_eq!(full.symbol, "ERIC B");
    assert_eq!(full.currency, "SEK");
    assert_eq!(full.asset_class.as_deref(), Some("EQUITY"));
    assert_eq!(full.expiration_date, Some(date!(2099 - 12 - 31)));
    assert_eq!(full.market_view.as_deref(), Some("U"));
    assert_eq!(full.mifid2_category, Some(1));
    assert_eq!(full.sfdr_article, Some(8));
    assert_eq!(
        full.leverage_percentage,
        Some("1.234567".parse::<Decimal>().unwrap())
    );
    assert_eq!(
        full.margin_percentage,
        Some("25.5".parse::<Decimal>().unwrap())
    );
    assert_eq!(
        full.number_of_securities,
        Some("3267000000.0".parse::<Decimal>().unwrap())
    );
    assert_eq!(
        full.pawn_percentage,
        Some("80.0".parse::<Decimal>().unwrap())
    );
    assert_eq!(full.strike_price, Some("100.5".parse::<Decimal>().unwrap()));
    assert_eq!(full.total_fee, Some("0.0025".parse::<Decimal>().unwrap()));

    let kid = full.key_information_documents.as_ref().unwrap();
    assert_eq!(
        kid.url_for_short.as_deref(),
        Some("https://www.nordnet.se/kid/eric_b_short.pdf")
    );

    let tradables = full.tradables.as_ref().unwrap();
    assert_eq!(tradables.len(), 2);
    assert_eq!(tradables[0].market_id, MarketId(11));
    assert_eq!(tradables[0].mic, "XSTO");
    assert_eq!(tradables[0].tick_size_id, TickSizeId(1));
    assert_eq!(tradables[1].lot_size, "100.0".parse::<Decimal>().unwrap());

    let underlyings = full.underlyings.as_ref().unwrap();
    assert_eq!(underlyings.len(), 1);
    assert_eq!(underlyings[0].instrument_id, ERIC_B);
    assert_eq!(underlyings[0].instrumment_id, Some(ERIC_B));
    assert_eq!(underlyings[0].symbol, "ERIC B");

    // Second entry: minimal (only required fields).
    let minimal = &parsed[1];
    assert_eq!(minimal.instrument_id, InstrumentId(101));
    assert_eq!(minimal.asset_class, None);
    assert_eq!(minimal.tradables, None);
    assert_eq!(minimal.underlyings, None);
    assert_eq!(minimal.key_information_documents, None);
    assert_eq!(minimal.leverage_percentage, None);

    assert_canonical_roundtrip::<Vec<Instrument>>(raw);
}

#[test]
fn list_leverages_fixture_roundtrip() {
    let raw = list_leverages_fixture();
    let parsed: Vec<Instrument> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].symbol, "BULL ERIC X5 NORDNET");
    assert_eq!(parsed[0].market_view.as_deref(), Some("U"));
    assert_canonical_roundtrip::<Vec<Instrument>>(raw);
}

#[test]
fn get_leverage_filters_fixture_roundtrip() {
    let raw = get_leverage_filters_fixture();
    let parsed: LeverageFilter = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.no_of_instruments, 1234);
    assert_eq!(parsed.currencies, vec!["SEK", "EUR", "USD"]);
    assert_eq!(parsed.issuers.len(), 2);
    assert_eq!(parsed.issuers[0].issuer_id, IssuerId(1));
    assert_eq!(parsed.issuers[0].name, "Nordnet Markets");
    assert_eq!(parsed.market_view, vec!["U", "D"]);
    assert_canonical_roundtrip::<LeverageFilter>(raw);
}

#[test]
fn list_instrument_trades_fixture_roundtrip() {
    let raw = list_instrument_trades_fixture();
    let parsed: Vec<InstrumentPublicTrades> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].instrument_id, ERIC_B);
    assert_eq!(parsed[0].trades.len(), 2);

    let t0 = &parsed[0].trades[0];
    assert_eq!(t0.broker_buying.as_deref(), Some("AVA"));
    assert_eq!(t0.broker_selling.as_deref(), Some("NON"));
    assert_eq!(t0.trade_type.as_deref(), Some("REGULAR"));
    assert_eq!(t0.price, "65.4321".parse::<Decimal>().unwrap());
    assert_eq!(t0.volume, 200);

    let t1 = &parsed[0].trades[1];
    assert_eq!(t1.broker_buying, None);
    assert_eq!(t1.broker_selling, None);
    assert_eq!(t1.trade_type, None);

    assert_canonical_roundtrip::<Vec<InstrumentPublicTrades>>(raw);
}

#[test]
fn instrument_decimal_precision_survives_roundtrip() {
    // Verifies the `opt_arb_prec` adapter on `Instrument::leverage_percentage`
    // preserves multi-significant-digit precision through serde.
    let inst = Instrument {
        asset_class: None,
        brochure_url: None,
        currency: "SEK".to_owned(),
        dividend_policy: None,
        expiration_date: None,
        instrument_group_type: None,
        instrument_id: ERIC_B,
        instrument_type: "ESH".to_owned(),
        isin_code: None,
        key_information_documents: None,
        leverage_percentage: Some("0.123456789".parse::<Decimal>().unwrap()),
        margin_percentage: None,
        market_view: None,
        mifid2_category: None,
        multiplier: None,
        name: "Ericsson B".to_owned(),
        number_of_securities: None,
        pawn_percentage: None,
        price_type: None,
        prospectus_url: None,
        sector: None,
        sector_group: None,
        sfdr_article: None,
        strike_price: None,
        symbol: "ERIC B".to_owned(),
        total_fee: None,
        tradables: None,
        underlyings: None,
    };
    let serialized = serde_json::to_string(&inst).unwrap();
    let re_parsed: Instrument = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        re_parsed.leverage_percentage,
        Some("0.123456789".parse::<Decimal>().unwrap())
    );
    assert_eq!(inst, re_parsed);
}

#[test]
fn issuer_id_is_serde_transparent() {
    // Bare integer must parse as IssuerId.
    let parsed: IssuerId = serde_json::from_str("42").unwrap();
    assert_eq!(parsed, IssuerId(42));

    // And serialize back to the same bare integer.
    let serialized = serde_json::to_string(&parsed).unwrap();
    assert_eq!(serialized, "42");
}

#[test]
fn underlying_info_preserves_misspelled_instrumment_id_on_wire() {
    let u = UnderlyingInfo {
        instrument_id: ERIC_B,
        instrumment_id: Some(InstrumentId(999)),
        isin_code: "SE0000108656".to_owned(),
        symbol: "ERIC B".to_owned(),
    };
    let serialized = serde_json::to_string(&u).unwrap();
    // The misspelled wire key MUST be preserved verbatim.
    assert!(
        serialized.contains("\"instrumment_id\":999"),
        "expected misspelled key in serialized output, got: {}",
        serialized
    );
    let re_parsed: UnderlyingInfo = serde_json::from_str(&serialized).unwrap();
    assert_eq!(re_parsed, u);
}

// ---------------------------------------------------------------------------
// Layer 2 — deny_unknown_fields rejection
// ---------------------------------------------------------------------------

#[test]
fn instrument_rejects_unknown_fields() {
    let raw = r#"{
        "currency": "SEK",
        "instrument_id": 1,
        "instrument_type": "ESH",
        "name": "X",
        "symbol": "X",
        "extra": "nope"
    }"#;
    let r: Result<Instrument, _> = serde_json::from_str(raw);
    assert!(
        r.is_err(),
        "deny_unknown_fields must reject extra fields on Instrument"
    );
}

#[test]
fn instrument_type_rejects_unknown_fields() {
    let raw = r#"{
        "instrument_type": "ESH",
        "name": "Equity",
        "extra": "nope"
    }"#;
    let r: Result<InstrumentType, _> = serde_json::from_str(raw);
    assert!(
        r.is_err(),
        "deny_unknown_fields must reject extra fields on InstrumentType"
    );
}

#[test]
fn instrument_eligibility_rejects_unknown_fields() {
    let raw = r#"{
        "eligible": true,
        "instrument_id": 1,
        "extra": "nope"
    }"#;
    let r: Result<InstrumentEligibility, _> = serde_json::from_str(raw);
    assert!(
        r.is_err(),
        "deny_unknown_fields must reject extra fields on InstrumentEligibility"
    );
}

#[test]
fn leverage_filter_rejects_unknown_fields() {
    let raw = r#"{
        "currencies": [],
        "expiration_dates": [],
        "instrument_group_types": [],
        "instrument_types": [],
        "issuers": [],
        "market_view": [],
        "no_of_instruments": 0,
        "extra": "nope"
    }"#;
    let r: Result<LeverageFilter, _> = serde_json::from_str(raw);
    assert!(
        r.is_err(),
        "deny_unknown_fields must reject extra fields on LeverageFilter"
    );
}

// Sanity touch on the helper types, kept simple.
#[test]
fn key_information_documents_and_tradable_construct() {
    let _kid = KeyInformationDocuments {
        url_for_combined: None,
        url_for_long: None,
        url_for_short: None,
    };
    let _t = Tradable {
        display_order: 1,
        identifier: TradableId("101".to_owned()),
        lot_size: "1.0".parse::<Decimal>().unwrap(),
        market_id: MarketId(11),
        mic: "XSTO".to_owned(),
        price_unit: "SEK".to_owned(),
        tick_size_id: TickSizeId(1),
    };
}

// ---------------------------------------------------------------------------
// Layer 3 — Wiremock integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn lookup_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/lookup/market_id_identifier/11:101"))
        .respond_with(ResponseTemplate::new(200).set_body_string(lookup_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .lookup("market_id_identifier", "11:101")
        .await
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].symbol, "ERIC B");
}

#[tokio::test]
async fn lookup_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/lookup/market_id_identifier/bad"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .lookup("market_id_identifier", "bad")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn list_types_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/types"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_types_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let types = client.list_types().await.unwrap();
    assert_eq!(types.len(), 3);
}

#[tokio::test]
async fn list_types_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/types"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"code":"NEXT_INVALID_SESSION","message":"nope"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.list_types().await.unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}

#[tokio::test]
async fn get_type_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/types/ESH"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_type_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let types = client.get_type("ESH").await.unwrap();
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].instrument_type, "ESH");
}

#[tokio::test]
async fn get_type_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/types/ESH"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"code":"NEXT_INVALID_SESSION","message":"nope"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get_type("ESH").await.unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}

#[tokio::test]
async fn list_underlyings_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/underlyings/leverage/SEK"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_underlyings_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let entries = client.list_underlyings("leverage", "SEK").await.unwrap();
    assert_eq!(entries.len(), 2);
}

#[tokio::test]
async fn list_underlyings_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/underlyings/leverage/SEK"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .list_underlyings("leverage", "SEK")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn get_instrument_suitability_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/validation/suitability/16099583"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(get_instrument_suitability_fixture()),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let entries = client.get_instrument_suitability(ERIC_B).await.unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].eligible);
    assert!(!entries[1].eligible);
}

#[tokio::test]
async fn get_instrument_suitability_403_maps_to_forbidden() {
    let server = MockServer::start().await;
    // Doc says 403 carries No Content for anonymous sessions — empty body.
    Mock::given(method("GET"))
        .and(path("/instruments/validation/suitability/16099583"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get_instrument_suitability(ERIC_B).await.unwrap_err();
    match err {
        Error::Forbidden { body } => assert_eq!(body, ""),
        other => panic!("expected Forbidden, got {:?}", other),
    }
}

#[tokio::test]
async fn get_instrument_suitability_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/validation/suitability/16099583"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get_instrument_suitability(ERIC_B).await.unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn get_instrument_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/16099583"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_instrument_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let entries = client.get_instrument(ERIC_B).await.unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].instrument_id, ERIC_B);
}

#[tokio::test]
async fn get_instrument_204_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/16099583"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let entries = client.get_instrument(ERIC_B).await.unwrap();
    assert_eq!(
        entries.len(),
        0,
        "204 No Content should map to empty Vec<Instrument>"
    );
}

#[tokio::test]
async fn get_instrument_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/16099583"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get_instrument(ERIC_B).await.unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn list_leverages_without_filters_omits_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/16099583/leverages"))
        .and(move |req: &Request| req.url.query().is_none())
        .respond_with(ResponseTemplate::new(200).set_body_string(list_leverages_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let entries = client
        .list_leverages(ERIC_B, LeveragesQuery::default())
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
}

#[tokio::test]
async fn list_leverages_with_all_filters_forwards_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/16099583/leverages"))
        .and(query_param("currency", "SEK"))
        .and(query_param("expiration_date", "2025-12-19"))
        .and(query_param("instrument_group_type", "LEVERAGE"))
        .and(query_param("instrument_type", "WNT"))
        .and(query_param("issuer_id", "42"))
        .and(query_param("market_view", "U"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_leverages_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let q = LeveragesQuery {
        currency: Some("SEK"),
        expiration_date: Some("2025-12-19"),
        instrument_group_type: Some("LEVERAGE"),
        instrument_type: Some("WNT"),
        issuer_id: Some(IssuerId(42)),
        market_view: Some("U"),
    };
    let entries = client.list_leverages(ERIC_B, q).await.unwrap();
    assert_eq!(entries.len(), 1);
}

#[tokio::test]
async fn list_leverages_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/16099583/leverages"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .list_leverages(ERIC_B, LeveragesQuery::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn get_leverage_filters_returns_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/16099583/leverages/filters"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_leverage_filters_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let filter = client.get_leverage_filters(ERIC_B).await.unwrap();
    assert_eq!(filter.no_of_instruments, 1234);
    assert_eq!(filter.issuers.len(), 2);
}

#[tokio::test]
async fn get_leverage_filters_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/16099583/leverages/filters"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.get_leverage_filters(ERIC_B).await.unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn list_instrument_trades_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/16099583/trades"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_instrument_trades_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let entries = client.list_instrument_trades(ERIC_B).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].trades.len(), 2);
}

#[tokio::test]
async fn list_instrument_trades_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instruments/16099583/trades"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.list_instrument_trades(ERIC_B).await.unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}
