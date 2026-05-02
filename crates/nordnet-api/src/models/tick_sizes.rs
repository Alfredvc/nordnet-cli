//! Types for the `tick_sizes` resource group.
//!
//! Derived strictly from `docs-extract/_definitions/TicksizeTable.md` and
//! `docs-extract/_definitions/TicksizeInterval.md`.
//!
//! Doc note: `TicksizeInterval` documents `from_price`, `tick`, and
//! `to_price` as `number(double)`. Per CONTRACTS.md, `f64` is forbidden for
//! numeric price/tick fields; `rust_decimal::Decimal` is used instead,
//! which round-trips losslessly through JSON as a decimal number literal.

use crate::ids::TickSizeId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A single tick size interval entry.
///
/// Corresponds to the `TicksizeInterval` definition in the Nordnet API docs.
/// All price and tick fields use [`Decimal`] instead of `f64` per
/// CONTRACTS.md ("Never `f64`").
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TicksizeInterval {
    /// Number of decimals used in this interval.
    pub decimals: i64,
    /// The interval is valid from this price.
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub from_price: Decimal,
    /// The tick size.
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub tick: Decimal,
    /// The interval is valid to this price.
    #[serde(with = "rust_decimal::serde::arbitrary_precision")]
    pub to_price: Decimal,
}

/// A tick size table, consisting of one or more tick size intervals.
///
/// Corresponds to the `TicksizeTable` definition in the Nordnet API docs.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TicksizeTable {
    /// The unique tick size table ID.
    pub tick_size_id: TickSizeId,
    /// The tick size interval table.
    pub ticks: Vec<TicksizeInterval>,
}
