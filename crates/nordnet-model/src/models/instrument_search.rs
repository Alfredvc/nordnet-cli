//! Models for the `instrument_search` resource group.
//!
//! Derived from the Nordnet attribute / list / pair / entity / info
//! schemas covering `AttributeResults`, `Stocklist`, `BullBearList`,
//! `MinifutureList`, `UnlimitedTurboList`, `OptionList`, plus the
//! `CertificateInfo`, `EtpInfo`, `ExchangeInfo`, `InstrumentInfo`,
//! `MarketInfo`, `PriceInfo`, `PriceWithDecimals`, `DiffWithDecimals`,
//! `CompanyInfo`, `HistoricalReturnsInfo`, `KeyRatiosInfo`,
//! `StatisticalInfo`, `KoCalcInfo`, `KoInfo`, `PriceKoInfo`,
//! `DerivativeInfo`, and `OptionInfo` definitions.
//!
//! Every referenced type is defined locally here. Some shapes (`EtpInfo`,
//! `KoInfo`, `MarketInfo`, `PriceKoInfo`, `PriceWithDecimals`) are
//! byte-equivalent to their counterparts in `models::main_search` and are
//! intentionally duplicated.
//!
//!
//! ## Doc notes
//!
//! - `number(double)` and bare `number` fields are typed as
//!   [`rust_decimal::Decimal`] (with the `arbitrary_precision` adapter) —
//!   never `f64`. As a result these types cannot derive [`Eq`].
//!   `Option<Decimal>` fields use
//!   [`crate::models::shared::opt_arb_prec`].
//! - Several timestamp-shaped `integer(int64)` fields
//!   (`first_trading_date`, `dividend_date`, `excluding_date`,
//!   `general_meeting_date`, `report_date`, `statistics_timestamp`,
//!   `tick_timestamp`, `expire_date`, `start_date`) follow Nordnet's
//!   UNIX-epoch-millis convention per the docs but are kept as plain
//!   `i64`.
//! - `InstrumentInfo.issuer_id` uses [`crate::ids::IssuerId`].
//! - `MarketInfo.market_sub_id` is `integer(int64)` with no dedicated
//!   newtype, so plain `i64` is used.
//! - `OptionInfo.risk_free_interest` and `OptionInfo.strike_price` are
//!   bare `number` (not `number(double)`) in the schema; we still use
//!   [`Decimal`].
//! - `OptionlistPair.strike_price` is bare `number` (required); typed as
//!   [`Decimal`].
//! - `attributes_count` (in `AttributeResults`) is the only required field
//!   on that struct.

use crate::ids::{InstrumentId, IssuerId, MarketId, TickSizeId};
use crate::models::shared::opt_arb_prec;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Attribute search (get_attributes)
// ---------------------------------------------------------------------------

/// One attribute filter value.
///
/// Schema: `_definitions/FilterVal.md`. All fields are optional per the doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FilterVal {
    /// The number of instruments or tradables which have this filter ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// Attribute filter ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Attribute filter display name. Can be localized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Additional details for attributes having `filterable=true`.
///
/// Schema: `_definitions/FilterDetails.md`. All fields are optional per
/// the doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FilterDetails {
    /// Attribute ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    /// List of attribute IDs which are logical parent filters for this
    /// attribute (e.g. `market_id` is a parent of `market_sub_id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_filters: Option<Vec<String>>,
    /// When `true`, the attribute ID must be provided to the attribute
    /// search APIs via `expand` if filter values should be returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_expand: Option<bool>,
    /// List of filter values for this attribute, if `expand` is specified.
    /// Supports only the `MULTISELECT` filter type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<FilterVal>>,
}

/// One attribute search result.
///
/// All fields are optional per the doc.
///
/// `min` and `max` are `number(double)` and typed as [`Decimal`]; this
/// prevents deriving [`Eq`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AttributeResult {
    /// Additional details for attributes having `filterable=true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_details: Option<FilterDetails>,
    /// Whether the attribute can be used as a filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filterable: Option<bool>,
    /// Attribute ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Maximum value. `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub max: Option<Decimal>,
    /// Minimum value. `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub min: Option<Decimal>,
    /// Attribute name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether the attribute can be returned by the instrument search APIs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returnable: Option<bool>,
    /// Whether the attribute can be used for sorting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sortable: Option<bool>,
}

