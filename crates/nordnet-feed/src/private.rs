//! Private feed event payload types.
//!
//! Auto-pushed after private-feed login (no subscription needed).
//! Currently only `OrderEvent` is typed — `Trade` payload is shipped
//! as `serde_json::Value` (see PrivateEvent::TradeRaw in event.rs)
//! because its schema isn't in public Nordnet docs (Decision §12).
//!
//! Every wire-string field uses the `Known + Unknown(String)` typed-
//! enum pattern (Decision §10) — known variants from
//! `docs-source/nordnet-api-v2.html`, unknown values absorbed without
//! deserialize errors so Nordnet can add new states without breaking
//! us.

use nordnet_model::ids::{AccountId, MarketId, OrderId, TradableId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// An order event pushed from the private feed after login.
///
/// Schema: the official Nordnet example payload (see design spec
/// §"Private feed event types") plus the `_definitions/Order.md`
/// reference in `docs-source/nordnet-api-v2.html`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct OrderEvent {
    pub order_id: OrderId,
    pub accno: AccountId,
    pub accid: AccountId,
    pub tradable: Tradable,
    pub side: Side,
    pub volume: Decimal,
    pub price: PriceWithCurrency,
    pub volume_condition: VolumeCondition,
    pub validity: Validity,
    pub activation_condition: ActivationCondition,
    pub order_state: OrderState,
    pub action_state: ActionState,
    pub order_type: OrderType,
    /// UNIX-millisecond epoch of the last server-side modification.
    pub modified: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// Tradable identifier nested inside [`OrderEvent`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Tradable {
    pub market_id: MarketId,
    pub identifier: TradableId,
}

/// Price with currency nested inside [`OrderEvent`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PriceWithCurrency {
    pub value: Decimal,
    pub currency: String,
}

/// Validity period nested inside [`OrderEvent`].
///
/// The wire field `type` is mapped to the Rust field `kind` to avoid
/// the keyword collision.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Validity {
    #[serde(rename = "type")]
    pub kind: ValidityKind,
    pub valid_until: i64,
}

/// Activation condition nested inside [`OrderEvent`].
///
/// The wire field `type` is mapped to the Rust field `kind` to avoid
/// the keyword collision.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ActivationCondition {
    #[serde(rename = "type")]
    pub kind: ActivationConditionKind,
}

// ===== Typed enums (Known + Unknown split) =====
//
// Every outer enum is `#[serde(untagged)]` so serde attempts to
// deserialize the inner `Known` enum first; if that fails (unknown
// variant), the `Unknown(String)` arm catches the raw wire string.
// This preserves round-trip fidelity: unknown values come back out
// as-is.

/// Buy or sell side.
///
/// Known variants: `BUY`, `SELL` (from HTML enum definition).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Side {
    Known(KnownSide),
    Unknown(String),
}

/// Documented `Side` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum KnownSide {
    #[serde(rename = "BUY")]
    Buy,
    #[serde(rename = "SELL")]
    Sell,
}

/// Volume condition on the order.
///
/// Known variants from `docs-source/nordnet-api-v2.html`:
/// - `NORMAL` — all types of fills accepted.
/// - `ALL_OR_NOTHING` — partial fills not accepted.
///
/// Additional values from the spec seed (feed wire):
/// - `AON` — all-or-none (alternate spelling).
/// - `FOK` — fill-or-kill.
/// - `IOC` — immediate-or-cancel.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum VolumeCondition {
    Known(KnownVolumeCondition),
    Unknown(String),
}

/// Documented `VolumeCondition` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum KnownVolumeCondition {
    #[serde(rename = "NORMAL")]
    Normal,
    #[serde(rename = "ALL_OR_NOTHING")]
    AllOrNothing,
    #[serde(rename = "AON")]
    AllOrNone,
    #[serde(rename = "FOK")]
    FillOrKill,
    #[serde(rename = "IOC")]
    ImmediateOrCancel,
}

/// Validity type for the order.
///
/// Known variants from `docs-source/nordnet-api-v2.html` (`_validity`
/// section): `DAY`, `UNTIL_DATE`, `EXTENDED_HOURS`, `IMMEDIATE`.
///
/// Additional values from the spec seed (feed wire): `GTC`, `GTD`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ValidityKind {
    Known(KnownValidityKind),
    Unknown(String),
}

/// Documented `ValidityKind` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum KnownValidityKind {
    #[serde(rename = "DAY")]
    Day,
    #[serde(rename = "UNTIL_DATE")]
    UntilDate,
    #[serde(rename = "EXTENDED_HOURS")]
    ExtendedHours,
    #[serde(rename = "IMMEDIATE")]
    Immediate,
    #[serde(rename = "GTC")]
    GoodTillCancel,
    #[serde(rename = "GTD")]
    GoodTillDate,
    #[serde(rename = "IOC")]
    ImmediateOrCancel,
}

/// Activation condition kind for stop-loss orders.
///
/// Known variants from `docs-source/nordnet-api-v2.html`
/// (`_activationcondition` section): `NONE`, `MANUAL`,
/// `STOP_ACTPRICE_PERC`, `STOP_ACTPRICE`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ActivationConditionKind {
    Known(KnownActivationConditionKind),
    Unknown(String),
}

