//! `nordnet news` — news article and source lookups.
//!
//! # Implemented ops
//!
//! - `sources` → `client.list_news_sources()`
//! - `get`     → `client.get_news_item(NewsId::from(id))`
//!
//! # Missing op
//!
//! The deprecated `GET /news` list op is intentionally not surfaced; only
//! `sources` and `get` are wired. There is no `list_news_items` on the
//! client.

use clap::{Args, Subcommand};
use nordnet_model::models::news::NewsId;

/// Subcommands for the `news` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List all news sources the authenticated user has access to.
    Sources,
    /// Get a news article by ID.
    Get(GetArgs),
}

/// Arguments for the `get` subcommand.
#[derive(Debug, Args)]
pub struct GetArgs {
    /// News article ID (integer).
    pub id: i64,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::Sources => {
                let r = client.list_news_sources().await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Get(a) => {
                let r = client.get_news_item(NewsId::from(a.id)).await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}