/// Result wrapper for `GET /instrument_search/attributes`.
///
/// Schema: `_definitions/AttributeResults.md`. `attributes_count` is the
/// only required field.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AttributeResults {
    /// Attribute search results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<AttributeResult>>,
    /// Number of results returned.
    pub attributes_count: i64,
}

// ---------------------------------------------------------------------------
// Shared per-entity info blocks (instrument, exchange, market, price, etc.)
// ---------------------------------------------------------------------------

/// Instrument exchange information.
///
/// Schema: `_definitions/ExchangeInfo.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExchangeInfo {
    /// Instrument trading country.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_country: Option<String>,
}

/// Market info reported on an instrument-search result.
///
/// All fields optional per doc. Structurally distinct from
/// `crate::models::markets::Market`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct MarketInfo {
    /// Market identifier (string form, e.g. `"XSTO"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    /// Market ID (numeric form).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_id: Option<MarketId>,
    /// Market sub-ID (`integer(int64)`; plain `i64` — see module doc note).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_sub_id: Option<i64>,
    /// Tick size table ID (when applicable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_size_id: Option<TickSizeId>,
}

/// Instrument information block reported in an instrument-search row.
///
/// All fields optional per doc. `issuer_id` uses [`crate::ids::IssuerId`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct InstrumentInfo {
    /// Clearing place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearing_place: Option<String>,
    /// Currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Country-specific name. Populated only for indicators.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Instrument group type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_group_type: Option<String>,
    /// Unique instrument ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_id: Option<InstrumentId>,
    /// Maximum pawn percentage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_pawn_percentage: Option<i32>,
    /// Instrument type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_type: Option<String>,
    /// Instrument type hierarchy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_type_hierarchy: Option<String>,
    /// Set to `true` if the instrument can be used for monthly savings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_monthly_saveable: Option<bool>,
    /// Set to `true` if the instrument is shortable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_shortable: Option<bool>,
    /// Set to `true` if the instrument is tradable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_tradable: Option<bool>,
    /// International securities identification number (ISIN).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isin: Option<String>,
    /// Issuer ID — see [`crate::ids::IssuerId`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer_id: Option<IssuerId>,
    /// Issuer name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer_name: Option<String>,
    /// Localized long instrument name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_name: Option<String>,
    /// Short instrument name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Price unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_unit: Option<String>,
    /// Instrument symbol. Intended for presentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

/// A price value paired with its number of decimals.
///
/// Schema: `_definitions/PriceWithDecimals.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PriceWithDecimals {
    /// Number of decimals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<i32>,
    /// Price amount. `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub price: Option<Decimal>,
}

/// A diff value paired with its number of decimals.
///
/// Schema: `_definitions/DiffWithDecimals.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DiffWithDecimals {
    /// Number of decimals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<i32>,
    /// Difference amount. `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub diff: Option<Decimal>,
}

/// Top-of-book price information reported on a search-result row.
///
/// Schema: `_definitions/PriceInfo.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PriceInfo {
    /// Ask price, top of book.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask: Option<PriceWithDecimals>,
    /// Ask volume, top of book.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_volume: Option<i64>,
    /// Bid price, top of book.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid: Option<PriceWithDecimals>,
    /// Bid volume, top of book.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_volume: Option<i64>,
    /// Close price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close: Option<PriceWithDecimals>,
    /// Price difference since the last close price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<DiffWithDecimals>,
    /// Percent difference since the last close price.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub diff_pct: Option<Decimal>,
    /// Highest paid today.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high: Option<PriceWithDecimals>,
    /// Last price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<PriceWithDecimals>,
    /// Lowest paid today.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low: Option<PriceWithDecimals>,
    /// Open price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open: Option<PriceWithDecimals>,
    /// Set to `true` if the price information is based on a real-time
    /// snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realtime: Option<bool>,
    /// Bid-ask spread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread: Option<PriceWithDecimals>,
    /// Bid-ask spread percent.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub spread_pct: Option<Decimal>,
    /// Last tick time stamp (UNIX millis per Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_timestamp: Option<i64>,
    /// Daily turnover. `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub turnover: Option<Decimal>,
    /// Turnover volume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turnover_volume: Option<i64>,
}

