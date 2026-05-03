//! Resource methods for the `root` group.
//!
//! Covers: `GET /api/2` — system status.

use crate::client::Client;
use crate::error::Error;
use nordnet_model::models::root::Status;

impl Client {
    /// `GET /api/2` — Returns information about the system status.
    ///
    /// The path is the API root itself (relative to the configured base URL).
    /// In production the base URL is `https://public.nordnet.se/api/2`, so
    /// this method issues `GET https://public.nordnet.se/api/2/`.
    pub async fn get_system_status(&self) -> Result<Status, Error> {
        self.get("").await
    }
}
