//! `nordnet info` — system status (the `root` API group).

use clap::Subcommand;
use indoc::indoc;

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Print Nordnet system status (GET /api/2).
    ///
    /// Public endpoint — no authentication required. Useful as a
    /// liveness probe before running authenticated workflows. Returns
    /// API version, valid system status, and supported feeds.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet info
            nordnet info --fields system_status,valid_version
    "})]
    Info,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::Info => {
                let status = client.get_system_status().await?;
                crate::output::emit(&status, fields)?;
            }
        }
        Ok(())
    }
}
