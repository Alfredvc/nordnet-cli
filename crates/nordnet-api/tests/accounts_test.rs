//! Tests for the `accounts` resource group.
//!
//! Three test layers per CONTRACTS.md:
//!
//! 1. Fixture roundtrip — every fixture parses under `deny_unknown_fields`
//!    and re-serializes to the same canonical JSON `Value`. Plus a
//!    Decimal precision survival test on `Trade::volume`.
//! 2. `deny_unknown_fields` rejection — covers `Account`, `AccountInfo`,
//!    `Position`, and `Trade`.
//! 3. Wiremock integration — every op exercised with success + at least
//!    one error mapping. `list_accounts`, `list_positions`,
//!    `get_returns_today`, and `list_account_trades` additionally cover
//!    the 204 No Content -> empty Vec mapping. `list_account_trades`
//!    asserts the `days` query param is forwarded.

use nordnet_api::ids::{AccountId, MarketId, OrderId, TradableId};
use nordnet_api::models::accounts::{
    Account, AccountInfo, AccountTransactionsToday, Amount, Ledger, LedgerInformation, Position,
    PositionInstrument, Reserved, TradableRef, Trade,
};
use nordnet_api::models::shared::Currency;
use nordnet_api::resources::accounts::{
    AccountInfoQuery, ListAccountTradesQuery, ListAccountsQuery, ListPositionsQuery,
    ReturnsTodayQuery,
};
use nordnet_api::{Client, Error};
use pretty_assertions::assert_eq;
use rust_decimal::Decimal;
use time::macros::date;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn list_accounts_fixture() -> &'static str {
    include_str!("../fixtures/accounts/list_accounts.response.json")
}

fn get_account_info_fixture() -> &'static str {
    include_str!("../fixtures/accounts/get_account_info.response.json")
}

fn list_ledgers_fixture() -> &'static str {
    include_str!("../fixtures/accounts/list_ledgers.response.json")
}

fn list_positions_fixture() -> &'static str {
    include_str!("../fixtures/accounts/list_positions.response.json")
}

fn get_returns_today_fixture() -> &'static str {
    include_str!("../fixtures/accounts/get_returns_today.response.json")
}

fn list_account_trades_fixture() -> &'static str {
    include_str!("../fixtures/accounts/list_account_trades.response.json")
}

const ACC: AccountId = AccountId(1);

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
fn list_accounts_fixture_roundtrip() {
    let raw = list_accounts_fixture();
    let parsed: Vec<Account> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].accid, Some(AccountId(1)));
    assert_eq!(parsed[0].accno, 12345678);
    assert_eq!(parsed[0].alias, "Main");
    assert_eq!(parsed[0].r#type, "AF");
    assert!(parsed[0].default);
    assert_eq!(parsed[0].is_blocked, Some(false));
    assert_eq!(parsed[0].atyid, Some(3));

    assert_eq!(parsed[1].accid, None);
    assert_eq!(parsed[1].accno, 87654321);
    assert_eq!(parsed[1].r#type, "ISK");
    assert_eq!(parsed[1].blocked_reason.as_deref(), Some("Pending KYC"));
    assert_eq!(parsed[1].is_blocked, Some(true));

    assert_canonical_roundtrip::<Vec<Account>>(raw);
}

#[test]
fn get_account_info_fixture_roundtrip() {
    let raw = get_account_info_fixture();
    let parsed: Vec<AccountInfo> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 2);

    let full = &parsed[0];
    assert_eq!(full.accid, Some(AccountId(1)));
    assert_eq!(full.accno, 12345678);
    assert_eq!(full.account_currency, "SEK");
    assert_eq!(full.account_sum.currency, Currency::from("SEK"));
    assert_eq!(
        full.account_sum.value,
        "100000.50".parse::<Decimal>().unwrap()
    );
    assert_eq!(
        full.bonus_cash.as_ref().map(|a| a.value),
        Some("250.0".parse::<Decimal>().unwrap())
    );
    assert_eq!(full.registration_date, Some(date!(2018 - 04 - 15)));
    assert_eq!(full.reserved.total.value, "0.0".parse::<Decimal>().unwrap());

    let minimal = &parsed[1];
    assert_eq!(minimal.accid, None);
    assert_eq!(minimal.bonus_cash, None);
    assert_eq!(minimal.credit_account_sum, None);
    assert_eq!(minimal.intraday_credit, None);
    assert_eq!(minimal.short_positions_margin, None);
    assert_eq!(minimal.registration_date, None);

    assert_canonical_roundtrip::<Vec<AccountInfo>>(raw);
}