// ---------------------------------------------------------------------------
// Stocklist (search_stocklist)
// ---------------------------------------------------------------------------

/// Company information for a stock-list result.
///
/// Schema: `_definitions/CompanyInfo.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CompanyInfo {
    /// Upcoming dividend amount. `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub dividend_amount: Option<Decimal>,
    /// Upcoming bonus dividend frequency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dividend_bonus_frequency: Option<i64>,
    /// Upcoming dividend currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dividend_currency: Option<String>,
    /// Upcoming dividend payout date (UNIX millis per Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dividend_date: Option<i64>,
    /// Upcoming dividend frequency (excl. bonus dividends).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dividend_frequency: Option<i64>,
    /// Upcoming dividend exclude date (UNIX millis per Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub excluding_date: Option<i64>,
    /// Upcoming annual general meeting date (UNIX millis per Nordnet
    /// convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub general_meeting_date: Option<i64>,
    /// Upcoming report date (UNIX millis per Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_date: Option<i64>,
    /// Upcoming report type translation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_description: Option<String>,
    /// Upcoming report type. For example, `ANNUAL_REPORT`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_type: Option<String>,
}

/// Historical returns information for a stock-list result.
///
/// Schema: `_definitions/HistoricalReturnsInfo.md`. All fields optional.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct HistoricalReturnsInfo {
    /// Set to `true` if the historical returns information is based on a
    /// real-time snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realtime: Option<bool>,
    /// Yield ten years.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub yield_10y: Option<Decimal>,
    /// Yield one month.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub yield_1m: Option<Decimal>,
    /// Yield one week.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub yield_1w: Option<Decimal>,
    /// Yield one year.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub yield_1y: Option<Decimal>,
    /// Yield three months.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub yield_3m: Option<Decimal>,
    /// Yield three years.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub yield_3y: Option<Decimal>,
    /// Yield five years.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub yield_5y: Option<Decimal>,
    /// Yield year to date.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub yield_ytd: Option<Decimal>,
}

/// Key ratios information for a stock-list result.
///
/// Schema: `_definitions/KeyRatiosInfo.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct KeyRatiosInfo {
    /// Dividend per share.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub dividend_per_share: Option<Decimal>,
    /// Dividend yield.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub dividend_yield: Option<Decimal>,
    /// Earnings per share (EPS).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub eps: Option<Decimal>,
    /// Price-to-book ratio (P/B).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub pb: Option<Decimal>,
    /// Price-to-earnings ratio (P/E).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub pe: Option<Decimal>,
    /// Price-to-sales ratio (P/S).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub ps: Option<Decimal>,
}

/// Statistical information for a stock-list result.
///
/// Schema: `_definitions/StatisticalInfo.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct StatisticalInfo {
    /// Number of Nordnet customers with positions in the instrument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number_of_owners: Option<i64>,
    /// Statistics time stamp (UNIX millis per Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistics_timestamp: Option<i64>,
}

/// One stock-list search result row.
///
/// Schema: `_definitions/Stocklist.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Stocklist {
    /// Company information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company_info: Option<CompanyInfo>,
    /// Instrument exchange information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_info: Option<ExchangeInfo>,
    /// Historical returns information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub historical_returns_info: Option<HistoricalReturnsInfo>,
    /// Instrument information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_info: Option<InstrumentInfo>,
    /// Key ratios information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_ratios_info: Option<KeyRatiosInfo>,
    /// Market information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_info: Option<MarketInfo>,
    /// Top-of-book price information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_info: Option<PriceInfo>,
    /// Statistical information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistical_info: Option<StatisticalInfo>,
}

/// Result wrapper for `GET /instrument_search/query/stocklist`.
///
/// Schema: `_definitions/StocklistResults.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct StocklistResults {
    /// Stock search results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<Stocklist>>,
    /// Number of results returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<i32>,
    /// Number of search hits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_hits: Option<i64>,
}

