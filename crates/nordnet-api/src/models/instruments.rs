//! Models for the `instruments` resource group.
//!
//! Derived strictly from these schema files in `docs-extract/_definitions/`:
//!
//! - `Instrument.md`
//! - `InstrumentType.md`
//! - `InstrumentEligibility.md`
//! - `InstrumentPublicTrades.md`
//! - `LeverageFilter.md`
//! - `Issuer.md`
//! - `Tradable.md`
//! - `UnderlyingInfo.md`
//! - `KeyInformationDocuments.md`
//! - `PublicTrade.md`
//!
//! Per CONTRACTS.md every referenced type is defined locally here. Cross-group
//! deduplication (e.g. with the structurally similar `tradables` group types,
//! which also defines `PublicTrade`) is deferred to Phase 3X.
//!
//! ## Doc notes
//!
//! - `instrument_id` in [`InstrumentEligibility`] and [`InstrumentPublicTrades`]
//!   is documented as `integer(int32)` whereas every other `instrument_id` in
//!   the API is `integer(int64)`. We keep the uniform [`InstrumentId`] newtype
//!   (which is `i64`) and flag the asymmetry here. Phase 3X may either widen
//!   the docs upstream or introduce a narrower newtype.
//! - `issuer_id` in [`Issuer`] is `integer(int64)`. Promoted in Phase 3X to
//!   [`crate::ids::IssuerId`]; previously a local newtype `IssuerId` lived
//!   here.
//! - `Instrument.expiration_date` is `string(date)` (i.e. `YYYY-MM-DD`).
//!   Phase 3X switched it from `Option<String>` to `Option<time::Date>` via
//!   [`crate::models::shared::date_iso8601::option`].
//! - `LeverageFilter.expiration_dates` is an array of `string(date)`. Phase
//!   3X switched it from `Vec<String>` to `Vec<time::Date>` via
//!   [`crate::models::shared::date_iso8601::vec`].
//! - `Tradable.identifier` is documented as a bare `string`. Phase 3X
//!   switched it from `String` to [`crate::ids::TradableId`] (which is a
//!   `serde(transparent)` newtype over `String`, wire-compatible).
//! - `Instrument.currency` is documented as a bare `string`. We deliberately
//!   do NOT use `crate::models::shared::Currency`: the Nordnet schema does
//!   not specify the typed shape, so harmonisation deferred.
//! - `number(double)` fields are typed as [`rust_decimal::Decimal`] (with the
//!   `arbitrary_precision` adapter) per CONTRACTS.md — never `f64`. The
//!   resulting types cannot derive [`Eq`]. The `Option<Decimal>` adapter
//!   was promoted to [`crate::models::shared::opt_arb_prec`] in Phase 3X.
//! - `UnderlyingInfo` exposes BOTH `instrument_id` (required) AND the legacy
//!   misspelled `instrumment_id` (optional). The Rust field name preserves
//!   the misspelling so the doc note is self-explanatory at the use site;
//!   the wire encoding is identical because the field name matches the
//!   schema verbatim.
//! - The resource ops `list_trades` (this group) and `get_suitability` (this
//!   group) are renamed in [`crate::Client`] to `list_instrument_trades` and
//!   `get_instrument_suitability` respectively, to avoid Rust-impl name
//!   collisions with the same-named ops in the `accounts` and `tradables`
//!   groups (all three groups install methods on the same `Client`). See
//!   `crate::resources::instruments` for the documented rationale.

use crate::ids::{InstrumentId, IssuerId, MarketId, TickSizeId, TradableId};
use crate::models::shared::{date_iso8601, opt_arb_prec};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// URLs to key information documents (KIDs).
///
/// Schema: `_definitions/KeyInformationDocuments.md`. All fields are
/// optional per the doc.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KeyInformationDocuments {
    /// URL to a Combined KID document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_for_combined: Option<String>,
    /// URL to a Long KID document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_for_long: Option<String>,
    /// URL to a Short KID document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_for_short: Option<String>,
}

