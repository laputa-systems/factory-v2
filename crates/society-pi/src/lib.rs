//! Closed, sealed peer for the `society-pi-host/v4` JSONL boundary.
//!
//! This crate deliberately owns only transport validation and transient
//! execution facts.  It neither charges durable budgets nor authorizes Pi;
//! those decisions remain with the kernel and daemon integrations.

mod cost;
mod forum;
mod peer;
mod protocol;

pub use miniserde::json::Value as SdkJsonValue;

/// Small constructors kept beside the SDK value alias so downstream daemon
/// code does not need to depend directly on the JSON implementation crate.
pub fn sdk_json_object(fields: impl IntoIterator<Item = (String, SdkJsonValue)>) -> SdkJsonValue {
    let mut object = miniserde::json::Object::new();
    for (key, value) in fields {
        object.insert(key, value);
    }
    SdkJsonValue::Object(object)
}

pub fn sdk_json_u64(value: u64) -> SdkJsonValue {
    SdkJsonValue::Number(miniserde::json::Number::U64(value))
}

/// Compares JSON values using the canonical boundary rendering. Pi's JSON
/// value type intentionally does not implement `Eq`; callers outside this
/// crate still need a deterministic equality check for validated evidence.
pub fn sdk_json_values_equal(left: &SdkJsonValue, right: &SdkJsonValue) -> bool {
    miniserde::json::to_string(left) == miniserde::json::to_string(right)
}

pub use cost::{CostDecodeError, ProviderCost, UsageDelta, UsageTracker, UsdMicros};
pub use forum::{
    FORUM_F0_AWARENESS_BLAKE3, FORUM_F0_AWARENESS_BYTES, FORUM_F0_AWARENESS_REVISION,
    FORUM_F0_AWARENESS_TEXT, FORUM_F0_TOOL_CONTRACT_BLAKE3, FORUM_F0_TOOL_CONTRACT_BYTES,
    ForumMessageKind, ForumSessionContractV1, ForumToolArguments, ForumToolArgumentsError,
    ForumToolContractDescriptor, ForumToolName, decode_forum_tool_arguments,
};
pub use peer::{
    BoundaryPeer, PeerError, PeerObservation, PeerPhase, SealedLine, TurnDisposition, TurnReceipt,
};
pub use protocol::*;
