#![doc = include_str!("../README.md")]

pub mod client;
pub mod error;
pub mod pagination;
pub mod resources;

pub use client::Client;
pub use error::Error;
