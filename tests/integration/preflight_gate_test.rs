//! Red-phase integration harness for the F50 production preflight state machine.

use std::time::Instant;

#[path = "../../crates/engram-indexer/src/preflight.rs"]
mod preflight;

use preflight::{Build, Failure, Preflight, StageKind, Succeeded, VerificationError, Verifier};

#[derive(Default)]
struct RecordingVerifier {
    fail_at: Option<StageKind>,
    calls: Vec<(StageKind, Instant, String)>,
}

impl Verifier for RecordingVerifier {
    fn verify(
        &mut self,
        stage: StageKind,
        deadline: Instant,
        expected_generation: &str,
    ) -> Result<(), VerificationError> {
        self.calls
            .push((stage, deadline, expected_generation.to_owned()));
        if self.fail_at == Some(stage) {
            Err(VerificationError)
        } else {
            Ok(())
        }
    }
}

const STAGES: [StageKind; 7] = [
    StageKind::Build,
    StageKind::Seal,
    StageKind::Publish,
    StageKind::DaemonVerified,
    StageKind::HealthVerified,
    StageKind::CliProbeVerified,
    StageKind::McpProbeVerified,
];

fn failure_for(stage: StageKind) -> Failure {
    match stage {
        StageKind::Build => Failure::Build,
        StageKind::Seal => Failure::Seal,
        StageKind::Publish => Failure::Publish,
        StageKind::DaemonVerified => Failure::DaemonVerified,
        StageKind::HealthVerified => Failure::HealthVerified,
        StageKind::CliProbeVerified => Failure::CliProbeVerified,
        StageKind::McpProbeVerified => Failure::McpProbeVerified,
    }
}

fn verify_all_stages(
    deadline: Instant,
    expected_generation: &str,
    verifier: &mut RecordingVerifier,
) -> Result<Preflight<Succeeded>, Failure> {
    Preflight::<Build>::start(deadline, expected_generation.to_owned())
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)
}

#[test]
fn succeeded_requires_all_stages_with_one_deadline_and_generation() {
    let deadline = Instant::now() + std::time::Duration::from_secs(60);
    let expected_generation = "generation-142".to_owned();
    let mut verifier = RecordingVerifier::default();

    let result = verify_all_stages(deadline, &expected_generation, &mut verifier);

    assert!(result.is_ok(), "all verified stages should succeed");
    assert_eq!(
        verifier.calls.iter().map(|call| call.0).collect::<Vec<_>>(),
        STAGES
    );
    assert!(
        verifier
            .calls
            .iter()
            .all(|call| call.1 == deadline && call.2 == expected_generation)
    );

    let Some(succeeded) = result.ok() else {
        panic!("verified stages should produce Succeeded");
    };
    assert_eq!(succeeded.deadline(), deadline);
    assert_eq!(succeeded.expected_generation(), expected_generation);
}

#[test]
fn every_stage_failure_returns_its_typed_failed_variant() {
    let deadline = Instant::now() + std::time::Duration::from_secs(60);

    for stage in STAGES {
        let mut verifier = RecordingVerifier {
            fail_at: Some(stage),
            calls: Vec::new(),
        };
        let result = verify_all_stages(deadline, "generation-142", &mut verifier);

        assert_eq!(result.err(), Some(failure_for(stage)));
        assert_eq!(verifier.calls.last().map(|call| call.0), Some(stage));
    }
}
