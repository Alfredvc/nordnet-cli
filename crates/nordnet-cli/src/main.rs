//! `nordnet` — agent-friendly command-line frontend for the Nordnet API.
//!
//! Phase 0 ships only the top-level scaffold. Phase 4 adds per-group
//! subcommands under `cmd/<group>.rs`. Every subcommand calls one library
//! method on `nordnet_api::Client` and emits the result via [`output::emit`].

use clap::{Parser, Subcommand};

mod cmd;
mod config;
mod output;
mod session;

use nordnet_api::auth::Session;
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

    /// Override the persisted session for one-off authenticated calls.
    /// Highest priority; falls back to the `NORDNET_SESSION_KEY` env var
    /// and finally to the session written by `nordnet auth login`.
    #[arg(long, global = true, env = "NORDNET_SESSION_KEY")]
    session_key: Option<String>,

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
    /// `nordnet auth <op>` — authentication: login persists a session
    /// to disk so subsequent commands run authenticated automatically.
    Auth {
        #[command(subcommand)]
        cmd: cmd::auth::Cmd,
    },
    /// `nordnet accounts <op>` — accounts, ledgers, positions, returns, trades.
    Accounts {
        #[command(subcommand)]
        cmd: cmd::accounts::Cmd,
    },
    /// `nordnet tradables <op>` — tradable info / trades / suitability.
    Tradables {
        #[command(subcommand)]
        cmd: cmd::tradables::Cmd,
    },
    /// `nordnet search <query>` — top-level instrument search.
    #[command(flatten)]
    Search(cmd::main_search::Cmd),
    /// `nordnet instruments <op>` — instrument lookups + leverage queries.
    Instruments {
        #[command(subcommand)]
        cmd: cmd::instruments::Cmd,
    },
    /// `nordnet instrument-search <op>` — attribute + entity-list searches.
    #[command(name = "instrument-search")]
    InstrumentSearch {
        #[command(subcommand)]
        cmd: cmd::instrument_search::Cmd,
    },
    /// `nordnet orders <op>` — list / place / modify / activate / cancel.
    #[cfg(feature = "orders-cli")]
    Orders {
        #[command(subcommand)]
        cmd: cmd::orders::OrdersCmd,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let fields = output::parse_fields(&cli.fields);
    let session_override = cli.session_key.clone();

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
            let client = build_client(session_override.as_deref())?;
            c.run(&client, &fields).await?;
        }
        Command::Countries { cmd } => {
            let client = build_client(session_override.as_deref())?;
            cmd.run(&client, &fields).await?;
        }
        Command::TickSizes { cmd } => {
            let client = build_client(session_override.as_deref())?;
            cmd.run(&client, &fields).await?;
        }
        Command::Markets { cmd } => {
            let client = build_client(session_override.as_deref())?;
            cmd.run(&client, &fields).await?;
        }
        Command::News { cmd } => {
            let client = build_client(session_override.as_deref())?;
            cmd.run(&client, &fields).await?;
        }
        Command::Auth { cmd } => {
            // `auth` sub-commands manage the session themselves; build
            // the unauthenticated client and let the sub-command attach
            // a session if the call requires one.
            let client = build_unauth_client()?;
            cmd.run(&client, &fields).await?;
        }
        Command::Accounts { cmd } => {
            let client = build_client(session_override.as_deref())?;
            cmd.run(&client, &fields).await?;
        }
        Command::Tradables { cmd } => {
            let client = build_client(session_override.as_deref())?;
            cmd.run(&client, &fields).await?;
        }
        Command::Search(c) => {
            let client = build_client(session_override.as_deref())?;
            c.run(&client, &fields).await?;
        }
        Command::Instruments { cmd } => {
            let client = build_client(session_override.as_deref())?;
            cmd.run(&client, &fields).await?;
        }
        Command::InstrumentSearch { cmd } => {
            let client = build_client(session_override.as_deref())?;
            cmd.run(&client, &fields).await?;
        }
        #[cfg(feature = "orders-cli")]
        Command::Orders { cmd } => {
            let client = build_client(session_override.as_deref())?;
            cmd.run(&client, &fields).await?;
        }
    }

    Ok(())
}

/// Build an unauthenticated `Client` targeting the configured base URL.
fn build_unauth_client() -> anyhow::Result<Client> {
    let cfg = config::Config::load()?;
    Client::new(cfg.base_url).map_err(Into::into)
}

/// Build a `Client` and attach a session resolved per the documented
/// override order (`--session-key` / `NORDNET_SESSION_KEY` env / disk
/// file written by `nordnet auth login`). Returns an unauthenticated
/// client when no session is available — the call may still succeed for
/// public-only endpoints.
fn build_client(session_override: Option<&str>) -> anyhow::Result<Client> {
    let client = build_unauth_client()?;
    if let Some(key) = session_override {
        return Ok(client.with_session(Session {
            session_key: key.to_owned(),
            // Override path doesn't carry expiry; the server is the
            // source of truth and will 401 if the key is stale.
            expires_in: 0,
        }));
    }
    if let Some(stored) = session::load()? {
        return Ok(client.with_session(stored.to_api_session()));
    }
    Ok(client)
}
