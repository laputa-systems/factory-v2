//! Closed, sealed peer for the `society-pi-host/v2` JSONL boundary.
//!
//! This crate deliberately owns only transport validation and transient
//! execution facts.  It neither charges durable budgets nor authorizes Pi;
//! those decisions remain with the kernel and daemon integrations.

mod cost;
mod peer;
mod protocol;

pub use cost::{CostDecodeError, ProviderCost, UsageDelta, UsageTracker, UsdMicros};
pub use peer::{
    BoundaryPeer, PeerError, PeerObservation, PeerPhase, SealedLine, TurnDisposition, TurnReceipt,
};
pub use protocol::*;
