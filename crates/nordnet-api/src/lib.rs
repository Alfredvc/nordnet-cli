//! Typed REST bindings for the Nordnet External API v2.
//!
//! HTTP-shaped surface only. Wire-typed inputs / outputs (request and
//! response structs, ID newtypes, the Ed25519 login primitives) live in
//! the sibling [`nordnet_model`] crate; this crate composes them with
//! `reqwest`-backed HTTP plumbing.
//!
//! See `CONTRACTS.md` at the workspace root for the full contract.

pub mod client;
pub mod error;
pub mod pagination;
pub mod resources;

pub use client::Client;
pub use error::Error;
