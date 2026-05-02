//! Models for the `countries` resource group.
//!
//! Derived from `docs-extract/_definitions/Country.md`.
//! The `Country` definition has two required string fields: `country` (ISO
//! country code) and `name` (translated country name). Neither is marked
//! optional in the schema table, so no `Option<T>` wrappers are used.

use serde::{Deserialize, Serialize};

/// A country entry as returned by `GET /countries` and
/// `GET /countries/{country}`.
///
/// Schema source: `docs-extract/_definitions/Country.md`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Country {
    /// ISO country code (e.g. `"SE"`, `"NO"`).
    pub country: String,
    /// Translated name of the country (language controlled by the
    /// `Accept-Language` request header).
    pub name: String,
}