// ---------------------------------------------------------------------------
// Bull/Bear, Mini-future, Unlimited-turbo shared building blocks
// ---------------------------------------------------------------------------

/// Certificate information block.
///
/// Schema: `_definitions/CertificateInfo.md`. All fields optional per doc.
/// `static_leverage` is `number(double)` and typed as [`Decimal`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CertificateInfo {
    /// High-risk (static).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub static_high_risk: Option<bool>,
    /// Static leverage. `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub static_leverage: Option<Decimal>,
}

/// Exchange-Traded Product information.
///
/// Schema: `_definitions/EtpInfo.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct EtpInfo {
    /// Certificate direction; localized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    /// First ETP trading date (UNIX millis per Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_trading_date: Option<i64>,
    /// Leverage ETPs market view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_view: Option<String>,
    /// Signals whether the instrument is part of the "Nordnet markets".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nordnet_markets: Option<bool>,
    /// Underlying instrument ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlying_instrument_id: Option<InstrumentId>,
    /// Underlying instrument name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlying_name: Option<String>,
}

/// Knock-out instrument calculation information.
///
/// Schema: `_definitions/KoCalcInfo.md`. All fields optional per doc.
/// `ko_calc_conversion_ratio` is `number(double)` -> [`Decimal`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct KoCalcInfo {
    /// The conversion ratio. `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub ko_calc_conversion_ratio: Option<Decimal>,
    /// The underlying currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ko_calc_underlying_currency: Option<String>,
    /// The underlying identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ko_calc_underlying_identifier: Option<String>,
    /// The underlying market id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ko_calc_underlying_market_id: Option<MarketId>,
}

/// Knock-out instrument structural information.
///
/// Schema: `_definitions/KoInfo.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct KoInfo {
    /// Financial level (strike price). `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub financial_level: Option<Decimal>,
    /// Stop-loss (barrier price). `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub stop_loss: Option<Decimal>,
}

/// Knock-out instrument price information.
///
/// Schema: `_definitions/PriceKoInfo.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PriceKoInfo {
    /// High-risk (indicative).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indicative_high_risk: Option<bool>,
    /// Indicative leverage. `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub indicative_leverage: Option<Decimal>,
    /// Risk buffer. `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub risk_buffer: Option<Decimal>,
}

/// Bull & Bear search-result row.
///
/// Schema: `_definitions/BullBearEntity.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BullBearEntity {
    /// Certificate information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_info: Option<CertificateInfo>,
    /// ETP information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etp_info: Option<EtpInfo>,
    /// Instrument exchange information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_info: Option<ExchangeInfo>,
    /// Instrument information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_info: Option<InstrumentInfo>,
    /// Market information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_info: Option<MarketInfo>,
    /// Top-of-book price information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_info: Option<PriceInfo>,
}

/// Result wrapper for `GET /instrument_search/query/bullbearlist`.
///
/// Schema: `_definitions/BullBearListResults.md`. All fields optional per
/// doc. `underlying_instrument_id` is the only field at this level (it
/// uses `integer(int64)`, mapped to [`InstrumentId`]).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BullBearListResults {
    /// Bull & Bear search results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<BullBearEntity>>,
    /// Number of results returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<i32>,
    /// Number of search hits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_hits: Option<i64>,
    /// ID of the underlying instrument iff results share one underlying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlying_instrument_id: Option<InstrumentId>,
}

/// Mini-future search-result row.
///
/// Schema: `_definitions/MinifutureEntity.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MinifutureEntity {
    /// ETP information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etp_info: Option<EtpInfo>,
    /// Instrument exchange information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_info: Option<ExchangeInfo>,
    /// Instrument information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_info: Option<InstrumentInfo>,
    /// Knock-out instrument related information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ko_calc_info: Option<KoCalcInfo>,
    /// Information related to knock-out instruments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ko_info: Option<KoInfo>,
    /// Market information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_info: Option<MarketInfo>,
    /// Top-of-book price information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_info: Option<PriceInfo>,
    /// Knock-out instrument related price information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_ko_info: Option<PriceKoInfo>,
}

