//! Models for the `accounts` resource group.
//!
//! Derived strictly from these schema files in `docs-extract/_definitions/`:
//!
//! - `Account.md`
//! - `AccountInfo.md`
//! - `AccountTransactionsToday.md`
//! - `Amount.md`
//! - `Ledger.md`
//! - `LedgerInformation.md`
//! - `Position.md`
//! - `Reserved.md`
//! - `Trade.md`
//! - `TradableId.md` (the schema object — distinct from
//!   [`crate::ids::TradableId`], which is the bare-string newtype)
//! - `Instrument.md` (referenced from `Position.instrument`)
//!
//! Per CONTRACTS.md, every referenced type is defined locally here. Cross-group
//! deduplication (e.g. with the structurally similar `instruments::Instrument`,
//! and the documented `Amount` schema vs `crate::models::shared::Money`) is
//! deferred to Phase 3X.
//!
//! ## Doc notes
//!
//! - The documented `Amount` schema is `{currency: string, value:
//!   number(double)}`. Phase 3X promoted this to
//!   [`crate::models::shared::AmountWithCurrency`]; the local `Amount`
//!   alias below is kept for source compatibility (typedef'd to the
//!   shared type) and uses [`crate::models::shared::Currency`] for the
//!   `currency` field. Wire format unchanged (Currency is
//!   `serde(transparent)` over `String`).
//! - [`PositionInstrument`] is a local definition for the `Instrument` type
//!   used by `Position.instrument`. The full `instruments::Instrument` lives
//!   in another group's models module; `crate::models::shared` cannot host
//!   it (locked after Phase 0). Only the fields we have schema evidence for
//!   are present here. Cross-group `Instrument` consolidation deferred —
//!   the two shapes have different field sets (the `Position.instrument`
//!   shape lacks `tradables`, `underlyings`, `key_information_documents`,
//!   `mifid2_category`, etc.).
//! - [`TradableRef`] is the documented `TradableId` *schema object*
//!   (`{identifier, market_id}`) used as the `tradable` field of
//!   [`Trade`]. The bare-string newtype `crate::ids::TradableId` is a
//!   different concept (a single `identifier` value); we keep both
//!   distinct here. (See also `orders::OrderTradable` for the same wire
//!   shape under a different name — kept duplicated as 2-group dup
//!   without field-shape divergence.)
//! - `Position.qty` and `Trade.volume` are `number(float)` /
//!   `number(double)` per the schema. They are typed as
//!   [`rust_decimal::Decimal`] (with the `arbitrary_precision` adapter)
//!   per CONTRACTS.md. Because of this `Position`, `Trade`,
//!   [`crate::models::shared::AmountWithCurrency`], `AccountInfo`,
//!   `Ledger`, `LedgerInformation`, `Reserved` and
//!   `AccountTransactionsToday` cannot derive [`Eq`].
//! - `Account.atyid` is documented as `integer(int32)`. Kept as `i32`.
//! - `Trade.tradetime` is `integer(int64)` UNIX milliseconds. Kept as
//!   plain `i64` (no `EpochMillis` newtype exists under
//!   `crate::models::shared`).
//! - `AccountInfo.registration_date` is `string(date)` (`YYYY-MM-DD`).
//!   Phase 3X switched it from `Option<String>` to `Option<time::Date>`
//!   via [`crate::models::shared::date_iso8601::option`].
//! - `PositionInstrument.expiration_date` (same `string(date)` shape) was
//!   likewise switched in Phase 3X.
//! - The `Account.type` and `AccountInfo.account_currency` fields are bare
//!   strings per the schema; we deliberately do NOT use
//!   [`crate::models::shared::Currency`] for `account_currency` because
//!   the schema documents it as a plain `string` (no separate `currency`
//!   wire shape), and the field is documented to mirror the bare ledger
//!   currency strings used elsewhere on `Ledger`.

