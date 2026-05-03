//! `nordnet tick-sizes` — tick-size table lookups.
//!
//! # Implemented ops
//!
//! - `list` → `client.list_tick_sizes()`
//! - `get`  → `client.get_tick_size(TickSizeId)`

use clap::{Args, Subcommand};
use indoc::indoc;
use nordnet_model::ids::TickSizeId;

/// Subcommands for the `tick-sizes` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List all tick-size tables.
    ///
    /// Static reference data. Each tradable references a tick-size
    /// table that defines the minimum price increment per price band.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet tick-sizes list
    "})]
    List,
    /// Get a tick-size table by ID.
    ///
    /// The ID is the `tick_size_id` referenced from a tradable's info.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet tick-sizes get 1
    "})]
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
