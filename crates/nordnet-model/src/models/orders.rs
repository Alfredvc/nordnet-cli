//! Models for the `orders` resource group.
//!
//! Derived from the Nordnet `Order`, `OrderReply`, `ActivationCondition`,
//! `Validity`, `Amount` (modelled as [`OrderAmount`]) and `TradableId`
//! schemas plus the per-op parameter tables.
//!
//!
//! ## Doc notes
//!
//! - **Wire format for `place_order` / `modify_order`: `application/x-www-form-urlencoded`.**
//!   The parameter tables describe every body parameter as Swagger 2.0
//!   `FormData`. The resource layer sends these via
//!   [`crate::Client::post_form`] / [`crate::Client::put_form`]. The
//!   request structs intentionally do NOT carry the
//!   `rust_decimal::serde::arbitrary_precision_option` adapter on
//!   `Decimal` fields (it serializes via a `serde_json` magic struct that
//!   `serde_urlencoded` rejects with `unsupported value`). The default
//!   `Decimal` `Display`-based serialization produces a decimal string in
//!   both formats, which is what the live API expects on the wire.
//! - This module's [`OrderType`] enum is the closed set of values
//!   accepted by `place_order` on the request side. The structurally
//!   different `(name, type)` pair in the `tradables` group lives there
//!   as [`crate::models::tradables::AllowedOrderType`].
//! - `ActivationCondition` exists in two distinct shapes in the docs:
//!   the **request** form is a single enum string sent as the
//!   `activation_condition` form field on `place_order`; the **response**
//!   form is a struct nested inside `Order`. We model both:
//!   [`OrderActivationCondition`] (enum, request) and
//!   [`ActivationCondition`] (struct, response).
//! - `Order.modified` is documented as `integer(int64)` UNIX-millisecond
//!   epoch. Kept as `i64`. Same precedent as `tradables::PublicTrade`.
//! - Several numeric fields on `Order` are typed as `number(double)` in
//!   the docs (`open_volume`, `traded_volume`, `volume`). They are
//!   modelled as [`rust_decimal::Decimal`] with the
//!   `arbitrary_precision` adapter — never `f64`. Because of this `Order`
//!   cannot derive [`Eq`].
//! - The `Validity` definition models `valid_until` as
//!   `integer(int64)` (UNIX ms). Kept as `i64`.

use crate::ids::{AccountId, MarketId, OrderId, TradableId};
use crate::models::shared::Currency;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Money shape
// ---------------------------------------------------------------------------

/// Re-export of the shared `{currency, value}` amount type, kept under the
/// in-group spelling `OrderAmount`.
pub use crate::models::shared::AmountWithCurrency as OrderAmount;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Composite key identifying a tradable (market + symbol). Wire form is
/// the nested object `{ "identifier": "...", "market_id": ... }` per the
/// documented `TradableId` schema.
///
/// Note: this is the *response-body* nested representation. The
/// `tradables::TradableKey` value (which renders as `market_id:identifier`
/// for path slots) is unrelated and lives in its own module.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct OrderTradable {
    /// The Nordnet tradable identifier.
    pub identifier: TradableId,
    /// The Nordnet market identifier.
    pub market_id: MarketId,
}

/// Activation-condition object nested inside [`Order`] (response shape).
///
/// `type` is the documented enum; renamed from the wire `type` (Rust
/// keyword) via `#[serde(rename)]`.
///
/// `trailing_value` and `trigger_value` are `number(double)` per the
/// schema; modelled as `Decimal`, so this struct cannot derive [`Eq`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ActivationCondition {
    /// The fix point that the trigger_value and target_value percent is
    /// calculated from. Only used when type is `STOP_ACTPRICE_PERC`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "rust_decimal::serde::arbitrary_precision_option"
    )]
    pub trailing_value: Option<Decimal>,
    /// The comparison that should be used on `trigger_value`. Valid values
    /// are `<=` (less than or equal to) or `>=` (greater than or equal to).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_condition: Option<String>,
    /// The trigger value. If type is `STOP_ACTPRICE_PERC` the value is
    /// given in percentage points. If type is `STOP_ACTPRICE` the value is
    /// a fixed price.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "rust_decimal::serde::arbitrary_precision_option"
    )]
    pub trigger_value: Option<Decimal>,
    /// The stop-loss activation condition. Wire field name is `type`.
    #[serde(rename = "type")]
    pub r#type: ActivationConditionType,
}

