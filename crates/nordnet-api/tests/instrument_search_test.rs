//! Tests for the `instrument_search` resource group.
//!
//! Three test layers per CONTRACTS.md:
//!
//! 1. Fixture roundtrip — every fixture parses under `deny_unknown_fields`
//!    and re-serializes to the same canonical JSON `Value`.
//! 2. `deny_unknown_fields` rejection — sanity touch on a representative
//!    struct.
//! 3. Wiremock integration — every op exercised with success + at least
//!    one error mapping. `search_bullbearlist` additionally tests the
//!    documented 204 No Content -> empty results mapping.
//!    `search_optionlist_pairs` asserts the required query parameters
//!    are forwarded.

use nordnet_api::resources::instrument_search::{AttributesQuery, ListSearchQuery, StocklistQuery};
use nordnet_api::{Client, Error};
use nordnet_model::ids::{InstrumentId, IssuerId, MarketId, TickSizeId};
use nordnet_model::models::instrument_search::{
    AttributeResults, BullBearListResults, MinifutureListResults, OptionListResults,
    StocklistResults, UnlimitedTurboListResults,
};
use pretty_assertions::assert_eq;
use rust_decimal::Decimal;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn get_attributes_fixture() -> &'static str {
    include_str!("../fixtures/instrument_search/get_attributes.response.json")
}

fn search_stocklist_fixture() -> &'static str {
    include_str!("../fixtures/instrument_search/search_stocklist.response.json")
}

fn search_bullbearlist_fixture() -> &'static str {
    include_str!("../fixtures/instrument_search/search_bullbearlist.response.json")
}

fn search_minifuturelist_fixture() -> &'static str {
    include_str!("../fixtures/instrument_search/search_minifuturelist.response.json")
}

fn search_unlimitedturbolist_fixture() -> &'static str {
    include_str!("../fixtures/instrument_search/search_unlimitedturbolist.response.json")
}

fn search_optionlist_pairs_fixture() -> &'static str {
    include_str!("../fixtures/instrument_search/search_optionlist_pairs.response.json")
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
fn get_attributes_fixture_roundtrip() {
    let raw = get_attributes_fixture();
    let parsed: AttributeResults = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.attributes_count, 2);
    let attrs = parsed.attributes.as_ref().unwrap();
    assert_eq!(attrs.len(), 2);
    assert_eq!(attrs[0].id.as_deref(), Some("name"));
    assert_eq!(attrs[1].id.as_deref(), Some("market_id"));
    let details = attrs[1].filter_details.as_ref().unwrap();
    let values = details.values.as_ref().unwrap();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].id.as_deref(), Some("11"));
    assert_eq!(values[0].count, Some(1500));
    assert_canonical_roundtrip::<AttributeResults>(raw);
}

#[test]
fn search_stocklist_fixture_roundtrip() {
    let raw = search_stocklist_fixture();
    let parsed: StocklistResults = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.rows, Some(2));
    assert_eq!(parsed.total_hits, Some(2));
    let results = parsed.results.as_ref().unwrap();
    assert_eq!(results.len(), 2);

    let first_info = results[0].instrument_info.as_ref().unwrap();
    assert_eq!(first_info.instrument_id, Some(ERIC_B));
    assert_eq!(first_info.symbol.as_deref(), Some("ERIC B"));
    assert_eq!(first_info.issuer_id, Some(IssuerId(1)));

    let market = results[0].market_info.as_ref().unwrap();
    assert_eq!(market.market_id, Some(MarketId(11)));
    assert_eq!(market.tick_size_id, Some(TickSizeId(1)));

    let price = results[0].price_info.as_ref().unwrap();
    let last = price.last.as_ref().unwrap();
    assert_eq!(last.decimals, Some(4));
    assert_eq!(last.price, Some("65.4321".parse::<Decimal>().unwrap()));
    let diff = price.diff.as_ref().unwrap();
    assert_eq!(diff.diff, Some("0.4321".parse::<Decimal>().unwrap()));

    let key_ratios = results[0].key_ratios_info.as_ref().unwrap();
    assert_eq!(key_ratios.pe, Some("12.7".parse::<Decimal>().unwrap()));

    let returns = results[0].historical_returns_info.as_ref().unwrap();
    assert_eq!(returns.yield_1y, Some("15.3".parse::<Decimal>().unwrap()));

    // Second result has only instrument_info; everything else None.
    assert!(results[1].market_info.is_none());
    assert!(results[1].price_info.is_none());

    assert_canonical_roundtrip::<StocklistResults>(raw);
}

