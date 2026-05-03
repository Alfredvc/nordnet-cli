//! `nordnet instruments` — instrument lookups + leverage queries.
//!
//! # Implemented ops
//!
//! - `lookup`            → `client.lookup(lookup_type, lookup)`
//! - `types`             → `client.list_types()`
//! - `get-type`          → `client.get_type(instrument_type)`
//! - `underlyings`       → `client.list_underlyings(derivative_type, currency)`
//! - `suitability`       → `client.get_instrument_suitability(InstrumentId)`
//! - `get`               → `client.get_instrument(InstrumentId)`
//! - `leverages`         → `client.list_leverages(InstrumentId, LeveragesQuery)`
//! - `leverage-filters`  → `client.get_leverage_filters(InstrumentId)`
//! - `trades`            → `client.list_instrument_trades(InstrumentId)`

use clap::{Args, Subcommand};
use indoc::indoc;
use nordnet_api::resources::instruments::LeveragesQuery;
use nordnet_model::ids::{InstrumentId, IssuerId};

/// Subcommands for the `instruments` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Look up instruments by predefined fields.
    ///
    /// Resolves a fixed set of identifier formats to instrument IDs.
    /// `lookup_type` selects the format (e.g. `market_id_identifier`,
    /// `isin_code_currency_market_id`); `lookup` is the formatted value
    /// that goes with it (e.g. `11:101`).
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instruments lookup market_id_identifier 11:101
            nordnet instruments lookup isin_code_currency_market_id SE0000108656:SEK:11
    "})]
    Lookup(LookupArgs),
    /// List all Nordnet instrument types.
    ///
    /// Static reference data — the canonical list of `instrument_type`
    /// codes used by `get-type`, `leverages --instrument-type`, and the
    /// instrument-search APIs.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instruments types
    "})]
    Types,
    /// Get one or more instrument types by code.
    ///
    /// Accepts a single code or a comma-separated list. Returns the
    /// description and metadata for each type.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instruments get-type ESH
            nordnet instruments get-type ESH,FUT
    "})]
    GetType(GetTypeArgs),
    /// List underlying instruments for a derivative type and currency.
    ///
    /// `derivative_type` is one of `leverage` or `option_pair`. Use the
    /// returned IDs as the underlying for `leverages` or
    /// `instrument-search option-list-pairs`.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instruments underlyings leverage SEK
            nordnet instruments underlyings option_pair SEK
    "})]
    Underlyings(UnderlyingsArgs),
    /// Get customer trading eligibility for an instrument.
    ///
    /// Authenticated. Returns whether the current customer can trade
    /// this instrument under their suitability profile.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instruments suitability 16099051
    "})]
    Suitability(InstrumentArg),
    /// Get instrument information by ID.
    ///
    /// Static + market metadata for a single instrument: name, type,
    /// ISIN, currency, leverage attributes (if applicable).
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instruments get 16099051
            nordnet instruments get 16099051 --fields instrument_id,name,currency
    "})]
    Get(InstrumentArg),
    /// List leverage instruments for an underlying with optional filters.
    ///
    /// All filter flags narrow the response server-side. Use
    /// `leverage-filters` first to discover which filter values are
    /// valid for a given underlying.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instruments leverages 16099051 --currency SEK
            nordnet instruments leverages 16099051 --currency SEK --market-view U
            nordnet instruments leverages 16099051 --issuer-id 14 --instrument-type MINI
    "})]
    Leverages(LeveragesArgs),
    /// Get valid leverage filter values for an underlying instrument.
    ///
    /// Returns the set of currencies, expiration dates, issuer IDs,
    /// instrument types, and market views accepted by `leverages` for
    /// this underlying.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instruments leverage-filters 16099051
    "})]
    LeverageFilters(InstrumentArg),
    /// List public trades for an instrument.
    ///
    /// Aggregated across all venues for the instrument. For per-venue
    /// trades, use `nordnet tradables trades <market_id>:<identifier>`.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instruments trades 16099051
    "})]
    Trades(InstrumentArg),
}

/// Arguments for the `lookup` subcommand.
#[derive(Debug, Args)]
pub struct LookupArgs {
    /// Lookup type (e.g. `market_id_identifier` or `isin_code_currency_market_id`).
    pub lookup_type: String,
    /// Lookup value formatted per the lookup type (e.g. `11:101`).
    pub lookup: String,
}

/// Arguments for the `get-type` subcommand.
#[derive(Debug, Args)]
pub struct GetTypeArgs {
    /// Instrument type code (or comma-separated list).
    pub instrument_type: String,
}

/// Arguments for the `underlyings` subcommand.
#[derive(Debug, Args)]
pub struct UnderlyingsArgs {
    /// Derivative type: `leverage` or `option_pair`.
    pub derivative_type: String,
    /// Derivative currency (e.g. `SEK`).
    pub currency: String,
}

/// Arguments carrying only an instrument ID.
#[derive(Debug, Args)]
pub struct InstrumentArg {
    /// Instrument ID (integer).
    pub instrument_id: i64,
}

/// Arguments for the `leverages` subcommand.
#[derive(Debug, Args)]
pub struct LeveragesArgs {
    /// Underlying instrument ID.
    pub instrument_id: i64,
    /// Filter: show only leverage instruments with this currency.
    #[arg(long)]
    pub currency: Option<String>,
    /// Filter: show only leverage instruments with this expiration date (YYYY-MM-DD).
    #[arg(long)]
    pub expiration_date: Option<String>,
    /// Filter: show only instruments with this instrument group type.
    #[arg(long)]
    pub instrument_group_type: Option<String>,
    /// Filter: show only instruments with this instrument type.
    #[arg(long)]
    pub instrument_type: Option<String>,
    /// Filter: show only leverage instruments from this issuer ID.
    #[arg(long)]
    pub issuer_id: Option<i64>,
    /// Filter: show only leverage instruments with this market view (`D` or `U`).
    #[arg(long)]
    pub market_view: Option<String>,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::Lookup(a) => {
                let r = client.lookup(&a.lookup_type, &a.lookup).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Types => {
                let r = client.list_types().await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::GetType(a) => {
                let r = client.get_type(&a.instrument_type).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Underlyings(a) => {
                let r = client
                    .list_underlyings(&a.derivative_type, &a.currency)
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Suitability(a) => {
                let r = client
                    .get_instrument_suitability(InstrumentId::from(a.instrument_id))
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Get(a) => {
                let r = client
                    .get_instrument(InstrumentId::from(a.instrument_id))
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Leverages(a) => {
                let currency = a.currency.as_deref();
                let expiration_date = a.expiration_date.as_deref();
                let instrument_group_type = a.instrument_group_type.as_deref();
                let instrument_type = a.instrument_type.as_deref();
                let market_view = a.market_view.as_deref();
                let q = LeveragesQuery {
                    currency,
                    expiration_date,
                    instrument_group_type,
                    instrument_type,
                    issuer_id: a.issuer_id.map(IssuerId::from),
                    market_view,
                };
                let r = client
                    .list_leverages(InstrumentId::from(a.instrument_id), q)
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::LeverageFilters(a) => {
                let r = client
                    .get_leverage_filters(InstrumentId::from(a.instrument_id))
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Trades(a) => {
                let r = client
                    .list_instrument_trades(InstrumentId::from(a.instrument_id))
                    .await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}