/// Activation-condition `type` value as it appears in [`Order`] (response).
///
/// Distinct from [`OrderActivationCondition`] which is the
/// **request-side** value sent on `place_order`. The two enums share the
/// non-`NONE` variants but the response form additionally documents
/// `NONE` for orders that have no activation condition.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActivationConditionType {
    /// No activation condition. Sent directly to the market.
    None,
    /// The order is inactive in the Nordnet system and is activated by the
    /// customer.
    Manual,
    /// Trailing stop-loss. The order is activated when the price changes
    /// by the given percentage.
    StopActpricePerc,
    /// The order is activated when the market price of the instrument
    /// reaches a trigger price.
    StopActprice,
}

/// `Validity` object nested inside [`Order`].
///
/// Schema: `_definitions/Validity.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Validity {
    /// Validity type. Wire field name is `type`.
    #[serde(rename = "type")]
    pub r#type: ValidityType,
    /// The cancel date, only used when type is `UNTIL_DATE`. UNIX
    /// timestamp in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<i64>,
}

/// Validity `type` value. Documented set: `DAY, UNTIL_DATE, EXTENDED_HOURS,
/// IMMEDIATE`.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidityType {
    /// Day order.
    Day,
    /// Cancel date set explicitly via `valid_until`.
    UntilDate,
    /// Order valid during US extended-hours trading.
    ExtendedHours,
    /// Immediate-or-cancel.
    Immediate,
}

/// Action state of the last action performed on an order. Documented set
/// per `_definitions/Order.md`.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionState {
    /// Delete request failed and the order is still active on the market.
    DelFail,
    /// Delete request in progress and unconfirmed by the market.
    DelPend,
    /// Delete confirmed by the market.
    DelConf,
    /// Deleted by the market.
    DelPush,
    /// Insert failed.
    InsFail,
    /// Pending insert.
    InsPend,
    /// Confirmed insert.
    InsConf,
    /// Inserted into the Nordnet system and stopped (inactive / not
    /// triggered stop-loss).
    InsStop,
    /// Modification failed; previous order values still valid.
    ModFail,
    /// Modification in progress and waiting confirmation from the market.
    ModPend,
    /// Modified by the market.
    ModPush,
    /// Insert waiting for market opening.
    InsWait,
    /// Modification of order on the market, waiting for market opening.
    ModWait,
    /// Delete of order on the market, waiting for market opening.
    DelWait,
    /// Modification confirmed by the market.
    ModConf,
}

/// Order-state value. Documented set per `_definitions/Order.md`.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderState {
    /// Order is deleted.
    Deleted,
    /// The order is offline/local and eligible for activation.
    Local,
    /// The order is active on the market.
    OnMarket,
    /// The order can't be modified by the customer.
    Locked,
}

/// `side` value used on both [`Order`] (response) and
/// [`PlaceOrderRequest`] (request body). Documented set: `BUY, SELL`.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderSide {
    /// Buy.
    Buy,
    /// Sell.
    Sell,
}

/// `price_condition` value documented on [`Order`].
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PriceCondition {
    /// The order is limited by the given price.
    Limit,
    /// The order is entered at the current market price. Not supported by
    /// most markets.
    AtMarket,
}

/// `volume_condition` value documented on [`Order`].
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VolumeCondition {
    /// All types of fills are accepted.
    Normal,
    /// Partial fills are not accepted.
    AllOrNothing,
}