/// One underlying instrument reference.
///
/// Schema: `_definitions/UnderlyingInfo.md`.
///
/// The schema lists BOTH `instrument_id` (required) and the misspelled
/// legacy `instrumment_id` (optional). The Rust field for the misspelled
/// variant preserves the typo to keep the legacy nature self-documenting
/// at the use site (the wire field name is identical).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UnderlyingInfo {
    /// Unique identifier of the underlying instrument (canonical field).
    pub instrument_id: InstrumentId,
    /// Legacy misspelled duplicate of [`UnderlyingInfo::instrument_id`].
    /// Optional per the schema. The Rust field name preserves the typo
    /// deliberately — see the type-level doc and the module doc note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instrumment_id: Option<InstrumentId>,
    /// The ISIN code of the underlying instrument.
    pub isin_code: String,
    /// The symbol of the underlying instrument.
    pub symbol: String,
}

/// One tradable variant of an instrument.
///
/// Schema: `_definitions/Tradable.md`. All fields are required.
///
/// `lot_size` is `number(double)` — typed as [`Decimal`] per CONTRACTS.md.
/// As a result this type cannot derive [`Eq`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Tradable {
    /// Determines the display order of the tradables for an instrument.
    pub display_order: i64,
    /// Nordnet tradable identifier. The combination of market ID and
    /// identifier is unique. Phase 3X switched the type from `String` to
    /// [`crate::ids::TradableId`] (serde-transparent newtype, wire form
    /// unchanged) for consistency with the rest of the API surface.
    pub identifier: TradableId,
    /// The lot size of the tradable. `Decimal` (never `f64`) per
    /// CONTRACTS.md.
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub lot_size: Decimal,
    /// Nordnet market identifier.
    pub market_id: MarketId,
    /// The market identifier code (MIC) of the tradable.
    pub mic: String,
    /// The unit that prices are sent in (e.g. `GBX`, `%`, currency code).
    pub price_unit: String,
    /// Tick size identifier.
    pub tick_size_id: TickSizeId,
}

/// An instrument as returned by `GET /instruments/...` responses.
///
/// Schema: `_definitions/Instrument.md` (28 fields). Several fields use
/// `number(double)` and are typed as [`Decimal`] — as a result this type
/// cannot derive [`Eq`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    /// Asset class key word.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_class: Option<String>,
    /// URL to brochure if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brochure_url: Option<String>,
    /// The currency of the instrument. Bare `string` per the schema —
    /// see module doc note re: `Currency` newtype.
    pub currency: String,
    /// The dividend policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dividend_policy: Option<String>,
    /// Expiration date if applicable. `YYYY-MM-DD` per the schema; typed
    /// as [`time::Date`] via the `date_iso8601::option` adapter (Phase 3X).
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
    /// is not tradable.
    pub instrument_id: InstrumentId,
    /// The instrument type.
    pub instrument_type: String,
    /// The instrument ISIN code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isin_code: Option<String>,
    /// URLs to key information documents (KIDs) if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_information_documents: Option<KeyInformationDocuments>,
    /// The leverage percentage if applicable. `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub leverage_percentage: Option<Decimal>,
    /// The margin percentage if applicable. `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub margin_percentage: Option<Decimal>,
    /// Marking market view for leverage instruments. `U` for up, `D` for
    /// down.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_view: Option<String>,
    /// The MiFID II category of the instrument. Used to determine if a
    /// user can trade the instrument.
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
    /// Number of securities, not available for all instruments.
    /// `Decimal` per CONTRACTS.md.
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
    /// URL to prospectus if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prospectus_url: Option<String>,
    /// The sector ID of the instrument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector: Option<String>,
    /// The sector group of the instrument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sector_group: Option<String>,
    /// The SFDR article of a fund. Can be 6, 8 or 9.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sfdr_article: Option<i32>,
    /// Strike price if applicable. `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub strike_price: Option<Decimal>,
    /// The instrument symbol, e.g. `ERIC B`.
    pub symbol: String,
    /// Total fee. `Decimal` per CONTRACTS.md.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "opt_arb_prec"
    )]
    pub total_fee: Option<Decimal>,
    /// The tradables that belong to the instrument. Omitted when the
    /// instrument is not tradable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tradables: Option<Vec<Tradable>>,
    /// A list of underlyings to the instrument.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underlyings: Option<Vec<UnderlyingInfo>>,
}

