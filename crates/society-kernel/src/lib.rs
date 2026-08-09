//! Trusted, replayable domain and SQLite storage for the XSH Society.
//!
//! This crate intentionally has no JSON serialization boundary.  Commands are
//! closed Rust values, and each accepted command/event has one named SQLite
//! body table.  JSON is reserved for the separately-owned Pi SDK-host adapter.

mod domain;
mod store;

pub use domain::*;
pub use store::{KernelStore, StoreError};
