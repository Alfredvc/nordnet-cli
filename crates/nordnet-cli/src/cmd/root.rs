//! `nordnet info` — system status (the `root` API group).

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Print Nordnet system status (GET /api/2).
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
