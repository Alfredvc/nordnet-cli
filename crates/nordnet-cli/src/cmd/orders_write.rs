//! `nordnet orders place|modify|activate|cancel` — write-side orders subcommands.
//!
//! Lives at `crate::cmd::orders_write::Cmd` because the dispatcher in
//! `cmd/orders.rs` (gated behind `feature = "orders-cli"`) flattens this
//! into the top-level `nordnet orders` namespace alongside
//! `crate::cmd::orders_read::Cmd`.

use clap::{Args, Subcommand};
use indoc::indoc;
use nordnet_model::ids::{AccountId, MarketId, OrderId, TradableId};
use nordnet_model::models::orders::{
    ModifyOrderRequest, OrderActivationCondition, OrderSide, OrderType, PlaceOrderRequest,
};
use nordnet_model::models::shared::Currency;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Write-side subcommands for the `orders` namespace.
#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Place a new order (POST /accounts/{accid}/orders).
    ///
    /// Real money. Requires `--market-id`, `--side`, `--volume`. Most
    /// other fields are optional with API-side defaults; supply
    /// `--price` + `--currency` for limit orders, `--trigger-value` +
    /// `--activation-condition` for stops. Returns the new `order_id`.
    #[command(after_help = indoc! {"
        EXAMPLES:
            # Limit buy 10 ERIC B @ 101.50 SEK
            nordnet orders place 12345 --market-id 11 --side BUY \\
                --volume 10 --price 101.50 --currency SEK \\
                --order-type LIMIT --identifier 101

            # Stop-loss sell at trigger 95.00, market on activation
            nordnet orders place 12345 --market-id 11 --side SELL \\
                --volume 10 --identifier 101 \\
                --activation-condition STOP_ACTPRICE \\
                --trigger-value 95.00 --trigger-condition '<='
    "})]
    Place(PlaceArgs),
    /// Modify an existing order's price/volume (PUT /accounts/{accid}/orders/{order_id}).
    ///
    /// Real money. At least one of `--price` (with `--currency`),
    /// `--volume`, or `--open-volume` must be set. Use
    /// `nordnet orders list <accid>` to discover order IDs.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet orders modify 12345 67890 --price 102.00 --currency SEK
            nordnet orders modify 12345 67890 --volume 20
    "})]
    Modify(ModifyArgs),
    /// Activate an inactive order (PUT /accounts/{accid}/orders/{order_id}/activate).
    ///
    /// Real money. Promotes a held order to active state — only
    /// applicable to orders that were placed in an inactive state.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet orders activate 12345 67890
    "})]
    Activate(OrderRefArgs),
    /// Cancel an order (DELETE /accounts/{accid}/orders/{order_id}).
    ///
    /// Real money. Idempotent on the server — re-cancelling a
    /// cancelled order returns the same response shape.
    #[command(after_help = indoc! {"
        EXAMPLES:
            nordnet orders cancel 12345 67890
    "})]
    Cancel(OrderRefArgs),
}

/// Shared positional arguments for order operations that only need an account
/// and an order ID.
#[derive(Debug, Args)]
pub struct OrderRefArgs {
    /// Account ID.
    pub accid: i64,
    /// Order ID.
    pub order_id: i64,
}

/// Arguments for the `modify` subcommand.
#[derive(Debug, Args)]
pub struct ModifyArgs {
    /// Account ID.
    pub accid: i64,
    /// Order ID.
    pub order_id: i64,
    /// New price (decimal, e.g. `101.50`). Currency must also be set when
    /// changing the price.
    #[arg(long)]
    pub price: Option<String>,
    /// Currency code (e.g. `SEK`). Required when `--price` is set.
    #[arg(long)]
    pub currency: Option<String>,
    /// New total volume.
    #[arg(long)]
    pub volume: Option<i64>,
    /// New open (visible iceberg) volume.
    #[arg(long)]
    pub open_volume: Option<i64>,
}

