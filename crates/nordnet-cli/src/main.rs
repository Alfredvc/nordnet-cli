//! `nordnet` — agent-friendly command-line frontend for the Nordnet API.
//!
//! Phase 0 ships only the top-level scaffold. Phase 4 adds per-group
//! subcommands under `cmd/<group>.rs`. Every subcommand calls one library
//! method on `nordnet_api::Client` and emits the result via [`output::emit`].

use clap::{Parser, Subcommand};

mod cmd;
mod config;
mod output;

use nordnet_api::Client;

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
    /// `nordnet info` — system status (root API group).
    #[command(flatten)]
    Root(cmd::root::Cmd),
    /// `nordnet countries <op>` — country lookups.
    Countries {
        #[command(subcommand)]
        cmd: cmd::countries::Cmd,
    },
    /// `nordnet tick-sizes <op>` — tick-size table lookups.
    #[command(name = "tick-sizes")]
    TickSizes {
        #[command(subcommand)]
        cmd: cmd::tick_sizes::Cmd,
    },
    /// `nordnet markets <op>` — market lookups.
    Markets {
        #[command(subcommand)]
        cmd: cmd::markets::Cmd,
    },
    /// `nordnet news <op>` — news source + article lookups.
    News {
        #[command(subcommand)]
        cmd: cmd::news::Cmd,
    },
    /// `nordnet login <op>` — authentication subcommands.
    Login {
        #[command(subcommand)]
        cmd: cmd::login::Cmd,
    },
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
        Command::Root(c) => {
            let client = build_client()?;
            c.run(&client, &fields).await?;
        }
        Command::Countries { cmd } => {
            let client = build_client()?;
            cmd.run(&client, &fields).await?;
        }
        Command::TickSizes { cmd } => {
            let client = build_client()?;
            cmd.run(&client, &fields).await?;
        }
        Command::Markets { cmd } => {
            let client = build_client()?;
            cmd.run(&client, &fields).await?;
        }
        Command::News { cmd } => {
            let client = build_client()?;
            cmd.run(&client, &fields).await?;
        }
        Command::Login { cmd } => {
            let client = build_client()?;
            cmd.run(&client, &fields).await?;
        }
    }

    Ok(())
}

/// Build a `Client` targeting the configured base URL. Authenticated
/// commands attach a session via `Client::with_session` after running
/// `nordnet login verify`.
fn build_client() -> anyhow::Result<Client> {
    let cfg = config::Config::load()?;
    Client::new(cfg.base_url).map_err(Into::into)
}
