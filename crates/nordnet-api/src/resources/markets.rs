//! Resource methods for the `markets` API group.
//!
//! # Operations
//!
//! | Method | Op | Path |
//! |--------|-----|------|
//! | GET | `list_markets` | `/markets` |
//! | GET | `get_market` | `/markets/{market_id}` |
//!
//! ## Multi-ID lookups for `get_market`
//!
//! The Nordnet API path parameter for `get_market` accepts a comma-separated
//! list of market IDs. This method covers the single-ID case via the
//! [`MarketId`] newtype, mirroring the [`Client::get_country`] pattern. A
//! future helper may be added for multi-ID lookups when needed.
//!
//! ## 204 No Content
//!
//! `GET /markets/{market_id}` may return HTTP 204 (No Content) when no
//! matching market is found. The base [`Client::get`] method attempts to
//! parse the empty response body as JSON, which fails. `get_market` detects
//! this specific case (a [`Error::Decode`] with an empty body) and maps it
//! to an empty `Vec`, mirroring [`Client::get_country`].

use crate::client::Client;
use crate::error::Error;
use nordnet_model::ids::MarketId;
use nordnet_model::models::markets::Market;

impl Client {
    /// `GET /markets` — Returns information about all tradable markets.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unauthorized`] (401), [`Error::TooManyRequests`]
    /// (429), or [`Error::ServiceUnavailable`] (503) as documented.
    pub async fn list_markets(&self) -> Result<Vec<Market>, Error> {
        self.get("/markets").await
    }

    /// `GET /markets/{market_id}` — Returns information about the market
    /// identified by `id`.
    ///
    /// Returns an empty `Vec` when the API responds with 204 No Content (no
    /// matching markets).
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), or [`Error::ServiceUnavailable`]
    /// (503) as documented.
    pub async fn get_market(&self, id: MarketId) -> Result<Vec<Market>, Error> {
        let path = format!("/markets/{id}");
        match self.get::<Vec<Market>>(&path).await {
            Ok(markets) => Ok(markets),
            // HTTP 204 No Content: the base client sees an empty body and
            // produces a Decode error. Map it to an empty Vec instead.
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }
}