#[test]
fn list_ledgers_fixture_roundtrip() {
    let raw = list_ledgers_fixture();
    let parsed: LedgerInformation = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.ledgers.len(), 2);
    assert_eq!(parsed.ledgers[0].currency, "SEK");
    assert_eq!(parsed.ledgers[1].currency, "EUR");
    assert_eq!(
        parsed.ledgers[1].exchange_rate.value,
        "11.5".parse::<Decimal>().unwrap()
    );
    assert_eq!(parsed.total.value, "111500.0".parse::<Decimal>().unwrap());

    assert_canonical_roundtrip::<LedgerInformation>(raw);
}

#[test]
fn list_positions_fixture_roundtrip() {
    let raw = list_positions_fixture();
    let parsed: Vec<Position> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 2);

    assert_eq!(parsed[0].accno, 12345678);
    assert_eq!(parsed[0].instrument.symbol, "ERIC B");
    assert_eq!(parsed[0].instrument.instrument_id, 16099583);
    assert_eq!(parsed[0].qty, "100.0".parse::<Decimal>().unwrap());
    assert_eq!(parsed[0].margin_percent, 100);
    assert_eq!(parsed[0].pawn_percent, 80);

    // 0 is a valid sentinel value for non-tradable instruments per schema.
    assert_eq!(parsed[1].instrument.instrument_id, 0);
    assert_eq!(parsed[1].qty, "-10.5".parse::<Decimal>().unwrap());
    assert_eq!(parsed[1].instrument.isin_code, None);

    assert_canonical_roundtrip::<Vec<Position>>(raw);
}

#[test]
fn get_returns_today_fixture_roundtrip() {
    let raw = get_returns_today_fixture();
    let parsed: Vec<AccountTransactionsToday> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].transactions.currency, Currency::from("SEK"));
    assert_eq!(
        parsed[0].transactions.value,
        "2500.0".parse::<Decimal>().unwrap()
    );
    assert_eq!(parsed[1].transactions.currency, Currency::from("EUR"));
    assert_eq!(
        parsed[1].transactions.value,
        "-100.50".parse::<Decimal>().unwrap()
    );

    assert_canonical_roundtrip::<Vec<AccountTransactionsToday>>(raw);
}

#[test]
fn list_account_trades_fixture_roundtrip() {
    let raw = list_account_trades_fixture();
    let parsed: Vec<Trade> = serde_json::from_str(raw).unwrap();
    assert_eq!(parsed.len(), 2);

    let t0 = &parsed[0];
    assert_eq!(t0.accid, Some(AccountId(1)));
    assert_eq!(t0.accno, 12345678);
    assert_eq!(t0.counterparty.as_deref(), Some("AVA"));
    assert_eq!(t0.order_id, OrderId(555001));
    assert_eq!(t0.price.currency, Currency::from("SEK"));
    assert_eq!(t0.price.value, "100.50".parse::<Decimal>().unwrap());
    assert_eq!(t0.side, "BUY");
    assert_eq!(t0.tradable.identifier, TradableId("101".to_owned()));
    assert_eq!(t0.tradable.market_id, MarketId(11));
    assert_eq!(t0.trade_id.as_deref(), Some("T-2024-0001"));
    assert_eq!(t0.tradetime, 1714568400000);
    assert_eq!(t0.volume, "100.0".parse::<Decimal>().unwrap());

    let t1 = &parsed[1];
    assert_eq!(t1.accid, None);
    assert_eq!(t1.counterparty, None);
    assert_eq!(t1.trade_id, None);
    assert_eq!(t1.side, "SELL");
    assert_eq!(t1.volume, "5.5".parse::<Decimal>().unwrap());

    assert_canonical_roundtrip::<Vec<Trade>>(raw);
}

