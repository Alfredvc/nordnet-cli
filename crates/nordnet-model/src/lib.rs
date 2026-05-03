//! Pure data types and crypto for the Nordnet External API v2.
//!
//! This crate has zero I/O dependencies (no `reqwest`, no `tokio`,
//! no `tokio-rustls`). It hosts:
//!
//! - [`auth`] — Ed25519 SSH-key login flow primitives plus the [`auth::Session`]
//!   newtype carried by [`crate::error::AuthError`].
//! - [`models`] — serde structs for every documented request and response
//!   shape, organised per resource group.
//! - [`ids`] — newtype wrappers for resource identifiers.
//! - [`error::AuthError`] — error type covering only what [`auth`] can fail at.
//!
//! Both `nordnet-api` (REST client) and `nordnet-feed` (streaming client)
//! depend on this crate for shared wire-typed inputs and outputs.

pub mod auth;
pub mod error;
pub mod ids;
pub mod models;

pub use error::AuthError;
