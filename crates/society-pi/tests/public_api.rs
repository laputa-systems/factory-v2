//! A downstream daemon must be able to name every normalized peer fact.

use society_pi::{
    BoundaryPeer, InboundFrame, OutboundFrame, PeerError, PeerObservation, TurnReceipt, UsageDelta,
};

fn consume(observation: PeerObservation) {
    match observation {
        PeerObservation::Usage(UsageDelta { .. }) => {}
        PeerObservation::UsageUnavailable { reason: _ } => {}
        PeerObservation::TurnSettled(TurnReceipt { .. }) => {}
        PeerObservation::Disposed => {}
        PeerObservation::Fatal { failure_code: _ } => {}
    }
}

#[test]
fn public_observation_union_is_matchable_by_a_downstream_consumer() {
    let _: fn(PeerObservation) = consume;
    let _: for<'a> fn(&'a mut BoundaryPeer, &'a [u8]) -> Result<InboundFrame, PeerError> =
        BoundaryPeer::admit_inbound_jsonl_bytes;
    let _: for<'a> fn(
        &'a mut BoundaryPeer,
        &'a [u8],
    ) -> Result<Option<PeerObservation>, PeerError> = BoundaryPeer::observe_outbound_jsonl_bytes;
    let _: fn(&mut BoundaryPeer, OutboundFrame) -> Result<Option<PeerObservation>, PeerError> =
        BoundaryPeer::observe_outbound;
}
