use society_pi::{
    FORUM_F0_AWARENESS_BLAKE3, FORUM_F0_AWARENESS_BYTES, FORUM_F0_AWARENESS_TEXT,
    FORUM_F0_TOOL_CONTRACT_BLAKE3, FORUM_F0_TOOL_CONTRACT_BYTES, ForumToolContractDescriptor,
    ForumToolName,
};

#[test]
fn public_f0_awareness_is_exactly_the_registered_utf8_and_digest() {
    assert_eq!(FORUM_F0_AWARENESS_BYTES, FORUM_F0_AWARENESS_TEXT.as_bytes());
    assert_eq!(
        blake3::hash(FORUM_F0_AWARENESS_BYTES).to_hex().as_str(),
        FORUM_F0_AWARENESS_BLAKE3
    );
    assert_eq!(
        blake3::hash(FORUM_F0_TOOL_CONTRACT_BYTES).to_hex().as_str(),
        FORUM_F0_TOOL_CONTRACT_BLAKE3
    );
}

#[test]
fn public_contract_is_closed_and_sequestered_has_no_forum_surface() {
    let enabled = ForumToolContractDescriptor::ForumEnabledV1;
    assert_eq!(
        enabled.tool_names(),
        &[
            ForumToolName::SocietyForumRead,
            ForumToolName::SocietyForumPost
        ]
    );
    assert_eq!(enabled.awareness_bytes(), Some(FORUM_F0_AWARENESS_BYTES));
    assert_eq!(
        enabled.awareness_blake3_hex(),
        Some(FORUM_F0_AWARENESS_BLAKE3)
    );
    assert_eq!(
        enabled.tool_contract_blake3_hex(),
        Some(FORUM_F0_TOOL_CONTRACT_BLAKE3)
    );

    let sequestered = ForumToolContractDescriptor::SequesteredV1;
    assert!(sequestered.tool_names().is_empty());
    assert!(sequestered.awareness_bytes().is_none());
    assert!(sequestered.awareness_blake3_hex().is_none());
    assert!(sequestered.tool_contract_blake3_hex().is_none());
}