/// One order belonging to an account.
///
/// Schema: `_definitions/Order.md`. Cannot derive [`Eq`] because
/// `open_volume`, `traded_volume`, `volume`, `price.amount` and the nested
/// `activation_condition` numeric fields are `Decimal`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Order {
    /// The account identifier. Optional per the schema (not applicable for
    /// partners).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accid: Option<AccountId>,
    /// The Nordnet account number. Always refers to a specific account.
    pub accno: i64,
    /// The state of the last action performed on the order.
    pub action_state: ActionState,
    /// The activation condition for stop-loss orders. Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation_condition: Option<ActivationCondition>,
    /// Last modification time of the order. UNIX timestamp in
    /// milliseconds.
    pub modified: i64,
    /// The open volume of an iceberg order. Optional per schema.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "rust_decimal::serde::arbitrary_precision_option"
    )]
    pub open_volume: Option<Decimal>,
    /// The Nordnet order identifier.
    pub order_id: OrderId,
    /// The state of the order.
    pub order_state: OrderState,
    /// The type of the order. The doc lists this as `string` (not as a
    /// closed enum on the response side), so it is kept as `String` here
    /// to admit any value the server might return — defensive against a
    /// drift between the place-order request enum and what the server
    /// later reports back.
    pub order_type: String,
    /// The price of the order.
    pub price: OrderAmount,
    /// The price condition on the order.
    pub price_condition: PriceCondition,
    /// Customer reference for the order. Free-text. Optional per schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// `BUY` or `SELL`.
    pub side: OrderSide,
    /// The tradable identifier (composite market + symbol).
    pub tradable: OrderTradable,
    /// The total traded volume of the order.
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub traded_volume: Decimal,
    /// The validity period for the order.
    pub validity: Validity,
    /// The original volume of the order.
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub volume: Decimal,
    /// The volume condition on the order.
    pub volume_condition: VolumeCondition,
}

/// Reply payload returned by every write operation
/// (`place`, `modify`, `activate`, `cancel`).
///
/// Schema: `_definitions/OrderReply.md`. Only `order_id` and `result_code`
/// are required; `action_state`, `order_state`, and `message` are
/// optional.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct OrderReply {
    /// The action state. Can be missing if the order fails the
    /// prevalidation and never enters the order system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_state: Option<ActionState>,
    /// Translated error message if `result_code` is not `OK`. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// The Nordnet order identifier.
    pub order_id: OrderId,
    /// The order state. Only returned for valid orders. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_state: Option<OrderState>,
    /// `OK` or an error code. Kept as `String` (the doc does not enumerate
    /// the full error-code set).
    pub result_code: String,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Order-type code as accepted by `place_order`. Documented set:
/// `FAK, FOK, NORMAL, LIMIT, STOP_LIMIT, STOP_TRAILING, OCO`.
///
/// `NORMAL` is documented as the default and as deprecated — clients
/// should pick the explicit type matching their intent.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    /// Fill-and-kill.
    Fak,
    /// Fill-or-kill.
    Fok,
    /// Normal (deprecated default; system guesses based on parameters).
    Normal,
    /// Limit order.
    Limit,
    /// Stop-limit order.
    StopLimit,
    /// Stop-trailing order.
    StopTrailing,
    /// One-cancels-other.
    Oco,
}

/// Activation-condition value sent on `place_order` (request side).
///
/// Distinct from [`ActivationConditionType`] (response side), which
/// additionally has a `NONE` variant and is the `type` field nested in
/// the response [`ActivationCondition`] struct.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderActivationCondition {
    /// Trailing stop-loss. Activated when the price changes by the given
    /// percentage. Requires `target_value`, `trigger_value`,
    /// `trigger_condition`; `price` must be omitted.
    StopActpricePerc,
    /// Activated when the market price reaches a trigger price. Requires
    /// `trigger_value`, `trigger_condition`, `price`.
    StopActprice,
    /// Inactive in the Nordnet system until manually activated.
    Manual,
    /// One-cancels-other (one normal order + one stop-loss; either fill
    /// cancels the other).
    OcoStopActprice,
}