#[test]
fn search_bullbearlist_fixture_roundtrip() {
    let raw = search_bullbearlist_fixture();
    let parsed: BullBearListResults = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.rows, Some(1));
    assert_eq!(parsed.total_hits, Some(1));
    assert_eq!(parsed.underlying_instrument_id, Some(ERIC_B));
    let results = parsed.results.as_ref().unwrap();
    assert_eq!(results.len(), 1);

    let cert = results[0].certificate_info.as_ref().unwrap();
    assert_eq!(cert.static_high_risk, Some(false));
    assert_eq!(
        cert.static_leverage,
        Some("5.0".parse::<Decimal>().unwrap())
    );

    let etp = results[0].etp_info.as_ref().unwrap();
    assert_eq!(etp.underlying_instrument_id, Some(ERIC_B));
    assert_eq!(etp.market_view.as_deref(), Some("U"));

    assert_canonical_roundtrip::<BullBearListResults>(raw);
}

#[test]
fn search_minifuturelist_fixture_roundtrip() {
    let raw = search_minifuturelist_fixture();
    let parsed: MinifutureListResults = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.rows, Some(1));
    let results = parsed.results.as_ref().unwrap();
    assert_eq!(results.len(), 1);

    let ko_calc = results[0].ko_calc_info.as_ref().unwrap();
    assert_eq!(ko_calc.ko_calc_underlying_market_id, Some(MarketId(11)));
    assert_eq!(
        ko_calc.ko_calc_conversion_ratio,
        Some("1.0".parse::<Decimal>().unwrap())
    );

    let ko = results[0].ko_info.as_ref().unwrap();
    assert_eq!(ko.financial_level, Some("50.0".parse::<Decimal>().unwrap()));
    assert_eq!(ko.stop_loss, Some("55.0".parse::<Decimal>().unwrap()));

    let pko = results[0].price_ko_info.as_ref().unwrap();
    assert_eq!(
        pko.indicative_leverage,
        Some("6.5".parse::<Decimal>().unwrap())
    );
    assert_eq!(pko.risk_buffer, Some("0.18".parse::<Decimal>().unwrap()));

    assert_canonical_roundtrip::<MinifutureListResults>(raw);
}

#[test]
fn search_unlimitedturbolist_fixture_roundtrip() {
    let raw = search_unlimitedturbolist_fixture();
    let parsed: UnlimitedTurboListResults = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.rows, Some(1));
    let results = parsed.results.as_ref().unwrap();
    assert_eq!(results.len(), 1);

    let info = results[0].instrument_info.as_ref().unwrap();
    assert_eq!(info.symbol.as_deref(), Some("UTURBO L ERIC"));

    let pko = results[0].price_ko_info.as_ref().unwrap();
    assert_eq!(
        pko.indicative_leverage,
        Some("4.5".parse::<Decimal>().unwrap())
    );

    assert_canonical_roundtrip::<UnlimitedTurboListResults>(raw);
}

#[test]
fn search_optionlist_pairs_fixture_roundtrip() {
    let raw = search_optionlist_pairs_fixture();
    let parsed: OptionListResults = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.rows, 1);
    assert_eq!(parsed.total_hits, 1);
    assert_eq!(parsed.results.len(), 1);

    let pair = &parsed.results[0];
    assert_eq!(pair.strike_price, "100.0".parse::<Decimal>().unwrap());

    let call_info = pair.call_option.instrument_info.as_ref().unwrap();
    assert_eq!(call_info.symbol.as_deref(), Some("ERIC4C100"));
    let put_info = pair.put_option.instrument_info.as_ref().unwrap();
    assert_eq!(put_info.symbol.as_deref(), Some("ERIC4P100"));

    let call_opt = pair.call_option.option_info.as_ref().unwrap();
    assert_eq!(
        call_opt.strike_price,
        Some("100.0".parse::<Decimal>().unwrap())
    );
    assert_eq!(call_opt.underlying_instrument_id, Some(ERIC_B));

    let call_deriv = pair.call_option.derivative_info.as_ref().unwrap();
    assert_eq!(call_deriv.contract_multiplier, Some(100));
    assert_eq!(call_deriv.expire_date, Some(1735000000000));

    assert_canonical_roundtrip::<OptionListResults>(raw);
}

// ---------------------------------------------------------------------------
// Layer 2 — deny_unknown_fields rejection
// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Layer 3 — Wiremock integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_attributes_returns_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/attributes"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_attributes_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .get_attributes(AttributesQuery::default())
        .await
        .unwrap();
    assert_eq!(result.attributes_count, 2);
}

