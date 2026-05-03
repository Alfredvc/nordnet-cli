//! `nordnet orders list` — read-side orders subcommand.
//!
//! Lives at `crate::cmd::orders_read::Cmd` because the foundation-locked
//! dispatcher in `cmd/orders.rs` (gated behind `feature = "orders-cli"`)
//! flattens this into the top-level `nordnet orders` namespace alongside
//! `crate::cmd::orders_write::Cmd`.

use clap::{Args, Subcommand};
use nordnet_model::ids::AccountId;

/// Read-side subcommands for the `nordnet orders` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List orders belonging to the given account (GET /accounts/{accid}/orders).
    ///
    /// Returns an empty list when there are no orders (the API returns 204
    /// No Content in that case).
    List(ListArgs),
}

/// Arguments for `nordnet orders list`.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Account ID (integer).
    pub accid: i64,
    /// Include orders that were deleted today (server default: false).
    ///
    /// Pass `--deleted=true` to see deleted orders alongside active ones.
    #[arg(long)]
    pub deleted: Option<bool>,
}

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::List(a) => {
                let r = client
                    .list_orders(AccountId::from(a.accid), a.deleted)
                    .await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}