use crate::ids::{AccountId, MarketId, OrderId, TradableId};
use crate::models::shared::{date_iso8601, opt_arb_prec};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Local source-compatibility alias for the documented `Amount` schema
/// (`{currency, value}`). Phase 3X promoted the type to
/// [`crate::models::shared::AmountWithCurrency`]; this `pub use` keeps
/// the in-group spelling (`Amount`) working at every reference site.
pub use crate::models::shared::AmountWithCurrency as Amount;

/// One Nordnet account the authenticated user has access to.
///
/// Schema: `_definitions/Account.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Account {
    /// The account identifier. Optional per schema (not applicable for
    /// partners).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accid: Option<AccountId>,
    /// The Nordnet account number. Always refers to a specific account.
    pub accno: i64,
    /// The account alias. Set by the customer.
    pub alias: String,
    /// The account type identifier. `integer(int32)` per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atyid: Option<i32>,
    /// The reason why the account is blocked. Translated to the language
    /// specified in the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
    /// `true` if this is the default account.
    pub default: bool,
    /// `true` if the account is blocked. No queries can be made against a
    /// blocked account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_blocked: Option<bool>,
    /// The account type. Translated.
    #[serde(rename = "type")]
    pub r#type: String,
}

/// Summary of trading-power reservations attached to an [`AccountInfo`].
///
/// Schema: `_definitions/Reserved.md`. All fields are required.
///
/// Cannot derive [`Eq`] because the nested [`Amount::value`] is a `Decimal`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Reserved {
    /// Reserved trading power for corporate actions.
    pub corporate_actions: Amount,
    /// Reserved trading power for mutual fund orders.
    pub fund_orders: Amount,
    /// Reserved trading power for exchange traded monthly savings.
    pub monthly_savings_exchange_traded: Amount,
    /// Sum of other trading power reservations.
    pub other: Amount,
    /// Total reserved trading power.
    pub total: Amount,
}

/// Account information details for a single account.
///
/// Schema: `_definitions/AccountInfo.md`.
///
/// Cannot derive [`Eq`] because nested [`Amount`] values use `Decimal`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AccountInfo {
    /// The account identifier. Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accid: Option<AccountId>,
    /// The Nordnet account number.
    pub accno: i64,
    /// The account credit.
    pub account_credit: Amount,
    /// The account currency. Bare `string` per schema (see module doc note).
    pub account_currency: String,
    /// The combined sum of all ledgers.
    pub account_sum: Amount,
    /// The bonus cash if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bonus_cash: Option<Amount>,
    /// The combined value of all pending buy orders.
    pub buy_orders_value: Amount,
    /// The collateral claim for options.
    pub collateral: Amount,
    /// The accrued interest for credit account if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit_account_interest: Option<Amount>,
    /// The sum for credit account if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit_account_sum: Option<Amount>,
    /// The sum of `own_capital` and `credit_account_sum`.
    pub equity: Amount,
    /// The locked amount for forwards.
    pub forward_sum: Amount,
    /// The total market value.
    pub full_marketvalue: Amount,
    /// The sum of intraday realized profits/losses for futures in account
    /// currency. Reset at night. Differs from
    /// `unrealized_future_profit_loss` which looks at existing positions.
    pub future_sum: Amount,
    /// The interest on the account.
    pub interest: Amount,
    /// The intraday credit if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intraday_credit: Option<Amount>,
    /// The maximum loan limit, regardless of pawn value.
    pub loan_limit: Amount,
    /// The sum of `account_sum`, `full_marketvalue`, `interest`,
    /// `forward_sum`, `future_sum` and `unrealized_future_profit_loss`.
    pub own_capital: Amount,
    /// Own capital calculated in the morning. Does not change during the
    /// day.
    pub own_capital_morning: Amount,
    /// The pawn value of all positions combined.
    pub pawn_value: Amount,
    /// The registration date of the account formatted as `YYYY-MM-DD`;
    /// typed as [`time::Date`] via the `date_iso8601::option` adapter
    /// (Phase 3X).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "date_iso8601::option"
    )]
    pub registration_date: Option<time::Date>,
    /// Summary of reserved trading power.
    pub reserved: Reserved,
    /// The short position margin if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_positions_margin: Option<Amount>,
    /// The amount available for trading.
    pub trading_power: Amount,
    /// The sum of profit and loss for all currently existing futures
    /// positions. Not the same as `future_sum`.
    pub unrealized_future_profit_loss: Amount,
}

