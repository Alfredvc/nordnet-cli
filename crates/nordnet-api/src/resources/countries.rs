//! Resource methods for the `countries` API group.
//!
//! # Operations
//!
//! | Method | Op | Path |
//! |--------|-----|------|
//! | GET | `list_countries` | `/countries` |
//! | GET | `get_country` | `/countries/{country}` |
//!
//! ## Multi-code lookups for `get_country`
//!
//! The Nordnet API accepts comma-separated country codes in the path segment:
//! `GET /countries/SE,NO`. Pass a comma-joined string to `get_country`:
//!
//! ```ignore
//! client.get_country("SE,NO").await?
//! ```
//!
//! ## 204 No Content
//!
//! `GET /countries/{country}` may return HTTP 204 (No Content) when no
//! matching country is found. The base `Client::get` method attempts to parse
//! the empty response body as JSON, which fails. `get_country` detects this
//! specific case (successful decode failure with an empty body) and maps it to
//! an empty `Vec`.

use crate::client::Client;
use crate::error::Error;
use crate::models::countries::Country;

impl Client {
    /// Return all countries known to the Nordnet system.
    ///
    /// Not every returned country supports trading.
    ///
    /// Calls `GET /countries`.
    pub async fn list_countries(&self) -> Result<Vec<Country>, Error> {
        self.get("/countries").await
    }

    /// Look up one or more countries by ISO country code.
    ///
    /// Multiple codes can be queried in a single call by passing a
    /// comma-separated string (e.g. `"SE,NO"`). Returns an empty `Vec` when
    /// the API responds with 204 No Content (no matching countries).
    ///
    /// Calls `GET /countries/{country}`.
    pub async fn get_country(&self, code: &str) -> Result<Vec<Country>, Error> {
        let path = format!("/countries/{code}");
        match self.get::<Vec<Country>>(&path).await {
            Ok(countries) => Ok(countries),
            // HTTP 204 No Content: the base client sees an empty body and
            // produces a Decode error. Map it to an empty Vec instead.
            Err(Error::Decode { ref body, .. }) if body.trim().is_empty() => Ok(vec![]),
            Err(e) => Err(e),
        }
    }
}
