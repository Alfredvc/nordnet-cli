//! `nordnet accounts` — account info, ledgers, positions, returns, trades.

use clap::{Args, Subcommand};
use nordnet_api::ids::AccountId;
use nordnet_api::resources::accounts::{
    AccountInfoQuery, ListAccountTradesQuery, ListAccountsQuery, ListPositionsQuery,
    ReturnsTodayQuery,
};

/// Subcommands for the `accounts` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List accessible accounts (GET /accounts).
    List(ListArgs),
    /// Get account info (GET /accounts/{accid}/info).
    Info(InfoArgs),
    /// Get ledger information (GET /accounts/{accid}/ledgers).
    Ledgers(AccidArg),
    /// List positions held in an account (GET /accounts/{accid}/positions).
    Positions(PositionsArgs),
    /// Today's return transactions (GET /accounts/{accid}/returns/transactions/today).
    ReturnsToday(ReturnsArgs),
    /// List trades for an account (GET /accounts/{accid}/trades).
    Trades(TradesArgs),
}

/// Arguments for the `list` subcommand.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Include credit accounts in the response (server default: false).
    /// Pass `--include-credit-accounts=true` or `--include-credit-accounts=false`.
    #[arg(long)]
    pub include_credit_accounts: Option<bool>,
}

/// Bare account-ID argument shared by subcommands that only need an account ID.
#[derive(Debug, Args)]
pub struct AccidArg {
    /// Account ID (integer).
    pub accid: i64,
}

/// Arguments for the `info` subcommand.
#[derive(Debug, Args)]
pub struct InfoArgs {
    /// Account ID (integer).
    pub accid: i64,
    /// Include the interest rate field in the response (server default: true).
    #[arg(long)]
    pub include_interest_rate: Option<bool>,
    /// Include the short-position margin field in the response (server default: true).
    #[arg(long)]
    pub include_short_pos_margin: Option<bool>,
}

/// Arguments for the `positions` subcommand.
#[derive(Debug, Args)]
pub struct PositionsArgs {
    /// Account ID (integer).
    pub accid: i64,
    /// Include instrument loan positions (server default: false).
    #[arg(long)]
    pub include_instrument_loans: Option<bool>,
    /// Include intraday limit information (server default: false).
    #[arg(long)]
    pub include_intraday_limit: Option<bool>,
}

/// Arguments for the `returns-today` subcommand.
#[derive(Debug, Args)]
pub struct ReturnsArgs {
    /// Account ID (integer).
    pub accid: i64,
    /// Include credit account transactions (server default: true).
    #[arg(long)]
    pub include_credit_account: Option<bool>,
}

/// Arguments for the `trades` subcommand.
#[derive(Debug, Args)]
pub struct TradesArgs {
    /// Account ID (integer).
    pub accid: i64,
    /// Number of days back to fetch trades for (server default: 0; max 7).
    #[arg(long)]
    pub days: Option<i64>,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::List(a) => {
                let r = client
                    .list_accounts(ListAccountsQuery {
                        include_credit_accounts: a.include_credit_accounts,
                    })
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Info(a) => {
                let r = client
                    .get_account_info(
                        AccountId::from(a.accid),
                        AccountInfoQuery {
                            include_interest_rate: a.include_interest_rate,
                            include_short_pos_margin: a.include_short_pos_margin,
                        },
                    )
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Ledgers(a) => {
                let r = client.list_ledgers(AccountId::from(a.accid)).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Positions(a) => {
                let r = client
                    .list_positions(
                        AccountId::from(a.accid),
                        ListPositionsQuery {
                            include_instrument_loans: a.include_instrument_loans,
                            include_intraday_limit: a.include_intraday_limit,
                        },
                    )
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::ReturnsToday(a) => {
                let r = client
                    .get_returns_today(
                        AccountId::from(a.accid),
                        ReturnsTodayQuery {
                            include_credit_account: a.include_credit_account,
                        },
                    )
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Trades(a) => {
                let r = client
                    .list_account_trades(
                        AccountId::from(a.accid),
                        ListAccountTradesQuery { days: a.days },
                    )
                    .await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}
