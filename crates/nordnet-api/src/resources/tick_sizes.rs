//! Resource methods for the `tick_sizes` API group.
//!
//! Endpoints:
//! - `GET /tick_sizes`         → [`Client::list_tick_sizes`]
//! - `GET /tick_sizes/{id}`    → [`Client::get_tick_size`]

use crate::client::Client;
use crate::error::Error;
use crate::ids::TickSizeId;
use crate::models::tick_sizes::TicksizeTable;

impl Client {
    /// `GET /tick_sizes` — Returns a list of all tick size tables.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unauthorized`] (401), [`Error::TooManyRequests`]
    /// (429), or [`Error::ServiceUnavailable`] (503) as documented.
    pub async fn list_tick_sizes(&self) -> Result<Vec<TicksizeTable>, Error> {
        self.get("/tick_sizes").await
    }

    /// `GET /tick_sizes/{tick_size_id}` — Returns one or more tick size
    /// tables identified by `id`.
    ///
    /// The Nordnet API path parameter technically accepts a comma-separated
    /// list of IDs. This method covers the single-ID case. A future helper
    /// may be added for multi-ID lookups when needed (Phase 4+).
    ///
    /// # Errors
    ///
    /// Returns [`Error::BadRequest`] (400), [`Error::Unauthorized`] (401),
    /// [`Error::TooManyRequests`] (429), or [`Error::ServiceUnavailable`]
    /// (503) as documented.
    pub async fn get_tick_size(&self, id: TickSizeId) -> Result<Vec<TicksizeTable>, Error> {
        self.get(&format!("/tick_sizes/{}", id)).await
    }
}