/// Documented `ActivationConditionKind` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum KnownActivationConditionKind {
    /// No activation condition — sent directly to the market.
    #[serde(rename = "NONE")]
    None,
    /// Order is inactive until manually activated by the customer.
    #[serde(rename = "MANUAL")]
    Manual,
    /// Trailing stop-loss — activated when price changes by a given %.
    #[serde(rename = "STOP_ACTPRICE_PERC")]
    StopActpricePerc,
    /// Activated when market price reaches a trigger price.
    #[serde(rename = "STOP_ACTPRICE")]
    StopActprice,
}

/// State of the order.
///
/// Known variants from `docs-source/nordnet-api-v2.html` (`_order`
/// section): `DELETED`, `LOCAL`, `ON_MARKET`, `LOCKED`.
///
/// Additional values from the spec seed (feed wire): `ACTIVE`,
/// `FILLED`, `CANCELLED`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OrderState {
    Known(KnownOrderState),
    Unknown(String),
}

/// Documented `OrderState` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum KnownOrderState {
    #[serde(rename = "DELETED")]
    Deleted,
    #[serde(rename = "LOCAL")]
    Local,
    #[serde(rename = "ON_MARKET")]
    OnMarket,
    #[serde(rename = "LOCKED")]
    Locked,
    /// Seed value from feed wire — not in HTML REST docs.
    #[serde(rename = "ACTIVE")]
    Active,
    /// Seed value from feed wire — not in HTML REST docs.
    #[serde(rename = "FILLED")]
    Filled,
    /// Seed value from feed wire — not in HTML REST docs.
    #[serde(rename = "CANCELLED")]
    Cancelled,
}

/// State of the last action performed on the order.
///
/// Known variants from `docs-source/nordnet-api-v2.html` (`_order`
/// `action_state` field): `DEL_FAIL`, `DEL_PEND`, `DEL_CONF`,
/// `DEL_PUSH`, `INS_FAIL`, `INS_PEND`, `INS_CONF`, `INS_STOP`,
/// `MOD_FAIL`, `MOD_PEND`, `MOD_PUSH`, `INS_WAIT`, `MOD_WAIT`,
/// `DEL_WAIT`, `MOD_CONF`.
///
/// Additional value from the spec seed: `ACKED`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ActionState {
    Known(KnownActionState),
    Unknown(String),
}

/// Documented `ActionState` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum KnownActionState {
    /// Delete request failed; order still active on market.
    #[serde(rename = "DEL_FAIL")]
    DeleteFailed,
    /// Delete request in progress, unconfirmed by market.
    #[serde(rename = "DEL_PEND")]
    DeletePending,
    /// Delete confirmed by market.
    #[serde(rename = "DEL_CONF")]
    DeleteConfirmed,
    /// Deleted by the market.
    #[serde(rename = "DEL_PUSH")]
    DeletedByMarket,
    /// Insert failed.
    #[serde(rename = "INS_FAIL")]
    InsertFailed,
    /// Pending insert.
    #[serde(rename = "INS_PEND")]
    InsertPending,
    /// Confirmed insert.
    #[serde(rename = "INS_CONF")]
    InsertConfirmed,
    /// Inserted into Nordnet system and stopped (inactive / not triggered).
    #[serde(rename = "INS_STOP")]
    InsertStopped,
    /// Modification failed; previous order values still valid.
    #[serde(rename = "MOD_FAIL")]
    ModifyFailed,
    /// Modification in progress, waiting confirmation from market.
    #[serde(rename = "MOD_PEND")]
    ModifyPending,
    /// Modified by the market.
    #[serde(rename = "MOD_PUSH")]
    ModifiedByMarket,
    /// Insert waiting for market opening.
    #[serde(rename = "INS_WAIT")]
    InsertWaiting,
    /// Modification of order on market, waiting for market opening.
    #[serde(rename = "MOD_WAIT")]
    ModifyWaiting,
    /// Delete of order on market, waiting for market opening.
    #[serde(rename = "DEL_WAIT")]
    DeleteWaiting,
    /// Modification confirmed by market.
    #[serde(rename = "MOD_CONF")]
    ModifyConfirmed,
    /// Seed value from feed wire — not in HTML REST docs.
    #[serde(rename = "ACKED")]
    Acked,
}

/// Order type.
///
/// Known variants from `docs-source/nordnet-api-v2.html`
/// (place_order `order_type` parameter): `FAK`, `FOK`, `NORMAL`,
/// `LIMIT`, `STOP_LIMIT`, `STOP_TRAILING`, `OCO`.
///
/// Additional values from the spec seed (feed wire): `MARKET`,
/// `STOP_LOSS`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OrderType {
    Known(KnownOrderType),
    Unknown(String),
}

/// Documented `OrderType` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum KnownOrderType {
    /// Fill-and-kill.
    #[serde(rename = "FAK")]
    Fak,
    /// Fill-or-kill.
    #[serde(rename = "FOK")]
    Fok,
    /// Normal (deprecated default; system guesses based on parameters).
    #[serde(rename = "NORMAL")]
    Normal,
    /// Limit order.
    #[serde(rename = "LIMIT")]
    Limit,
    /// Stop-limit order.
    #[serde(rename = "STOP_LIMIT")]
    StopLimit,
    /// Stop-trailing order.
    #[serde(rename = "STOP_TRAILING")]
    StopTrailing,
    /// One-cancels-other.
    #[serde(rename = "OCO")]
    Oco,
    /// Seed value from feed wire — not in HTML REST docs.
    #[serde(rename = "MARKET")]
    Market,
    /// Seed value from feed wire — not in HTML REST docs.
    #[serde(rename = "STOP_LOSS")]
    StopLoss,
}
