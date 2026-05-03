//! Models for the `news` resource group.
//!
//! Derived strictly from these schema files in `docs-extract/_definitions/`:
//!
//! - `NewsArticle.md`
//! - `NewsSource.md`
//!
//! ## Doc notes (for Phase 3X reconciliation)
//!
//! - `news_id` (`integer(int64)`) and `source_id` (`integer(int64)`) require
//!   newtypes per CONTRACTS.md. The foundation `crate::ids` module is locked
//!   and does not currently expose `NewsId` / `NewsSourceId`, so both are
//!   defined LOCALLY here. Phase 3X may promote them to `crate::ids` and
//!   migrate this module (and `models/main_search.rs`, which currently uses
//!   a plain `i64` for `external_news_id`).
//! - The `instruments` field in `NewsArticle` is documented as
//!   `< integer > array` (no `(int64)` qualifier) while `instrument_id`
//!   elsewhere in the docs is `integer(int64)`. We keep `Vec<InstrumentId>`
//!   here on the basis that these are the same identifier; the schema
//!   asymmetry is flagged for Phase 3X.
//! - `timestamp` (`integer(int64)`) is documented as "milliseconds since
//!   January 1st 1970 00:00:00 UTC". Following the precedent set in
//!   `models/main_search.rs` (no epoch-millis `Timestamp` newtype exists in
//!   `crate::models::shared`), we keep it as a plain `i64`.

use crate::ids::{InstrumentId, MarketId};
use serde::{Deserialize, Serialize};

/// External unique news article ID.
///
/// Defined here because `crate::ids` (foundation) is locked and does not
/// currently expose `NewsId`. Phase 3X may promote this to
/// `crate::ids::NewsId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NewsId(pub i64);

impl std::fmt::Display for NewsId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl From<i64> for NewsId {
    fn from(v: i64) -> Self {
        Self(v)
    }
}

impl From<NewsId> for i64 {
    fn from(v: NewsId) -> Self {
        v.0
    }
}

/// Nordnet unique news source ID.
///
/// Defined here because `crate::ids` (foundation) is locked and does not
/// currently expose `NewsSourceId`. Phase 3X may promote this to
/// `crate::ids::NewsSourceId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NewsSourceId(pub i64);

impl std::fmt::Display for NewsSourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl From<i64> for NewsSourceId {
    fn from(v: i64) -> Self {
        Self(v)
    }
}

impl From<NewsSourceId> for i64 {
    fn from(v: NewsSourceId) -> Self {
        v.0
    }
}

/// A news article as returned by `GET /news/{item_id}`.
///
/// Schema source: `docs-extract/_definitions/NewsArticle.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NewsArticle {
    /// Article body. Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Article author. Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byline: Option<String>,
    /// Article headline.
    pub headline: String,
    /// List of instrument IDs affected by article. Optional per schema.
    ///
    /// Doc note: the schema lists `< integer > array` here without the
    /// `(int64)` qualifier used elsewhere for `instrument_id`. We treat
    /// the elements as `InstrumentId` (the same identifier semantics).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruments: Option<Vec<InstrumentId>>,
    /// List of ISINs affected by the article. Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isin_codes: Option<Vec<String>>,
    /// News language.
    pub lang: String,
    /// Whether the article is in markdown format.
    pub markdown_format: bool,
    /// List of market IDs affected by the article. Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markets: Option<Vec<MarketId>>,
    /// External unique news ID.
    pub news_id: NewsId,
    /// News type. Valid values: `NEWS`, `ANALYSIS`, `PRESS_RELEASE`,
    /// `MARKET_COMMENTARY`, `PM`, `PMVECKAN`, `MARKET_NEWS`,
    /// `VOLATILITY_HALT`, `TRADING_HALT`, `TRADING_EVENT`, `TOP10`.
    pub news_type: String,
    /// List of sectors affected by the article. Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sectors: Option<Vec<String>>,
    /// Nordnet unique news source ID.
    pub source_id: NewsSourceId,
    /// Article summary. Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Publication date.
    ///
    /// Doc note: documented as milliseconds since 1970-01-01 00:00:00 UTC.
    /// Kept as `i64` because no epoch-millis `Timestamp` newtype exists
    /// under `crate::models::shared` (matches the precedent in
    /// `models/main_search.rs`).
    pub timestamp: i64,
    /// Exists for backwards compatibility. Always set to `NEWS`.
    ///
    /// Renamed to `r#type` because `type` is a Rust keyword; the raw
    /// identifier syntax keeps the JSON field name visible at the use site.
    #[serde(rename = "type")]
    pub r#type: String,
    /// Article version. Plain `i64` — not an identifier.
    pub version: i64,
}

/// A news source as returned by `GET /news_sources`.
///
/// Schema source: `docs-extract/_definitions/NewsSource.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NewsSource {
    /// List containing the country codes affected by the news source.
    /// Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub countries: Option<Vec<String>>,
    /// Access level. Valid values: `DELAYED` (15-minute delayed news),
    /// `REALTIME` (real-time news), `FLASH` (flash news; implies real-time
    /// access for ordinary news).
    pub level: String,
    /// News source name.
    pub name: String,
    /// Nordnet unique news source ID.
    pub source_id: NewsSourceId,
}