/// Result wrapper for `GET /instrument_search/query/minifuturelist`.
///
/// Schema: `_definitions/MinifutureListResults.md`. All fields optional.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct MinifutureListResults {
    /// Mini Future search results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<MinifutureEntity>>,
    /// Number of results returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<i32>,
    /// Number of search hits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_hits: Option<i64>,
    /// ID of the underlying instrument iff results share one underlying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlying_instrument_id: Option<InstrumentId>,
}

/// Unlimited-turbo search-result row.
///
/// Schema: `_definitions/UnlimitedTurboEntity.md`. All fields optional.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct UnlimitedTurboEntity {
    /// ETP information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etp_info: Option<EtpInfo>,
    /// Instrument exchange information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_info: Option<ExchangeInfo>,
    /// Instrument information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_info: Option<InstrumentInfo>,
    /// Knock-out instrument related information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ko_calc_info: Option<KoCalcInfo>,
    /// Information related to knock-out instruments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ko_info: Option<KoInfo>,
    /// Market information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_info: Option<MarketInfo>,
    /// Top-of-book price information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_info: Option<PriceInfo>,
    /// Knock-out instrument related price information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_ko_info: Option<PriceKoInfo>,
}

/// Result wrapper for `GET /instrument_search/query/unlimitedturbolist`.
///
/// Schema: `_definitions/UnlimitedTurboListResults.md`. All fields optional.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct UnlimitedTurboListResults {
    /// Unlimited Turbo search results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<UnlimitedTurboEntity>>,
    /// Number of results returned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rows: Option<i32>,
    /// Number of search hits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_hits: Option<i64>,
    /// ID of the underlying instrument iff results share one underlying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlying_instrument_id: Option<InstrumentId>,
}

// ---------------------------------------------------------------------------
// Optionlist pairs (search_optionlist_pairs)
// ---------------------------------------------------------------------------

/// Derivative information block.
///
/// Schema: `_definitions/DerivativeInfo.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DerivativeInfo {
    /// Derivative contract multiplier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_multiplier: Option<i32>,
    /// Derivative expiration date (UNIX millis per Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expire_date: Option<i64>,
    /// Derivative start date (UNIX millis per Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<i64>,
}

/// Option-specific information block.
///
/// Schema: `_definitions/OptionInfo.md`. All fields optional per doc.
/// `risk_free_interest` and `strike_price` are bare `number` -> [`Decimal`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct OptionInfo {
    /// Option exercise type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exercise_type: Option<String>,
    /// Risk-free interest based on the option expiration date. `Decimal`
    /// (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub risk_free_interest: Option<Decimal>,
    /// Option strike price. `Decimal` (never `f64`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub strike_price: Option<Decimal>,
    /// Underlying instrument ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlying_instrument_id: Option<InstrumentId>,
    /// Underlying instrument name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlying_name: Option<String>,
}

/// One option entity (call or put leg).
///
/// Schema: `_definitions/OptionlistEntity.md`. All fields optional per doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct OptionlistEntity {
    /// Derivative information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derivative_info: Option<DerivativeInfo>,
    /// Instrument exchange information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_info: Option<ExchangeInfo>,
    /// Instrument information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_info: Option<InstrumentInfo>,
    /// Market information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_info: Option<MarketInfo>,
    /// Option information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub option_info: Option<OptionInfo>,
    /// Top-of-book price information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_info: Option<PriceInfo>,
}

/// One Put-Call option pair sharing a strike price.
///
/// Schema: `_definitions/OptionlistPair.md`. All fields are required.
/// `strike_price` is bare `number` -> [`Decimal`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct OptionlistPair {
    /// Call option leg.
    pub call_option: OptionlistEntity,
    /// Put option leg.
    pub put_option: OptionlistEntity,
    /// Common strike price for the Put and Call options. `Decimal` (never
    /// `f64`).
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub strike_price: Decimal,
}

/// Result wrapper for `GET /instrument_search/query/optionlist/pairs`.
///
/// Schema: `_definitions/OptionListResults.md`. All fields are required
/// per the doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct OptionListResults {
    /// Option Pair search results.
    pub results: Vec<OptionlistPair>,
    /// Number of results returned.
    pub rows: i32,
    /// Number of search hits.
    pub total_hits: i64,
}
