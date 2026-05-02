//! Models for the `main_search` resource group.
//!
//! Derived strictly from these schema files in `docs-extract/_definitions/`:
//!
//! - `MainSearchResponse.md`
//! - `MainSearchResponseRow.md`
//! - `PriceWithDecimals.md`
//! - `EtpInfo.md`
//! - `KoInfo.md`
//! - `MarketInfo.md`
//! - `PriceKoInfo.md`
//! - `StatusInfo.md`
//!
//! Per the CONTRACTS.md "no subagent edits files outside its own group"
//! rule, every referenced type is defined locally here. Cross-group
//! reconciliation (deduplication of e.g. `PriceWithDecimals` if other
//! groups also use it) is deferred to Phase 3X.
//!
//! ## Doc notes
//!
//! - `external_news_id` (in `MainSearchResponseRow`) is documented as
//!   `integer(int64)`. There is no `NewsId` newtype under `crate::ids`,
//!   so the field is typed as plain `i64` here. (`models::news` defines
//!   a private `NewsId(i64)` newtype but it lives in the `news` group's
//!   own model file. Promotion deferred — single use site.)
//! - Several timestamp-shaped `integer(int64)` fields
//!   (`published_date_time`, `joined_at`, `tick_timestamp`,
//!   `first_trading_date`) follow Nordnet's UNIX-epoch-millis convention
//!   per the docs but are kept as plain `i64` (no `Timestamp` newtype
//!   exists for epoch-millis under `crate::models::shared`).
//! - `number(double)` fields are typed as [`rust_decimal::Decimal`]
//!   instead of `f64` per CONTRACTS.md "Never `f64`". The
//!   `Option<Decimal>` adapter was promoted to
//!   [`crate::models::shared::opt_arb_prec`] in Phase 3X (4-group dup).
//! - `EtpInfo`, `KoInfo`, `MarketInfo`, `PriceKoInfo`, `PriceWithDecimals`
//!   here are byte-equivalent to their counterparts in
//!   `models::instrument_search`. Per the Phase 3X rule, two-group dups
//!   without field-shape divergence are left in place — promoting them
//!   would churn `models::shared` without clear payoff.

use crate::ids::{InstrumentId, MarketId, TickSizeId};
use crate::models::shared::opt_arb_prec;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A price value paired with its number of decimals.
///
/// Schema: `_definitions/PriceWithDecimals.md`. Both fields are optional
/// per the doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PriceWithDecimals {
    /// Number of decimals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<i32>,
    /// Price amount. `Decimal` (never `f64`) per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub price: Option<Decimal>,
}

/// Exchange-Traded Product information.
///
/// Schema: `_definitions/EtpInfo.md`. All fields are optional per the doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EtpInfo {
    /// Certificate direction; localized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    /// First ETP trading date; Epoch time (UNIX millis per Nordnet
    /// convention, see module-level doc note).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_trading_date: Option<i64>,
    /// Leverage ETPs market view.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_view: Option<String>,
    /// Signals whether the instrument is part of the "Nordnet markets."
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nordnet_markets: Option<bool>,
    /// Underlying instrument ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlying_instrument_id: Option<InstrumentId>,
    /// Underlying instrument name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlying_name: Option<String>,
}

/// Knock-out instrument structural information.
///
/// Schema: `_definitions/KoInfo.md`. All fields are optional per the doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KoInfo {
    /// Financial level (strike price). `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub financial_level: Option<Decimal>,
    /// Stop-loss (barrier price). `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub stop_loss: Option<Decimal>,
}

/// Market information for a search-result row.
///
/// Schema: `_definitions/MarketInfo.md`. All fields are optional per the
/// doc. Note this is structurally distinct from `crate::models::markets::Market`
/// (which is what `GET /markets` returns); reconciliation belongs in Phase 3X.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MarketInfo {
    /// Market identifier (string form, e.g. `"XSTO"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    /// Market ID (numeric form).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_id: Option<MarketId>,
    /// Market sub-ID. Doc says `integer(int64)` but does not define a
    /// dedicated identifier type, so plain `i64` is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_sub_id: Option<i64>,
    /// Tick size table ID (when applicable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_size_id: Option<TickSizeId>,
}

/// Knock-out instrument price information.
///
/// Schema: `_definitions/PriceKoInfo.md`. All fields are optional per the
/// doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PriceKoInfo {
    /// High-risk (indicative).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indicative_high_risk: Option<bool>,
    /// Indicative leverage. `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub indicative_leverage: Option<Decimal>,
    /// Risk buffer. `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub risk_buffer: Option<Decimal>,
}

