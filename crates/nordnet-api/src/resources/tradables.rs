//! Resource methods for the `tradables` API group.
//!
//! # Operations
//!
//! | Method | Op | Path |
//! |--------|----|------|
//! | GET | `get_tradable_info` | `/tradables/info/{tradables}` |
//! | GET | `list_tradable_trades` | `/tradables/trades/{tradables}` |
//! | GET | `get_suitability` | `/tradables/validation/suitability/{tradables}` |
//!
//! ## Path encoding — [`TradableKey`]
//!
//! Each operation takes a single [`TradableKey`] (e.g. `11:101` for
//! ERIC B). The Nordnet API also accepts a comma-separated list of keys at
//! the path slot, but the typed surface stays single-key for now — Phase 4
//! is expected to add a small helper for the multi-key shape.
//!
//! ## Naming — `list_tradable_trades`
//!
//! The Nordnet documentation calls this op `list_trades`. Renamed to
//! `list_tradable_trades` so it can co-exist on [`Client`] alongside the
//! same-named `list_trades` ops planned for the `accounts` and
//! `instruments` groups (Rust resolves all three onto a single `Client`
//! impl). Phase 3X may pick a uniform naming scheme.
//!
//! ## 204 No Content
//!
//! Every op may return HTTP 204 (No Content). The base [`Client::get`]
//! treats an empty body as a [`Error::Decode`]; each method here maps that
//! specific case to an empty `Vec`, mirroring the
//! [`Client::get_country`] precedent.
//!
//! ## 403 No Content (`get_suitability`)
//!
//! `GET /tradables/validation/suitability/{tradables}` returns HTTP 403 with
//! an empty body for anonymous sessions. The base client maps any 403 to
//! [`Error::Forbidden`] (with the empty body string preserved), so callers
//! can distinguish this from a parse error.

use crate::client::Client;
use crate::error::Error;
use nordnet_model::models::tradables::{
    TradableEligibility, TradableInfo, TradableKey, TradablePublicTrades,
};

impl Client {
    /// `GET /tradables/info/{tradables}` — Returns trading calendar and
    /// allowed trading types for the given tradable.
    ///
    /// Returns an empty `Vec` when the API responds with 204 No Content
    /// (no matching tradables).
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), or [`Error::ServiceUnavailable`]
    /// (503) as documented.
    pub async fn get_tradable_info(&self, key: &TradableKey) -> Result<Vec<TradableInfo>, Error> {
        let path = format!("/tradables/info/{key}");
        match self.get::<Vec<TradableInfo>>(&path).await {
            Ok(v) => Ok(v),
            // 204 No Content — base client surfaces this as a Decode error
            // over an empty body. Map it to an empty Vec.
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /tradables/trades/{tradables}` — Returns the public trades
    /// (all trades executed on the marketplace) for the given tradable.
    ///
    /// # Parameters
    ///
    /// - `key` — the tradable to look up.
    /// - `count` — optional. Number of trades to return. The API accepts
    ///   either a positive integer (`"5"`, `"10"`, ...) or the literal
    ///   string `"all"`; the default is `"5"`. Passed through verbatim.
    ///
    /// Returns an empty `Vec` when the API responds with 204 No Content.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), or [`Error::ServiceUnavailable`]
    /// (503) as documented.
    ///
    /// # Naming
    ///
    /// The Nordnet docs call this op `list_trades`; renamed here to
    /// `list_tradable_trades` so it can co-exist with the same-named ops
    /// planned for the `accounts` and `instruments` groups (see module
    /// doc).
    pub async fn list_tradable_trades(
        &self,
        key: &TradableKey,
        count: Option<&str>,
    ) -> Result<Vec<TradablePublicTrades>, Error> {
        let path = match count {
            Some(c) => format!("/tradables/trades/{key}?count={c}"),
            None => format!("/tradables/trades/{key}"),
        };
        match self.get::<Vec<TradablePublicTrades>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    /// `GET /tradables/validation/suitability/{tradables}` — Returns the
    /// customer's trading eligibility for the given tradable.
    ///
    /// Returns an empty `Vec` when the API responds with 204 No Content.
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::Forbidden`] (403; documented for anonymous sessions and
    /// returned with an empty body), [`Error::TooManyRequests`] (429), or
    /// [`Error::ServiceUnavailable`] (503) as documented.
    pub async fn get_suitability(
        &self,
        key: &TradableKey,
    ) -> Result<Vec<TradableEligibility>, Error> {
        let path = format!("/tradables/validation/suitability/{key}");
        match self.get::<Vec<TradableEligibility>>(&path).await {
            Ok(v) => Ok(v),
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }
}