#[test]
fn trade_decimal_precision_survives_roundtrip() {
    // Verifies arbitrary_precision adapter on Trade::volume preserves
    // multi-significant-digit precision through serde.
    let trade = Trade {
        accid: None,
        accno: 1,
        counterparty: None,
        order_id: OrderId(2),
        price: Amount {
            currency: "SEK".into(),
            value: "100.123456789".parse::<Decimal>().unwrap(),
        },
        side: "BUY".to_owned(),
        tradable: TradableRef {
            identifier: TradableId("X".to_owned()),
            market_id: MarketId(11),
        },
        trade_id: None,
        tradetime: 0,
        volume: "0.123456789".parse::<Decimal>().unwrap(),
    };
    let serialized = serde_json::to_string(&trade).unwrap();
    let re_parsed: Trade = serde_json::from_str(&serialized).unwrap();
    assert_eq!(re_parsed.volume, "0.123456789".parse::<Decimal>().unwrap());
    assert_eq!(
        re_parsed.price.value,
        "100.123456789".parse::<Decimal>().unwrap()
    );
    assert_eq!(trade, re_parsed);
}

#[test]
fn tradable_ref_uses_object_shape_on_wire() {
    let r = TradableRef {
        identifier: TradableId("ABC".to_owned()),
        market_id: MarketId(11),
    };
    let s = serde_json::to_string(&r).unwrap();
    // Must serialize to the documented object form, not just the string.
    let v: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(v["identifier"], "ABC");
    assert_eq!(v["market_id"], 11);
    let back: TradableRef = serde_json::from_str(&s).unwrap();
    assert_eq!(back, r);
}

// ---------------------------------------------------------------------------
// Layer 2 — deny_unknown_fields rejection
// ---------------------------------------------------------------------------

#[test]
fn account_rejects_unknown_fields() {
    let raw = r#"{
        "accno": 1,
        "alias": "X",
        "default": true,
        "type": "AF",
        "extra": "nope"
    }"#;
    let r: Result<Account, _> = serde_json::from_str(raw);
    assert!(
        r.is_err(),
        "deny_unknown_fields must reject extra fields on Account"
    );
}

#[test]
fn account_info_rejects_unknown_fields() {
    let raw = format!(
        r#"{{
            "accno": 1,
            "account_credit": {{"currency":"SEK","value":0.0}},
            "account_currency": "SEK",
            "account_sum": {{"currency":"SEK","value":0.0}},
            "buy_orders_value": {{"currency":"SEK","value":0.0}},
            "collateral": {{"currency":"SEK","value":0.0}},
            "equity": {{"currency":"SEK","value":0.0}},
            "forward_sum": {{"currency":"SEK","value":0.0}},
            "full_marketvalue": {{"currency":"SEK","value":0.0}},
            "future_sum": {{"currency":"SEK","value":0.0}},
            "interest": {{"currency":"SEK","value":0.0}},
            "loan_limit": {{"currency":"SEK","value":0.0}},
            "own_capital": {{"currency":"SEK","value":0.0}},
            "own_capital_morning": {{"currency":"SEK","value":0.0}},
            "pawn_value": {{"currency":"SEK","value":0.0}},
            "reserved": {},
            "trading_power": {{"currency":"SEK","value":0.0}},
            "unrealized_future_profit_loss": {{"currency":"SEK","value":0.0}},
            "extra": "nope"
        }}"#,
        r#"{"corporate_actions":{"currency":"SEK","value":0.0},"fund_orders":{"currency":"SEK","value":0.0},"monthly_savings_exchange_traded":{"currency":"SEK","value":0.0},"other":{"currency":"SEK","value":0.0},"total":{"currency":"SEK","value":0.0}}"#
    );
    let r: Result<AccountInfo, _> = serde_json::from_str(&raw);
    assert!(
        r.is_err(),
        "deny_unknown_fields must reject extra fields on AccountInfo"
    );
}

