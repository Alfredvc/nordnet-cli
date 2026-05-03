//! `nordnet` — agent-friendly command-line frontend for the Nordnet API.
//!
//! Every subcommand calls one method on [`nordnet_api::Client`] and emits a
//! single pretty-printed JSON document on stdout via [`output::emit`].
//! Errors print a structured JSON document to stderr and the binary exits
//! non-zero.
//!
//! See the crate README for the user-facing manual: install, configuration
//! resolution order, output contract, shell completions, and the
//! agent/scripting integration guide.

use clap::{Parser, Subcommand};
use indoc::indoc;

mod cmd;
mod config;
mod output;
mod session;

use nordnet_api::Client;
use nordnet_model::auth::Session;

#[derive(Debug, Parser)]
#[command(
    name = "nordnet",
    version,
    about = "Agent-friendly CLI for the Nordnet External API v2.",
    long_about = indoc! {"
        Agent-friendly CLI for the Nordnet External API v2.

        Every subcommand emits a single pretty-printed JSON document on
        stdout. Errors print a structured JSON document to stderr and the
        binary exits non-zero. Output schemas are append-only within a
        0.x minor version — scripts may rely on top-level field names not
        being renamed or removed without a minor bump.

        Configuration resolves in this order, highest priority first:
        CLI flags → environment variables → credentials.toml.
    "},
    after_help = indoc! {"
        EXAMPLES:
            nordnet info                              # public health check, no auth
            nordnet auth login                        # SSH-key login, persists session
            nordnet accounts list                     # authenticated request
            nordnet accounts positions 12345 --fields id,instrument,qty
            nordnet completions zsh > _nordnet        # shell completion script

        See also:
            nordnet <subcommand> --help               # per-subcommand reference
            https://github.com/Alfredvc/nordnet-cli   # README + agent guide
    "}
)]
pub(crate) struct Cli {
    /// Comma-separated list of top-level fields to include in JSON
    /// output. Empty = full object. Applies to every subcommand that
    /// emits structured data.
    ///
    /// Field order in the output follows the order given on the command
    /// line. Scalars and primitive arrays cannot be filtered — the
    /// command will exit non-zero if `--fields` is set against a non-object
    /// payload.
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
    /// Print the loaded configuration (no secrets) and exit.
    ///
    /// Resolves the active configuration in the documented override
    /// order (CLI flags → env vars → credentials.toml) and prints it as
    /// JSON. Useful before scripted runs to confirm the binary is
    /// reading the keys you expect. Secrets are redacted: only
    /// `api_key_present: bool` is surfaced.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet config
            nordnet config --fields base_url,service,api_key_present
    "})]
    Config,
    /// Print a shell completion script to stdout.
    ///
    /// Runtime generation pattern that survives `cargo install`. Pipe
    /// the output into your shell's completion directory; see
    /// `nordnet completions --help` for per-shell install paths.
    #[command(after_help = indoc! {"
        EXAMPLES:
            # Bash (Linux)
            nordnet completions bash > ~/.local/share/bash-completion/completions/nordnet

            # Zsh — append to a directory in $fpath
            nordnet completions zsh > \"${fpath[1]}/_nordnet\"