/// Request body for `place_order` (`POST /accounts/{accid}/orders`).
///
/// Wire format: `application/x-www-form-urlencoded` (Swagger 2.0
/// `FormData`). Sent via [`crate::Client::post_form`]. The struct is flat
/// (no nested objects, sequences, or maps) so `serde_urlencoded` accepts
/// it. **Field declaration order determines wire field order** — keep
/// alphabetical (matches the doc parameter table) to keep the wire body
/// deterministic.
///
/// Required fields: `market_id`, `side`, `volume`. Everything else is
/// optional. Optional fields use `skip_serializing_if = "Option::is_none"`
/// so `None` is omitted from the wire body.
///
/// `price`, `target_value`, `trigger_value` are documented as
/// `number(double)`. Modelled as `Decimal` (never `f64`), so this type
/// cannot derive [`Eq`]. The `Decimal` fields
/// intentionally do NOT carry the `arbitrary_precision_option` adapter:
/// it relies on a `serde_json`-private magic struct that
/// `serde_urlencoded` rejects with `unsupported value`. Default
/// `Decimal` serde uses `Display`, producing the decimal-string form
/// (`101.5`) — correct for both wire formats.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlaceOrderRequest {
    /// Activation condition for stop-loss orders. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation_condition: Option<OrderActivationCondition>,
    /// The currency that the instrument is traded in. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
    /// If `true`, order is applicable for US pre-market trading. Default
    /// `false`. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extended_hours: Option<bool>,
    /// Nordnet tradable identifier. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<TradableId>,
    /// Nordnet market identifier. Required.
    pub market_id: MarketId,
    /// The visible part of an iceberg order. Only allowed for `LIMIT` /
    /// `NORMAL` order types. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_volume: Option<i64>,
    /// The order type. Defaults server-side to `NORMAL` if omitted.
    /// Optional on the wire; clients are encouraged to pick an explicit
    /// type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_type: Option<OrderType>,
    /// The price limit of the order. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<Decimal>,
    /// Free-text reference for the order. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// `BUY` or `SELL`. Required.
    pub side: OrderSide,
    /// Only used when activation type is `STOP_ACTPRICE_PERC` or
    /// `OCO_STOP_ACTPRICE`. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_value: Option<Decimal>,
    /// The comparison used on `trigger_value`. Valid values are `<=`
    /// or `>=`. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_condition: Option<String>,
    /// Trigger value. For `STOP_ACTPRICE_PERC` it is in percentage points
    /// (minimum 1); for `STOP_ACTPRICE` it is a fixed price. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_value: Option<Decimal>,
    /// Cancel date formatted as `YYYY-MM-DD`. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<String>,
    /// The volume of the order. Required.
    pub volume: i64,
}

/// Request body for `modify_order`
/// (`PUT /accounts/{accid}/orders/{order_id}`).
///
/// All fields optional; the doc notes `currency` is required when `price`
/// is changed but enforces no compile-time invariant.
///
/// Wire format: `application/x-www-form-urlencoded` (Swagger 2.0
/// `FormData`). Sent via [`crate::Client::put_form`]. See the
/// [`PlaceOrderRequest`] note for why `Decimal` fields omit the
/// `arbitrary_precision_option` adapter.
///
/// `price` is `number(double)` per the docs, modelled as `Decimal`; this
/// type cannot derive [`Eq`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModifyOrderRequest {
    /// The currency of the instrument. Required when the price is
    /// changed. Optional otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
    /// The new open volume. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub open_volume: Option<i64>,
    /// The new price. If left out the price is left unchanged. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<Decimal>,
    /// The new volume. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<i64>,
}