/// One Nordnet instrument type.
///
/// Schema: `_definitions/InstrumentType.md`. All fields are required.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InstrumentType {
    /// The instrument type code.
    pub instrument_type: String,
    /// The translated instrument type name.
    pub name: String,
}

/// Customer trading eligibility for a single instrument.
///
/// Schema: `_definitions/InstrumentEligibility.md`. All fields are required.
///
/// Note: `instrument_id` is documented as `integer(int32)` here while every
/// other `instrument_id` in the API is `integer(int64)`. We keep the uniform
/// [`InstrumentId`] (`i64`) newtype — see module doc note.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InstrumentEligibility {
    /// `true` if the customer is eligible to trade the instrument.
    pub eligible: bool,
    /// The instrument identifier.
    pub instrument_id: InstrumentId,
}

/// One public trade executed on the marketplace.
///
/// Schema: `_definitions/PublicTrade.md`.
///
/// Cannot derive [`Eq`] because `price` is a `Decimal`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PublicTrade {
    /// Buying participant. Optional per the schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broker_buying: Option<String>,
    /// Selling participant. Optional per the schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broker_selling: Option<String>,
    /// Market ID.
    pub market_id: MarketId,
    /// The price of the trade. `Decimal` (never `f64`) per CONTRACTS.md.
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub price: Decimal,
    /// Tick timestamp. UNIX time in milliseconds.
    pub tick_timestamp: i64,
    /// The trade ID on the exchange.
    pub trade_id: String,
    /// Trade timestamp. UNIX time in milliseconds.
    pub trade_timestamp: i64,
    /// The trade type defined by the exchange. Optional per the schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade_type: Option<String>,
    /// The volume of the trade.
    pub volume: i64,
}

/// Public trades for a single instrument.
///
/// Schema: `_definitions/InstrumentPublicTrades.md`.
///
/// Cannot derive [`Eq`] because the nested [`PublicTrade::price`] is a
/// `Decimal`. `instrument_id` is documented as `integer(int32)` — see
/// module doc note.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InstrumentPublicTrades {
    /// The unique instrument ID.
    pub instrument_id: InstrumentId,
    /// A list of the public trades.
    pub trades: Vec<PublicTrade>,
}

/// One issuer of a leverage instrument.
///
/// Schema: `_definitions/Issuer.md`. Both fields are required.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Issuer {
    /// Unique issuer ID.
    pub issuer_id: IssuerId,
    /// Issuer name.
    pub name: String,
}

/// Valid leverage instruments filter values for a given underlying.
///
/// Schema: `_definitions/LeverageFilter.md`. All fields are required.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LeverageFilter {
    /// List of valid currencies.
    pub currencies: Vec<String>,
    /// List of valid expiry dates (`YYYY-MM-DD` per Nordnet date
    /// convention); typed as [`time::Date`] via `date_iso8601::vec`
    /// (Phase 3X).
    #[serde(with = "date_iso8601::vec")]
    pub expiration_dates: Vec<time::Date>,
    /// List of valid instrument group types.
    pub instrument_group_types: Vec<String>,
    /// List of valid instrument types.
    pub instrument_types: Vec<String>,
    /// List of valid issuers.
    pub issuers: Vec<Issuer>,
    /// List of valid market views (e.g. `D`, `U`).
    pub market_view: Vec<String>,
    /// Number of derivative instruments matching this filter set.
    pub no_of_instruments: i64,
}