/// One currency ledger of an account.
///
/// Schema: `_definitions/Ledger.md`. All fields are required.
///
/// Cannot derive [`Eq`] because nested [`Amount`] values use `Decimal`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Ledger {
    /// The interest credit in the ledger currency.
    pub acc_int_cred: Amount,
    /// The interest debit in the ledger currency.
    pub acc_int_deb: Amount,
    /// The sum in the ledger currency.
    pub account_sum: Amount,
    /// The sum in the account currency.
    pub account_sum_acc: Amount,
    /// The currency of the ledger. Bare `string` per schema.
    pub currency: String,
    /// The price to convert to base currency.
    pub exchange_rate: Amount,
}

/// All ledgers for an account, plus account-currency totals.
///
/// Schema: `_definitions/LedgerInformation.md`. All fields are required.
///
/// Cannot derive [`Eq`] because nested [`Amount`] values use `Decimal`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LedgerInformation {
    /// The list of all ledgers.
    pub ledgers: Vec<Ledger>,
    /// The total of all the ledgers in the account currency.
    pub total: Amount,
    /// The total interest credit in the account currency.
    pub total_acc_int_cred: Amount,
    /// The total interest debit in the account currency.
    pub total_acc_int_deb: Amount,
}

/// Today's withdrawal/deposit transaction amounts for an account.
///
/// Schema: `_definitions/AccountTransactionsToday.md`. All fields required.
///
/// Cannot derive [`Eq`] because the nested [`Amount::value`] is a `Decimal`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AccountTransactionsToday {
    /// Transaction amounts today.
    pub transactions: Amount,
}

/// Minimal `Instrument` shape used by [`Position::instrument`].
///
/// Schema: `_definitions/Instrument.md`. Only fields with schema evidence
/// (required + commonly-populated optional) are exposed here. The full
/// `Instrument` type lives in `models/instruments.rs` for that group's
/// own ops; we keep a local copy here per the module-ownership rule.
/// Phase 3X may consolidate.
///
/// Cannot derive [`Eq`] because several `number(double)` fields are
/// typed as `Decimal`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PositionInstrument {
    /// Asset class key word.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_class: Option<String>,
    /// URL to brochure if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brochure_url: Option<String>,
    /// The currency of the instrument. Bare `string` per schema.
    pub currency: String,
    /// The dividend policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dividend_policy: Option<String>,
    /// Expiration date if applicable. `YYYY-MM-DD` per schema; typed as
    /// [`time::Date`] via the `date_iso8601::option` adapter (Phase 3X).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "date_iso8601::option"
    )]
    pub expiration_date: Option<time::Date>,
    /// The instrument group (wider description than instrument type).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_group_type: Option<String>,
    /// Unique identifier of the instrument. May be 0 if the instrument
    /// is not tradable. Plain `i64` rather than `crate::ids::InstrumentId`
    /// because the documented `Position` schema does not specify the
    /// strong-typed shape here, and 0 is a sentinel value; cross-group
    /// reconciliation (with `instruments::Instrument`) belongs in Phase 3X.
    pub instrument_id: i64,
    /// The instrument type.
    pub instrument_type: String,
    /// The instrument ISIN code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isin_code: Option<String>,
    /// Marking market view for leverage instruments. `U` for up, `D` for
    /// down.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_view: Option<String>,
    /// The MiFID II category of the instrument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mifid2_category: Option<i32>,
    /// The instrument multiplier. `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub multiplier: Option<Decimal>,
    /// The instrument name.
    pub name: String,
    /// Number of securities, not available for all instruments. `Decimal`
    /// per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub number_of_securities: Option<Decimal>,
    /// The pawn percentage if applicable. `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub pawn_percentage: Option<Decimal>,
    /// Price type when trading. Examples: `monetary_amount`, `percentage`,
    /// `yield`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_type: Option<String>,
    /// The sector ID of the instrument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector: Option<String>,
    /// The sector group of the instrument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector_group: Option<String>,
    /// Strike price if applicable. `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub strike_price: Option<Decimal>,
    /// The instrument symbol, e.g. `ERIC B`.
    pub symbol: String,
}

