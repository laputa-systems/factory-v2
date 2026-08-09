//! Application-owned construction of XSH inputs for the generic Society port.
//!
//! These factories create closed values only. They do not admit a mission,
//! assign a durable application revision, seal the rendering, or hold a daemon
//! control channel. A trusted composition root may submit the returned mission
//! through the generic supervisor boundary and later supply the kernel-issued
//! revision identity when constructing a Project alignment.

use society_kernel::{
    ApplicationIdentity, ApplicationMissionInput, ApplicationName, ApplicationRevisionId,
    ApplicationRevisionOrdinal, Blake3Digest, MissionPrinciple, MissionPrincipleKind,
    MissionPrincipleText, MissionPrinciples, MissionStatement, NorthStarBoundaryCommitmentQuestion,
    NorthStarChangeQuestion, NorthStarImprovementEvidenceQuestion, NorthStarQuestionSet,
    NorthStarRevisitQuestion, ProjectNorthStarAlignment, ProjectNorthStarBoundaryCommitmentAnswer,
    ProjectNorthStarChangeAnswer, ProjectNorthStarImprovementEvidenceAnswer,
    ProjectNorthStarRevisitAnswer,
};

const UNIVERSE_SEED_V1: &[u8] = include_bytes!("../UNIVERSE-SEED.v1.md");
const MISSION: &str = "Make XSH a practical, coherent, easy-to-learn, token-efficient, and trustworthy systems-glue language for humans and coding agents, capable of replacing fragile Unix glue with typed paths, explicit process and effect boundaries, structured streams and errors, reproducible execution, and inspectable policy while preserving the composability that makes Unix systems useful.";
const PURPOSE_PRINCIPLE: &str = "Make ordinary process composition typed and discoverable.";
const EVIDENCE_PRINCIPLE: &str =
    "Prefer executable, reproducible evidence over narrative confidence.";
const PRESERVATION_PRINCIPLE: &str = "Preserve explicit command fields, owned child lifecycles, structured setup errors, default stream inheritance, and ordinary path sinks.";
const REJECTION_PRINCIPLE: &str = "Reject quoting puzzles, ambient state, implicit evaluation, text-only boundaries, and stacked private DSLs.";
const REVISION_PRINCIPLE: &str = "Revise the direction when bounded reviews or real use-site evidence contradict the current contract.";
const CHANGE_QUESTION: &str = "What XSH capability or actor behavior would change?";
const IMPROVEMENT_EVIDENCE_QUESTION: &str = "What evidence distinguishes a general improvement from a local workaround, movement of complexity, or noise?";
const BOUNDARY_COMMITMENT_QUESTION: &str = "How does the change honor clarity, explicit boundaries, composability, and XSH's systems-glue scope?";
const REVISIT_QUESTION: &str = "At which review, replay, outcome horizon, or Grand Architect decision will the claim be revisited?";
const ALIGNMENT_CHANGE: &str = "XSH users and agents can express ordinary child stderr policy through typed Command composition without a shell boundary.";
const ALIGNMENT_IMPROVEMENT_EVIDENCE: &str = "Normative registry, executable behavior, proposal corpus, public discovery, focused tests, and real call sites converge at one pinned XSH revision; a paired task provides only qualitative agent-fluency evidence.";
const ALIGNMENT_BOUNDARY_COMMITMENT: &str = "Preserve explicit Command fields, owned process lifecycle, structured setup errors, default inheritance, and ordinary Path sinks.";
const ALIGNMENT_REVISIT: &str = "Revisit after C1 prototype review, C2 delivery review, the immediate contract outcome, the delayed Laputa use-site obligation, or any contradictory process test.";

/// Exact application-owned bytes committed by [`founding_mission_v1`].
pub const fn universe_seed_v1_rendering() -> &'static [u8] {
    UNIVERSE_SEED_V1
}

/// XSH revision 1 expressed through the generic mission input port.
pub fn founding_mission_v1() -> ApplicationMissionInput {
    ApplicationMissionInput {
        application_identity: ApplicationIdentity::parse("xsh").expect("static identity is valid"),
        application_name: ApplicationName::parse("XSH").expect("static name is valid"),
        revision_ordinal: ApplicationRevisionOrdinal::new(1)
            .expect("static revision ordinal is positive"),
        statement: MissionStatement::parse(MISSION).expect("static mission statement is valid"),
        principles: MissionPrinciples::new(vec![
            principle(MissionPrincipleKind::Purpose, PURPOSE_PRINCIPLE),
            principle(MissionPrincipleKind::Evidence, EVIDENCE_PRINCIPLE),
            principle(MissionPrincipleKind::Boundary, PRESERVATION_PRINCIPLE),
            principle(MissionPrincipleKind::Boundary, REJECTION_PRINCIPLE),
            principle(MissionPrincipleKind::Revision, REVISION_PRINCIPLE),
        ])
        .expect("static mission principle set is valid"),
        north_star_questions: NorthStarQuestionSet {
            change: NorthStarChangeQuestion::parse(CHANGE_QUESTION)
                .expect("static question is valid"),
            improvement_evidence: NorthStarImprovementEvidenceQuestion::parse(
                IMPROVEMENT_EVIDENCE_QUESTION,
            )
            .expect("static question is valid"),
            boundary_commitment: NorthStarBoundaryCommitmentQuestion::parse(
                BOUNDARY_COMMITMENT_QUESTION,
            )
            .expect("static question is valid"),
            revisit: NorthStarRevisitQuestion::parse(REVISIT_QUESTION)
                .expect("static question is valid"),
        },
        source_rendering_digest: Blake3Digest::of_bytes(universe_seed_v1_rendering()),
    }
}

