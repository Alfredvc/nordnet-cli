//! Typed REST bindings for the Nordnet External API v2.
//!
//! Foundation modules (this file + everything declared below) are locked
//! after Phase 0. Per-resource-group modules live under `models/<group>`
//! and `resources/<group>` and are added by Phase 3 implementers.
//!
//! See `CONTRACTS.md` at the workspace root for the full contract.

pub mod auth;
pub mod client;
pub mod error;
pub mod ids;
pub mod models;
pub mod pagination;
pub mod resources;

pub use client::Client;
pub use error::Error;
