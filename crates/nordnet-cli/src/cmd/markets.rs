//! `nordnet markets` — market lookups.

use clap::{Args, Subcommand};
use nordnet_api::ids::MarketId;

/// Subcommands for the `markets` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List all tradable markets.
    List,
    /// Get a market by ID.
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