/// One position in an account.
///
/// Schema: `_definitions/Position.md`.
///
/// Cannot derive [`Eq`] because `qty` is a `Decimal` and the nested
/// [`Amount`] values are `Decimal`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Position {
    /// The account identifier. Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accid: Option<AccountId>,
    /// The Nordnet account number.
    pub accno: i64,
    /// The acquisition price in the tradable currency.
    pub acq_price: Amount,
    /// The acquisition price in the account currency.
    pub acq_price_acc: Amount,
    /// The position instrument.
    pub instrument: PositionInstrument,
    /// The collateral percentage required to cover this position if short
    /// (`qty` < 0). `integer(int32)` per schema.
    pub margin_percent: i32,
    /// The market value in the tradable currency.
    pub market_value: Amount,
    /// The market value in the account currency.
    pub market_value_acc: Amount,
    /// The price of the position instrument in the morning.
    pub morning_price: Amount,
    /// The percentage the user is allowed loan on this position.
    /// `integer(int32)` per schema.
    pub pawn_percent: i32,
    /// The quantity of the position. `number(float)` per schema; typed
    /// as [`Decimal`] per CONTRACTS.md (never `f32`/`f64`).
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub qty: Decimal,
}

/// Tradable reference (`{identifier, market_id}`) per the documented
/// `TradableId` *schema object*.
///
/// Distinct from [`crate::ids::TradableId`], which is the bare-string
/// identifier newtype. We compose the bare-string newtype here so the
/// documented field shape round-trips correctly while retaining the
/// strongly-typed identifier.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct TradableRef {
    /// The Nordnet tradable identifier.
    pub identifier: TradableId,
    /// The Nordnet market identifier.
    pub market_id: MarketId,
}

/// One executed trade against an account.
///
/// Schema: `_definitions/Trade.md`.
///
/// Cannot derive [`Eq`] because `volume` is a `Decimal` and nested
/// [`Amount`] values are `Decimal`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Trade {
    /// The account identifier. Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accid: Option<AccountId>,
    /// The Nordnet account number.
    pub accno: i64,
    /// The counterparty if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counterparty: Option<String>,
    /// Nordnet order identifier.
    pub order_id: OrderId,
    /// The price of the trade.
    pub price: Amount,
    /// `BUY` or `SELL`. Bare `string` per schema (Nordnet does not enumerate
    /// the type in the docs as a typed enum); kept as `String` so unknown
    /// future variants do not break parsing.
    pub side: String,
    /// The tradable identifier (`{identifier, market_id}` per the schema).
    pub tradable: TradableRef,
    /// Trade identifier from the market if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_id: Option<String>,
    /// The time of the trade. UNIX timestamp in milliseconds (kept as
    /// plain `i64` — see module doc note).
    pub tradetime: i64,
    /// The volume of the trade. `number(double)` per schema; typed as
    /// [`Decimal`] per CONTRACTS.md (never `f64`).
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub volume: Decimal,
}

// Phase 3X: the `Option<Decimal>` arbitrary-precision adapter that used
// to live here was promoted to `crate::models::shared::opt_arb_prec` (it
// was duplicated in 4 group files — see PROCESS.md "Locked decisions"
// item 11).