/// Arguments for the `place` subcommand.
#[derive(Debug, Args)]
pub struct PlaceArgs {
    /// Account ID.
    pub accid: i64,
    /// Nordnet market identifier (required).
    #[arg(long)]
    pub market_id: i64,
    /// Order side: `BUY` or `SELL` (required).
    #[arg(long, value_parser = parse_side)]
    pub side: OrderSide,
    /// Order volume (required).
    #[arg(long)]
    pub volume: i64,
    /// Activation condition for stop-loss orders (e.g. `STOP_ACTPRICE`,
    /// `STOP_ACTPRICE_PERC`, `MANUAL`, `OCO_STOP_ACTPRICE`).
    #[arg(long, value_parser = parse_activation)]
    pub activation_condition: Option<OrderActivationCondition>,
    /// Currency code (e.g. `SEK`).
    #[arg(long)]
    pub currency: Option<String>,
    /// If `true`, the order is applicable for US pre-market trading.
    #[arg(long)]
    pub extended_hours: Option<bool>,
    /// Nordnet tradable identifier string.
    #[arg(long)]
    pub identifier: Option<String>,
    /// The visible part of an iceberg order. Only allowed for `LIMIT` /
    /// `NORMAL` order types.
    #[arg(long)]
    pub open_volume: Option<i64>,
    /// Order type (e.g. `NORMAL`, `LIMIT`, `FAK`, `FOK`, `STOP_LIMIT`,
    /// `STOP_TRAILING`, `OCO`).
    #[arg(long, value_parser = parse_order_type)]
    pub order_type: Option<OrderType>,
    /// Price limit (decimal, e.g. `101.50`).
    #[arg(long)]
    pub price: Option<String>,
    /// Free-text reference for the order.
    #[arg(long)]
    pub reference: Option<String>,
    /// Target value — only for `STOP_ACTPRICE_PERC` / `OCO_STOP_ACTPRICE`
    /// activation conditions (decimal, percentage points).
    #[arg(long)]
    pub target_value: Option<String>,
    /// Trigger condition: `<=` or `>=`.
    #[arg(long)]
    pub trigger_condition: Option<String>,
    /// Trigger value (decimal). For `STOP_ACTPRICE_PERC`: percentage points
    /// (minimum 1); for `STOP_ACTPRICE`: a fixed price.
    #[arg(long)]
    pub trigger_value: Option<String>,
    /// Cancel date formatted as `YYYY-MM-DD`.
    #[arg(long)]
    pub valid_until: Option<String>,
}

// ---------------------------------------------------------------------------
// Parsers
// ---------------------------------------------------------------------------

fn parse_decimal_opt(s: &Option<String>) -> anyhow::Result<Option<Decimal>> {
    s.as_deref()
        .map(|v| {
            Decimal::from_str(v).map_err(|e| anyhow::anyhow!("invalid decimal {:?}: {}", v, e))
        })
        .transpose()
}

fn parse_side(s: &str) -> Result<OrderSide, String> {
    serde_json::from_value::<OrderSide>(serde_json::Value::String(s.to_owned()))
        .map_err(|e| format!("invalid side {:?}: {}", s, e))
}

fn parse_order_type(s: &str) -> Result<OrderType, String> {
    serde_json::from_value::<OrderType>(serde_json::Value::String(s.to_owned()))
        .map_err(|e| format!("invalid order_type {:?}: {}", s, e))
}

fn parse_activation(s: &str) -> Result<OrderActivationCondition, String> {
    serde_json::from_value::<OrderActivationCondition>(serde_json::Value::String(s.to_owned()))
        .map_err(|e| format!("invalid activation_condition {:?}: {}", s, e))
}

// ---------------------------------------------------------------------------
// run
// ---------------------------------------------------------------------------

impl Cmd {
    pub async fn run(self, client: &nordnet_api::Client, fields: &[String]) -> anyhow::Result<()> {
        match self {
            Cmd::Place(a) => {
                let req = PlaceOrderRequest {
                    activation_condition: a.activation_condition,
                    currency: a.currency.as_deref().map(Currency::from),
                    extended_hours: a.extended_hours,
                    identifier: a.identifier.map(TradableId::from),
                    market_id: MarketId::from(a.market_id),
                    open_volume: a.open_volume,
                    order_type: a.order_type,
                    price: parse_decimal_opt(&a.price)?,
                    reference: a.reference,
                    side: a.side,
                    target_value: parse_decimal_opt(&a.target_value)?,
                    trigger_condition: a.trigger_condition,
                    trigger_value: parse_decimal_opt(&a.trigger_value)?,
                    valid_until: a.valid_until,
                    volume: a.volume,
                };
                let r = client.place_order(AccountId::from(a.accid), &req).await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Modify(a) => {
                let req = ModifyOrderRequest {
                    currency: a.currency.as_deref().map(Currency::from),
                    open_volume: a.open_volume,
                    price: parse_decimal_opt(&a.price)?,
                    volume: a.volume,
                };
                let r = client
                    .modify_order(AccountId::from(a.accid), OrderId::from(a.order_id), &req)
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Activate(a) => {
                let r = client
                    .activate_order(AccountId::from(a.accid), OrderId::from(a.order_id))
                    .await?;
                crate::output::emit(&r, fields)?;
            }
            Cmd::Cancel(a) => {
                let r = client
                    .cancel_order(AccountId::from(a.accid), OrderId::from(a.order_id))
                    .await?;
                crate::output::emit(&r, fields)?;
            }
        }
        Ok(())
    }
}
