//! `nordnet tradables` — tradable info / trades / suitability lookups.
//!
//! # Implemented ops
//!
//! - `info`        → `client.get_tradable_info(&key)`
//! - `trades`      → `client.list_tradable_trades(&key, count)`
//! - `suitability` → `client.get_suitability(&key)`

use clap::{Args, Subcommand};
use nordnet_model::ids::{MarketId, TradableId};
use nordnet_model::models::tradables::TradableKey;

/// Subcommands for the `tradables` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Get trading calendar and allowed trading types for a tradable.
    Info(KeyArgs),
    /// List public trades for a tradable.
    Trades(TradesArgs),
    /// Get customer trading eligibility for a tradable.
    Suitability(KeyArgs),
}

/// Arguments carrying only a tradable key.
#[derive(Debug, Args)]
pub struct KeyArgs {
    /// Tradable key in `<market_id>:<identifier>` form (e.g. `11:101` for ERIC B).
    pub key: String,
}

/// Arguments for the `trades` subcommand.
#[derive(Debug, Args)]
pub struct TradesArgs {
    /// Tradable key in `<market_id>:<identifier>` form (e.g. `11:101` for ERIC B).
    pub key: String,
    /// Number of trades to return: a positive integer or the literal `all`. Defaults to `5`.
    #[arg(long)]
    pub count: Option<String>,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::Info(a) => {
                let key = parse_tradable_key(&a.key)?;
                let r = client.get_tradable_info(&key).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Trades(a) => {
                let key = parse_tradable_key(&a.key)?;
                let r = client
                    .list_tradable_trades(&key, a.count.as_deref())
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Suitability(a) => {
                let key = parse_tradable_key(&a.key)?;
                let r = client.get_suitability(&key).await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}

/// Parse a `<market_id>:<identifier>` string into a [`TradableKey`].
fn parse_tradable_key(s: &str) -> anyhow::Result<TradableKey> {
    let (market, ident) = s
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("expected <market_id>:<identifier>, got {s:?}"))?;
    let market_id: i64 = market
        .parse()
        .map_err(|_| anyhow::anyhow!("market_id must be an integer in {s:?}"))?;
    Ok(TradableKey {
        market_id: MarketId(market_id),
        identifier: TradableId(ident.into()),
    })
}