#[test]
fn position_rejects_unknown_fields() {
    let raw = r#"{
        "accno": 1,
        "acq_price": {"currency":"SEK","value":1.0},
        "acq_price_acc": {"currency":"SEK","value":1.0},
        "instrument": {
            "currency": "SEK",
            "instrument_id": 1,
            "instrument_type": "ESH",
            "name": "X",
            "symbol": "X"
        },
        "margin_percent": 100,
        "market_value": {"currency":"SEK","value":1.0},
        "market_value_acc": {"currency":"SEK","value":1.0},
        "morning_price": {"currency":"SEK","value":1.0},
        "pawn_percent": 80,
        "qty": 1.0,
        "extra": "nope"
    }"#;
    let r: Result<Position, _> = serde_json::from_str(raw);
    assert!(
        r.is_err(),
        "deny_unknown_fields must reject extra fields on Position"
    );
}

#[test]
fn trade_rejects_unknown_fields() {
    let raw = r#"{
        "accno": 1,
        "order_id": 1,
        "price": {"currency":"SEK","value":1.0},
        "side": "BUY",
        "tradable": {"identifier":"X","market_id":11},
        "tradetime": 0,
        "volume": 1.0,
        "extra": "nope"
    }"#;
    let r: Result<Trade, _> = serde_json::from_str(raw);
    assert!(
        r.is_err(),
        "deny_unknown_fields must reject extra fields on Trade"
    );
}

// Sanity touch on small helper types.
#[test]
fn ledger_and_helpers_construct() {
    let _l = Ledger {
        acc_int_cred: Amount {
            currency: "SEK".into(),
            value: "0.0".parse().unwrap(),
        },
        acc_int_deb: Amount {
            currency: "SEK".into(),
            value: "0.0".parse().unwrap(),
        },
        account_sum: Amount {
            currency: "SEK".into(),
            value: "0.0".parse().unwrap(),
        },
        account_sum_acc: Amount {
            currency: "SEK".into(),
            value: "0.0".parse().unwrap(),
        },
        currency: "SEK".into(),
        exchange_rate: Amount {
            currency: "SEK".into(),
            value: "1.0".parse().unwrap(),
        },
    };
    let _r = Reserved {
        corporate_actions: Amount {
            currency: "SEK".into(),
            value: "0.0".parse().unwrap(),
        },
        fund_orders: Amount {
            currency: "SEK".into(),
            value: "0.0".parse().unwrap(),
        },
        monthly_savings_exchange_traded: Amount {
            currency: "SEK".into(),
            value: "0.0".parse().unwrap(),
        },
        other: Amount {
            currency: "SEK".into(),
            value: "0.0".parse().unwrap(),
        },
        total: Amount {
            currency: "SEK".into(),
            value: "0.0".parse().unwrap(),
        },
    };
    let _i = PositionInstrument {
        asset_class: None,
        brochure_url: None,
        currency: "SEK".into(),
        dividend_policy: None,
        expiration_date: None,
        instrument_group_type: None,
        instrument_id: 0,
        instrument_type: "ESH".into(),
        isin_code: None,
        market_view: None,
        mifid2_category: None,
        multiplier: None,
        name: "X".into(),
        number_of_securities: None,
        pawn_percentage: None,
        price_type: None,
        sector: None,
        sector_group: None,
        strike_price: None,
        symbol: "X".into(),
    };
}

// ---------------------------------------------------------------------------
// Layer 3 — Wiremock integration
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_accounts_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts"))
        .and(move |req: &Request| req.url.query().is_none())
        .respond_with(ResponseTemplate::new(200).set_body_string(list_accounts_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .list_accounts(ListAccountsQuery::default())
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].alias, "Main");
}

