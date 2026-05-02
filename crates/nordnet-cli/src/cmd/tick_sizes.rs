//! `nordnet tick-sizes` — tick-size table lookups.

use clap::{Args, Subcommand};
use nordnet_api::ids::TickSizeId;

/// Subcommands for the `tick-sizes` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List all tick-size tables.
    List,
    /// Get a tick-size table by ID.
    Get(GetArgs),
}

/// Arguments for the `get` subcommand.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// Tick-size table ID (integer).
    pub id: i64,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::List => {
                let r = client.list_tick_sizes().await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Get(a) => {
                let r = client.get_tick_size(TickSizeId::from(a.id)).await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}
