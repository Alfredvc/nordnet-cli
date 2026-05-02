//! `nordnet` — agent-friendly command-line frontend for the Nordnet API.
//!
//! Phase 0 ships only the top-level scaffold. Phase 4 adds per-group
//! subcommands under `cmd/<group>.rs`. Every subcommand calls one library
//! method on `nordnet_api::Client` and emits the result via [`output::emit`].

use clap::{Parser, Subcommand};

mod cmd;
mod config;
mod output;

#[derive(Debug, Parser)]
#[command(
    name = "nordnet",
    version,
    about = "Agent-friendly CLI for the Nordnet External API v2."
)]
struct Cli {
    /// Comma-separated list of top-level fields to include in JSON
    /// output. Empty = full object. Applies to every subcommand that
    /// emits structured data.
    #[arg(long, global = true, default_value = "")]
    fields: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the loaded configuration (no secrets) and exit. Useful for
    /// agents to confirm their environment before running real ops.
    Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let fields = output::parse_fields(&cli.fields);

    match cli.command {
        Command::Config => {
            let cfg = config::Config::load()?;
            // Redact secrets — only structural fields surface.
            let view = serde_json::json!({
                "base_url": cfg.base_url,
                "service": cfg.service,
                "api_key_present": cfg.api_key.is_some(),
                "key_path": cfg.key_path,
                "default_account": cfg.default_account,
            });
            output::emit(&view, &fields)?;
        }
    }

    Ok(())
}