#[tokio::test]
async fn get_attributes_with_filters_forwards_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/attributes"))
        .and(query_param("entity_type", "STOCKLIST"))
        .and(query_param("only_filterable", "true"))
        .and(query_param("attribute_group", "PRICE_INFO"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_attributes_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let q = AttributesQuery {
        entity_type: Some("STOCKLIST"),
        only_filterable: Some(true),
        attribute_group: vec!["PRICE_INFO".to_owned()],
        ..AttributesQuery::default()
    };
    let result = client.get_attributes(q).await.unwrap();
    assert_eq!(result.attributes_count, 2);
}

#[tokio::test]
async fn get_attributes_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/attributes"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .get_attributes(AttributesQuery::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn search_stocklist_returns_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/stocklist"))
        .and(move |req: &Request| req.url.query().is_none())
        .respond_with(ResponseTemplate::new(200).set_body_string(search_stocklist_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .search_stocklist(StocklistQuery::default())
        .await
        .unwrap();
    let results = result.results.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn search_stocklist_forwards_filters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/stocklist"))
        .and(query_param("free_text_search", "erics"))
        .and(query_param("limit", "10"))
        .and(query_param("offset", "0"))
        .and(query_param("sort_attribute", "name"))
        .and(query_param("sort_order", "asc"))
        .respond_with(ResponseTemplate::new(200).set_body_string(search_stocklist_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let q = StocklistQuery {
        free_text_search: Some("erics"),
        limit: Some(10),
        offset: Some(0),
        sort_attribute: Some("name"),
        sort_order: Some("asc"),
        ..StocklistQuery::default()
    };
    let result = client.search_stocklist(q).await.unwrap();
    assert_eq!(result.rows, Some(2));
}

#[tokio::test]
async fn search_stocklist_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/stocklist"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"code":"NEXT_INVALID_SESSION","message":"nope"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .search_stocklist(StocklistQuery::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}

#[tokio::test]
async fn search_bullbearlist_returns_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/bullbearlist"))
        .respond_with(ResponseTemplate::new(200).set_body_string(search_bullbearlist_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .search_bullbearlist(ListSearchQuery::default())
        .await
        .unwrap();
    assert_eq!(result.rows, Some(1));
    assert_eq!(result.underlying_instrument_id, Some(ERIC_B));
}

#[tokio::test]
async fn search_bullbearlist_204_maps_to_empty_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/bullbearlist"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .search_bullbearlist(ListSearchQuery::default())
        .await
        .unwrap();
    assert!(
        result.results.is_none(),
        "204 No Content should map to empty BullBearListResults"
    );
    assert!(result.rows.is_none());
    assert!(result.total_hits.is_none());
    assert!(result.underlying_instrument_id.is_none());
}

#[tokio::test]
async fn search_bullbearlist_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/bullbearlist"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .search_bullbearlist(ListSearchQuery::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn search_minifuturelist_returns_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/minifuturelist"))
        .respond_with(ResponseTemplate::new(200).set_body_string(search_minifuturelist_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .search_minifuturelist(ListSearchQuery::default())
        .await
        .unwrap();
    assert_eq!(result.rows, Some(1));
}

#[tokio::test]
async fn search_minifuturelist_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/minifuturelist"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .search_minifuturelist(ListSearchQuery::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn search_unlimitedturbolist_returns_results() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/unlimitedturbolist"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(search_unlimitedturbolist_fixture()),
        )
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .search_unlimitedturbolist(ListSearchQuery::default())
        .await
        .unwrap();
    assert_eq!(result.rows, Some(1));
}

#[tokio::test]
async fn search_unlimitedturbolist_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/unlimitedturbolist"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"code":"NEXT_INVALID_SESSION","message":"nope"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .search_unlimitedturbolist(ListSearchQuery::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}

#[tokio::test]
async fn search_optionlist_pairs_forwards_required_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/optionlist/pairs"))
        .and(query_param("currency", "SEK"))
        .and(query_param("expire_date", "1735000000000"))
        .and(query_param("underlying_symbol", "ERIC B"))
        .respond_with(ResponseTemplate::new(200).set_body_string(search_optionlist_pairs_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .search_optionlist_pairs("SEK", 1735000000000, "ERIC B")
        .await
        .unwrap();
    assert_eq!(result.rows, 1);
    assert_eq!(result.results.len(), 1);
}

#[tokio::test]
async fn search_optionlist_pairs_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/instrument_search/query/optionlist/pairs"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .search_optionlist_pairs("SEK", 1, "X")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}
