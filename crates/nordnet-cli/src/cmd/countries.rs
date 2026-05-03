//! `nordnet countries` — country lookups.
//!
//! # Implemented ops
//!
//! - `list` → `client.list_countries()`
//! - `get`  → `client.get_country(code)` (single ISO code or comma-separated list)

use clap::{Args, Subcommand};
use indoc::indoc;

/// Subcommands for `nordnet countries`.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List all countries known to the Nordnet system.
    ///
    /// Static reference data. Returns ISO code, display name, and
    /// settlement defaults per country.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet countries list
            nordnet countries list --fields code,name
    "})]
    List,
    /// Look up one or more countries by ISO code (comma-separated, e.g. "SE,NO").
    ///
    /// Accepts a single ISO-3166 alpha-2 code or a comma-separated list.
    /// Returns the same shape as `list`, filtered to the requested codes.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet countries get SE
            nordnet countries get SE,NO,DK
    "})]
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
