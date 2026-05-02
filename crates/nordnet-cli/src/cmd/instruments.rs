//! `nordnet instruments` — instrument lookups + leverage queries.

use clap::{Args, Subcommand};
use nordnet_api::ids::{InstrumentId, IssuerId};
use nordnet_api::resources::instruments::LeveragesQuery;

/// Subcommands for the `instruments` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Look up instruments by predefined fields.
    Lookup(LookupArgs),
    /// List all Nordnet instrument types.
    Types,
    /// Get one or more instrument types by code.
    GetType(GetTypeArgs),
    /// List underlying instruments for a derivative type and currency.
    Underlyings(UnderlyingsArgs),
    /// Get customer trading eligibility for an instrument.
    Suitability(InstrumentArg),
    /// Get instrument information by ID.
    Get(InstrumentArg),
    /// List leverage instruments for an underlying with optional filters.
    Leverages(LeveragesArgs),
    /// Get valid leverage filter values for an underlying instrument.
    LeverageFilters(InstrumentArg),
    /// List public trades for an instrument.
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
