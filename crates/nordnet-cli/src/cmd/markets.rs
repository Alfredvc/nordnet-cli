//! `nordnet markets` — market lookups.
//!
//! # Implemented ops
//!
//! - `list` → `client.list_markets()`
//! - `get`  → `client.get_market(MarketId)`

use clap::{Args, Subcommand};
use indoc::indoc;
use nordnet_model::ids::MarketId;

/// Subcommands for the `markets` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List all tradable markets.
    ///
    /// Static reference data. Each entry pairs a numeric `market_id`
    /// (used everywhere else in the API) with its MIC and human name.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet markets list
            nordnet markets list --fields market_id,mic,name
    "})]
    List,
    /// Get a market by ID.
    ///
    /// `id` is the integer `market_id` from `nordnet markets list`
    /// (e.g. 11 = Stockholm/XSTO, 14 = Helsinki/XHEL).
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet markets get 11
    "})]
    Get(GetArgs),
}

/// Arguments for the `get` subcommand.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Market ID (integer).
    pub id: i64,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::List => {
                let r = client.list_markets().await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Get(a) => {
                let r = client.get_market(MarketId::from(a.id)).await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}