#[tokio::test]
async fn list_accounts_forwards_include_credit_accounts() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts"))
        .and(query_param("include_credit_accounts", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_accounts_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .list_accounts(ListAccountsQuery {
            include_credit_accounts: Some(true),
        })
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn list_accounts_204_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .list_accounts(ListAccountsQuery::default())
        .await
        .unwrap();
    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn list_accounts_401_maps_to_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"code":"NEXT_INVALID_SESSION","message":"nope"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .list_accounts(ListAccountsQuery::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Unauthorized { .. }));
}

#[tokio::test]
async fn get_account_info_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_account_info_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .get_account_info(ACC, AccountInfoQuery::default())
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].account_currency, "SEK");
}

#[tokio::test]
async fn get_account_info_forwards_query_flags() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/info"))
        .and(query_param("include_interest_rate", "false"))
        .and(query_param("include_short_pos_margin", "false"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_account_info_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .get_account_info(
            ACC,
            AccountInfoQuery {
                include_interest_rate: Some(false),
                include_short_pos_margin: Some(false),
            },
        )
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn get_account_info_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/info"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .get_account_info(ACC, AccountInfoQuery::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn list_ledgers_returns_entry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/ledgers"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_ledgers_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client.list_ledgers(ACC).await.unwrap();
    assert_eq!(result.ledgers.len(), 2);
    assert_eq!(result.total.currency, Currency::from("SEK"));
}

#[tokio::test]
async fn list_ledgers_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/ledgers"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client.list_ledgers(ACC).await.unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn list_positions_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/positions"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_positions_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .list_positions(ACC, ListPositionsQuery::default())
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].instrument.symbol, "ERIC B");
}

#[tokio::test]
async fn list_positions_forwards_query_flags() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/positions"))
        .and(query_param("include_instrument_loans", "true"))
        .and(query_param("include_intraday_limit", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_positions_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .list_positions(
            ACC,
            ListPositionsQuery {
                include_instrument_loans: Some(true),
                include_intraday_limit: Some(true),
            },
        )
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn list_positions_204_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/positions"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .list_positions(ACC, ListPositionsQuery::default())
        .await
        .unwrap();
    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn list_positions_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/positions"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .list_positions(ACC, ListPositionsQuery::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn get_returns_today_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/returns/transactions/today"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_returns_today_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .get_returns_today(ACC, ReturnsTodayQuery::default())
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].transactions.currency, Currency::from("SEK"));
}

#[tokio::test]
async fn get_returns_today_204_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/returns/transactions/today"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .get_returns_today(ACC, ReturnsTodayQuery::default())
        .await
        .unwrap();
    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn get_returns_today_forwards_include_credit_account() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/returns/transactions/today"))
        .and(query_param("include_credit_account", "false"))
        .respond_with(ResponseTemplate::new(200).set_body_string(get_returns_today_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .get_returns_today(
            ACC,
            ReturnsTodayQuery {
                include_credit_account: Some(false),
            },
        )
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn get_returns_today_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/returns/transactions/today"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .get_returns_today(ACC, ReturnsTodayQuery::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}

#[tokio::test]
async fn list_account_trades_returns_entries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/trades"))
        .and(move |req: &Request| req.url.query().is_none())
        .respond_with(ResponseTemplate::new(200).set_body_string(list_account_trades_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .list_account_trades(ACC, ListAccountTradesQuery::default())
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].side, "BUY");
}

#[tokio::test]
async fn list_account_trades_forwards_days() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/trades"))
        .and(query_param("days", "7"))
        .respond_with(ResponseTemplate::new(200).set_body_string(list_account_trades_fixture()))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .list_account_trades(ACC, ListAccountTradesQuery { days: Some(7) })
        .await
        .unwrap();
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn list_account_trades_204_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/trades"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let result = client
        .list_account_trades(ACC, ListAccountTradesQuery::default())
        .await
        .unwrap();
    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn list_account_trades_400_maps_to_bad_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/accounts/1/trades"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_string(r#"{"code":"INVALID_PARAMETER","message":"bad"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::new(server.uri()).unwrap();
    let err = client
        .list_account_trades(ACC, ListAccountTradesQuery::default())
        .await
        .unwrap_err();
    assert!(matches!(err, Error::BadRequest { .. }));
}