/// Current market trading status.
///
/// Schema: `_definitions/StatusInfo.md`. All fields are optional per the
/// doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StatusInfo {
    /// The last tick timestamp (UNIX millis per Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_timestamp: Option<i64>,
    /// The trading status (untranslated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trading_status: Option<String>,
    /// The translated trading status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translated_trading_status: Option<String>,
}

/// One row inside a [`MainSearchResponse`] — represents a single
/// instrument, news article, page, or Shareville profile match.
///
/// Schema: `_definitions/MainSearchResponseRow.md`. The doc table marks
/// every field except `display_name` as optional.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MainSearchResponseRow {
    /// News agency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agency: Option<String>,
    /// Localized news agency description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agency_description: Option<String>,
    /// Shareville avatar URI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_uri: Option<String>,
    /// Close price value for the previous trading day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_price: Option<PriceWithDecimals>,
    /// Shareville profile country.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Instrument currency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Yield for one day in percent (string per docs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_pct_one_day: Option<String>,
    /// Yield for one year in percent (string per docs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_pct_one_year: Option<String>,
    /// Display name (the only required field on this row).
    pub display_name: String,
    /// Display name with highlight tags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name_highlighted: Option<String>,
    /// Display symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_symbol: Option<String>,
    /// Indicator entity type. For example, `COMMODITY`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    /// ETP information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etp_info: Option<EtpInfo>,
    /// Exchange country code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_country: Option<String>,
    /// External unique news ID. See module doc note about the missing
    /// `NewsId` newtype — typed as plain `i64` for now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_news_id: Option<i64>,
    /// Indicator ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indicator_identifier: Option<String>,
    /// Indicator source ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indicator_source: Option<String>,
    /// Instrument group type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_group_type: Option<String>,
    /// Unique instrument ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_id: Option<InstrumentId>,
    /// Instrument type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrument_type: Option<String>,
    /// True if the page is a CMS page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_cms: Option<bool>,
    /// True if the page is an external page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_external: Option<bool>,
    /// Shareville user join date (UNIX millis per Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<i64>,
    /// Information related to knock-out instruments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ko_info: Option<KoInfo>,
    /// Language of the news article or page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Current last price value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_price: Option<PriceWithDecimals>,
    /// Last price title. For example, "Senaste NAV".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_price_title: Option<String>,
    /// Market data order book ID used in NNX.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_data_order_book_id: Option<String>,
    /// Market information. Specifies which market the price information
    /// is collected from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_info: Option<MarketInfo>,
    /// News ID as UUID used in NNX.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub news_id: Option<String>,
    /// News type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub news_type: Option<String>,
    /// Localized news type description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub news_type_description: Option<String>,
    /// Instrument ID used in NNX.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nnx_instrument_id: Option<String>,
    /// Knock-out instrument price information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_ko_info: Option<PriceKoInfo>,
    /// UUID for Shareville profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    /// Publication date according to the news source (UNIX millis per
    /// Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_date_time: Option<i64>,
    /// Shareville rating.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rating: Option<String>,
    /// Bid-ask spread.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread: Option<PriceWithDecimals>,
    /// Bid-ask spread in percent. `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub spread_pct: Option<Decimal>,
    /// Current market trading status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_info: Option<StatusInfo>,
    /// Price time stamp (UNIX millis per Nordnet convention).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_timestamp: Option<i64>,
    /// Trading order book ID used in NNX.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trading_order_book_id: Option<String>,
    /// Daily turnover. `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub turnover: Option<Decimal>,
    /// Turnover volume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turnover_volume: Option<i64>,
    /// Page URI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Shareville username.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// Number of times news article has been viewed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub views: Option<i32>,
    /// 1-day yield (string per docs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yield_1y: Option<String>,
}

/// One result group inside a search response.
///
/// Schema: `_definitions/MainSearchResponse.md`. `GET /main_search`
/// returns `Vec<MainSearchResponse>` — one entry per result group (e.g.
/// equities, news, pages).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MainSearchResponse {
    /// Result group data description.
    pub display_group_description: String,
    /// Result group data type.
    pub display_group_type: String,
    /// Limit for the search results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    /// Offset for the search results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,
    /// Result rows for this group.
    pub results: Vec<MainSearchResponseRow>,
    /// Total number of available rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<i32>,
}
