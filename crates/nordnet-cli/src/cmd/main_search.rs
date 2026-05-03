//! `nordnet main_search` — top-level instrument search.
//!
//! Wired into main.rs as `nordnet search <query> [options]`.

use clap::{ArgAction, Args, Subcommand};
use indoc::indoc;

/// Subcommands for the `main_search` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Run a main search. The query string is required; all other
    /// parameters are optional with API-side defaults.
    ///
    /// Cross-corpus search across instruments, news, CMS pages, and
    /// blog posts. Use `--search-space` to narrow. The result shape
    /// varies per search-space hit; use `--fields` carefully.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet search ericsson
            nordnet search 'apple inc' --limit 25
            nordnet search ericsson --search-space INSTRUMENTS
            nordnet search ericsson --instrument-group EQUITY --instrument-group ETF
    "})]
    Search(SearchArgs),
}

/// Arguments for the `search` subcommand.
#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search string.
    pub query: String,
    /// Restrict to instrument group(s). Pass multiple times for multiple groups.
    /// Example: `--instrument-group EQUITY --instrument-group ETF`.
    #[arg(long, action = ArgAction::Append)]
    pub instrument_group: Option<Vec<String>>,
    /// Maximum number of results (server default 5).
    #[arg(long)]
    pub limit: Option<i32>,
    /// Skip the first N results (server default 0).
    #[arg(long)]
    pub offset: Option<i32>,
    /// Search space: ALL (default), INSTRUMENTS, NEWS, CMS, BLOG,
    /// INSTRUMENTS_NEWS, INSTRUMENTS_CMS, NEWS_CMS, NEWS_BLOG,
    /// NEWS_BLOG_CMS — see Nordnet docs.
    #[arg(long)]
    pub search_space: Option<String>,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::Search(a) => {
                let groups_borrowed: Option<Vec<&str>> = a
                    .instrument_group
                    .as_ref()
                    .map(|v| v.iter().map(String::as_str).collect());
                let groups = groups_borrowed.as_deref();
                let r = client
                    .search(
                        &a.query,
                        groups,
                        a.limit,
                        a.offset,
                        a.search_space.as_deref(),
                    )
                    .await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}
