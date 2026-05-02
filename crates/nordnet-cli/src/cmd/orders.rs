//! Foundation-owned dispatcher for the `nordnet orders ...` namespace.
//!
//! Locked after Phase 0 per PROCESS.md §"Phase 4 — CLI surface". Two
//! Phase 4 implementers (`orders_read` and `orders_write`) each define
//! their own `Cmd` enum + `run` method; this file glues them under the
//! single `nordnet orders ...` user-facing namespace with zero coupling
//! between the two implementer files.
//!
//! The body is gated on `feature = "orders-cli"` because `orders_read`
//! and `orders_write` modules are not created until Phase 4 — without
//! the gate, Phase 0 would not compile. Phase 4 enables the feature in
//! the crate's `Cargo.toml`. The shape of the gated code is the locked
//! contract; do not change it.

#[cfg(feature = "orders-cli")]
#[derive(Debug, clap::Subcommand)]
// `OrdersCmd::Write` carries `orders_write::Cmd::Place(PlaceArgs)`, which
// has 15 fields (full PlaceOrderRequest). Boxing the variant doesn't play
// well with clap's `#[command(flatten)]` derive plumbing, and the size
// only matters for runtime perf — we instantiate this enum exactly once
// per CLI invocation, so the heap cost of an indirection would buy
// nothing.
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