/// VS-001's application-owned answers, bound to the kernel-issued revision.
pub fn vs001_spawn_stderr_alignment(
    application_revision_id: ApplicationRevisionId,
) -> ProjectNorthStarAlignment {
    ProjectNorthStarAlignment {
        application_revision_id,
        change_answer: ProjectNorthStarChangeAnswer::parse(ALIGNMENT_CHANGE)
            .expect("static alignment answer is valid"),
        improvement_evidence_answer: ProjectNorthStarImprovementEvidenceAnswer::parse(
            ALIGNMENT_IMPROVEMENT_EVIDENCE,
        )
        .expect("static alignment answer is valid"),
        boundary_commitment_answer: ProjectNorthStarBoundaryCommitmentAnswer::parse(
            ALIGNMENT_BOUNDARY_COMMITMENT,
        )
        .expect("static alignment answer is valid"),
        revisit_answer: ProjectNorthStarRevisitAnswer::parse(ALIGNMENT_REVISIT)
            .expect("static alignment answer is valid"),
    }
}

fn principle(kind: MissionPrincipleKind, text: &'static str) -> MissionPrinciple {
    MissionPrinciple {
        kind,
        text: MissionPrincipleText::parse(text).expect("static principle is valid"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn founding_mission_binds_the_exact_application_rendering() {
        let mission = founding_mission_v1();

        assert_eq!(mission.application_identity.as_str(), "xsh");
        assert_eq!(mission.application_name.as_str(), "XSH");
        assert_eq!(mission.revision_ordinal.value(), 1);
        assert_eq!(mission.statement.as_str(), MISSION);
        assert_eq!(
            mission
                .principles
                .as_slice()
                .iter()
                .map(|principle| (principle.kind, principle.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (MissionPrincipleKind::Purpose, PURPOSE_PRINCIPLE),
                (MissionPrincipleKind::Evidence, EVIDENCE_PRINCIPLE),
                (MissionPrincipleKind::Boundary, PRESERVATION_PRINCIPLE),
                (MissionPrincipleKind::Boundary, REJECTION_PRINCIPLE),
                (MissionPrincipleKind::Revision, REVISION_PRINCIPLE),
            ]
        );
        assert_eq!(
            mission.north_star_questions.change.as_str(),
            CHANGE_QUESTION
        );
        assert_eq!(
            mission.north_star_questions.improvement_evidence.as_str(),
            IMPROVEMENT_EVIDENCE_QUESTION
        );
        assert_eq!(
            mission.north_star_questions.boundary_commitment.as_str(),
            BOUNDARY_COMMITMENT_QUESTION
        );
        assert_eq!(
            mission.north_star_questions.revisit.as_str(),
            REVISIT_QUESTION
        );
        assert_eq!(
            mission.source_rendering_digest,
            Blake3Digest::of_bytes(universe_seed_v1_rendering())
        );
        assert_eq!(
            universe_seed_v1_rendering(),
            expected_rendering().as_bytes()
        );
    }

    #[test]
    fn vs001_alignment_preserves_the_supplied_kernel_revision_and_exact_answers() {
        let revision = ApplicationRevisionId::new(73).expect("positive test identity");
        let alignment = vs001_spawn_stderr_alignment(revision);

        assert_eq!(alignment.application_revision_id, revision);
        assert_eq!(alignment.change_answer.as_str(), ALIGNMENT_CHANGE);
        assert_eq!(
            alignment.improvement_evidence_answer.as_str(),
            ALIGNMENT_IMPROVEMENT_EVIDENCE
        );
        assert_eq!(
            alignment.boundary_commitment_answer.as_str(),
            ALIGNMENT_BOUNDARY_COMMITMENT
        );
        assert_eq!(alignment.revisit_answer.as_str(), ALIGNMENT_REVISIT);
    }

    fn expected_rendering() -> String {
        format!(
            "# XSH Universe Seed, revision 1\n\n{MISSION}\n\n## Domain scope and non-goals\n\nXSH is a clean-slate systems scripting language for modern Linux userspace: strong glue between processes, files, paths, byte streams, structured data, and system state.\n\nXSH is not a POSIX compatibility shell, an interactive terminal, or a claim to be the best general application runtime.\n\n## Preserved Unix properties\n\n- Coarse-grained composability.\n- Ordinary files and visible process boundaries.\n- Pipeline flow.\n- The ability for a script to grow into a tool.\n\n## Principles\n\n- {PURPOSE_PRINCIPLE}\n- {EVIDENCE_PRINCIPLE}\n- {PRESERVATION_PRINCIPLE}\n- {REJECTION_PRINCIPLE}\n- {REVISION_PRINCIPLE}\n\n## North-star questions\n\n1. {CHANGE_QUESTION}\n2. {IMPROVEMENT_EVIDENCE_QUESTION}\n3. {BOUNDARY_COMMITMENT_QUESTION}\n4. {REVISIT_QUESTION}\n\n## Active Grand Architect Office contract\n\n`TheGrandArchitect` is the highest constitutional Office and the final decision authority inside the running XSH society. Its occupant may be a user or an assigned coding agent; durable authority comes only from authenticated occupancy and exact capability grants.\n\nIts reserved powers are to ratify or amend the active seed; govern Projects and resource envelopes inside hard ceilings; govern subordinate Offices and organization configurations; decide or accept risk for consequential changes; require review, postmortem, replay, or outcome observation; resolve documented conflicts and exceptions; reopen preserved work; and designate a successor.\n\nThe Office cannot write raw SQL, mutate the content store directly, access secrets outside an execution profile, forge evidence, alter prior events, create unreserved spend, force an invalid state transition, or deploy a replacement kernel through an ordinary command.\n"
        )
    }
}