            # Fish
            nordnet completions fish > ~/.config/fish/completions/nordnet.fish
    "})]
    Completions(cmd::completions::Cmd),
    /// `nordnet info` — system status (root API group).
    #[command(flatten)]
    Root(cmd::root::Cmd),
    /// `nordnet countries <op>` — country lookups.
    ///
    /// Static reference data. List or look up by ISO code.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet countries list
            nordnet countries get SE
            nordnet countries get SE,NO,DK
    "})]
    Countries {
        #[command(subcommand)]
        cmd: cmd::countries::Cmd,
    },
    /// `nordnet tick-sizes <op>` — tick-size table lookups.
    ///
    /// Static reference data describing the price increments allowed for
    /// each market's order book.
    #[command(name = "tick-sizes", after_help = indoc! {"
        EXAMPLES:
            nordnet tick-sizes list
            nordnet tick-sizes get 1
    "})]
    TickSizes {
        #[command(subcommand)]
        cmd: cmd::tick_sizes::Cmd,
    },
    /// `nordnet markets <op>` — market lookups.
    ///
    /// Static reference data: every Nordnet-tradable exchange/venue.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet markets list
            nordnet markets get 11   # Stockholm (XSTO)
    "})]
    Markets {
        #[command(subcommand)]
        cmd: cmd::markets::Cmd,
    },
    /// `nordnet news <op>` — news source + article lookups.
    ///
    /// Authenticated. The deprecated `GET /news` listing op is not
    /// surfaced — fetch articles by ID instead.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet news sources
            nordnet news get 12345678
    "})]
    News {
        #[command(subcommand)]
        cmd: cmd::news::Cmd,
    },
    /// `nordnet auth <op>` — authentication: login persists a session
    /// to disk so subsequent commands run authenticated automatically.
    ///
    /// `auth login` runs the SSH-key challenge round-trip and writes
    /// `<config_dir>/nordnet/session.toml` (mode 0600 on Unix). Override
    /// order for the active session: `--session-key` flag → env
    /// `NORDNET_SESSION_KEY` → on-disk session file.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet auth login
            nordnet auth status
            nordnet auth refresh
            nordnet auth logout
    "})]
    Auth {
        #[command(subcommand)]
        cmd: cmd::auth::Cmd,
    },
    /// `nordnet accounts <op>` — accounts, ledgers, positions, returns, trades.
    ///
    /// Authenticated. Account IDs are integers visible from
    /// `nordnet accounts list`.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet accounts list
            nordnet accounts info 12345
            nordnet accounts positions 12345 --fields id,instrument,qty
            nordnet accounts trades 12345 --days 7
    "})]
    Accounts {
        #[command(subcommand)]
        cmd: cmd::accounts::Cmd,
    },
    /// `nordnet tradables <op>` — tradable info / trades / suitability.
    ///
    /// Tradable keys are formatted as `<market_id>:<identifier>` (e.g.
    /// `11:101` for ERIC B on Stockholm). The market_id comes from
    /// `nordnet markets list`; the identifier from
    /// `nordnet search <query>` or `nordnet instruments lookup`.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet tradables info 11:101
            nordnet tradables trades 11:101 --count 10
            nordnet tradables suitability 11:101
    "})]
    Tradables {
        #[command(subcommand)]
        cmd: cmd::tradables::Cmd,
    },
    /// `nordnet search <query>` — top-level instrument search.
    #[command(flatten)]
    Search(cmd::main_search::Cmd),
    /// `nordnet instruments <op>` — instrument lookups + leverage queries.
    ///
    /// Authenticated. Instrument IDs come from `nordnet search` or
    /// `nordnet instruments lookup`.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet instruments lookup market_id_identifier 11:101
            nordnet instruments get 16099051
            nordnet instruments types
            nordnet instruments leverages 16099051 --currency SEK
    "})]
    Instruments {
        #[command(subcommand)]
        cmd: cmd::instruments::Cmd,
    },
    /// `nordnet instrument-search <op>` — attribute + entity-list searches.
    ///
    /// Authenticated. Each list type (stocks, bull/bear, mini-futures,
    /// unlimited turbos) accepts the same shape of pagination + sort
    /// arguments. Use `attributes` first to discover what's filterable
    /// and returnable.
    #[command(name = "instrument-search", after_help = indoc! {"
        EXAMPLES:
            nordnet instrument-search attributes --entity-type STOCKLIST
            nordnet instrument-search stocklist --free-text-search ericsson
            nordnet instrument-search bull-bear-list --limit 25
    "})]
    InstrumentSearch {
        #[command(subcommand)]
        cmd: cmd::instrument_search::Cmd,
    },
    /// `nordnet orders <op>` — list / place / modify / activate / cancel.
    ///
    /// Authenticated. Place returns the new order_id; modify, activate,
    /// and cancel are addressed by `<accid> <order_id>`. Use
    /// `nordnet orders list <accid>` to discover existing order IDs.
    #[cfg(feature = "orders-cli")]
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet orders list 12345
            nordnet orders place 12345 --market-id 11 --side BUY \\
                --volume 10 --price 101.50 --currency SEK \\
                --order-type LIMIT --identifier 101
            nordnet orders modify 12345 67890 --price 102.00 --currency SEK
            nordnet orders cancel 12345 67890
    "})]
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
        Command::Completions(c) => c.run(),
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
