//! `nordnet countries` — country lookups.

use clap::{Args, Subcommand};

/// Subcommands for `nordnet countries`.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List all countries known to the Nordnet system.
    List,
    /// Look up one or more countries by ISO code (comma-separated, e.g. "SE,NO").
    Get(GetArgs),
}

/// Arguments for `nordnet countries get`.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// ISO country code, or comma-separated list of codes (e.g. "SE" or "SE,NO").
    pub code: String,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::List => {
                let r = client.list_countries().await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Get(a) => {
                let r = client.get_country(&a.code).await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}
