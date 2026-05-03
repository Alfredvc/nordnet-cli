//! Models for the `root` resource group.
//!
//! Covers: `GET /api/2` (system status). Derived from the Nordnet `Status`
//! schema.

use serde::{Deserialize, Serialize};

/// System status information returned by `GET /api/2`.
///
/// All four fields are required per the schema.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Status {
    /// Additional information from the server (e.g. maintenance notices).
    pub message: String,

    /// Indicates whether the system is running or temporarily stopped.
    pub system_running: bool,

    /// Server time expressed as a UNIX timestamp in **milliseconds**.
    ///
    /// The schema specifies `integer(int64)` — milliseconds since the Unix
    /// epoch, not an ISO 8601 string. Stored as `i64` rather than
    /// `time::OffsetDateTime` because the wire format is a raw integer, not
    /// an ISO 8601 string (the `Timestamp` alias in `shared.rs` is for ISO
    /// 8601 fields). Callers that need wall-clock time should convert:
    /// `OffsetDateTime::from_unix_timestamp(timestamp / 1000)`.
    pub timestamp: i64,

    /// `true` if the API version targeted by this client is valid.
    pub valid_version: bool,
}
