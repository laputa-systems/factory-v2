//! Closed, sealed peer for the `society-pi-host/v4` JSONL boundary.
//!
//! This crate deliberately owns only transport validation and transient
//! execution facts.  It neither charges durable budgets nor authorizes Pi;
//! those decisions remain with the kernel and daemon integrations.

mod cost;
mod forum;
mod peer;
mod protocol;

pub use cost::{CostDecodeError, ProviderCost, UsageDelta, UsageTracker, UsdMicros};
pub use forum::{
    FORUM_F0_AWARENESS_BLAKE3, FORUM_F0_AWARENESS_BYTES, FORUM_F0_AWARENESS_REVISION,
    FORUM_F0_AWARENESS_TEXT, FORUM_F0_TOOL_CONTRACT_BLAKE3, FORUM_F0_TOOL_CONTRACT_BYTES,
    ForumToolContractDescriptor, ForumToolName,
};
pub use peer::{
    BoundaryPeer, PeerError, PeerObservation, PeerPhase, SealedLine, TurnDisposition, TurnReceipt,
};
pub use protocol::*;
