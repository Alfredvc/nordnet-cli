//! Dispatcher for the `nordnet orders ...` namespace.
//!
//! `orders_read` and `orders_write` each define their own `Cmd` enum +
//! `run` method; this file glues them under the single
//! `nordnet orders ...` user-facing namespace.

#[cfg(feature = "orders-cli")]
#[derive(Debug, clap::Subcommand)]
// `OrdersCmd::Write` carries `orders_write::Cmd::Place(PlaceArgs)`, which
// has 15 fields (full `PlaceOrderRequest`). Boxing the variant doesn't
// play well with clap's `#[command(flatten)]` derive plumbing, and the
// enum is instantiated once per CLI invocation — heap indirection would
// buy nothing.
#[allow(clippy::large_enum_variant)]
pub enum OrdersCmd {
    #[command(flatten)]
    Read(crate::cmd::orders_read::Cmd),
    #[command(flatten)]
    Write(crate::cmd::orders_write::Cmd),
}

#[cfg(feature = "orders-cli")]
impl OrdersCmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Self::Read(c) => c.run(client, fields).await,
            Self::Write(c) => c.run(client, fields).await,
        }
    }
}
