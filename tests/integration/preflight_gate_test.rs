//! Mock-driven harness for plan unit F50a (typed preflight gate).
//!
//! Covers success, fail-closed failure at each of the seven stages (one
//! table), and the single shared deadline / expected generation.

use std::time::{Duration, Instant};

use engram_indexer::preflight::{
    Build, Failure, Preflight, StageKind, Succeeded, VerificationError, Verifier,
};

const GENERATION: &str = "gen-0042";

const ALL_STAGES: [(StageKind, Failure); 7] = [
    (StageKind::Build, Failure::Build),
    (StageKind::Seal, Failure::Seal),
    (StageKind::Publish, Failure::Publish),
    (StageKind::DaemonVerified, Failure::DaemonVerified),
    (StageKind::HealthVerified, Failure::HealthVerified),
    (StageKind::CliProbeVerified, Failure::CliProbeVerified),
    (StageKind::McpProbeVerified, Failure::McpProbeVerified),
];

/// Records every call and optionally fails at one stage.
#[derive(Default)]
struct MockVerifier {
    fail_at: Option<StageKind>,
    calls: Vec<(StageKind, Instant, String)>,
}

impl Verifier for MockVerifier {
    fn verify(
        &mut self,
        stage: StageKind,
        deadline: Instant,
        expected_generation: &str,
    ) -> Result<(), VerificationError> {
        self.calls
            .push((stage, deadline, expected_generation.to_owned()));
        if self.fail_at == Some(stage) {
            Err(VerificationError::new("injected failure"))
        } else {
            Ok(())
        }
    }
}

fn far_deadline() -> Instant {
    Instant::now() + Duration::from_secs(3600)
}

/// Runs all seven stages in order, returning the typed result.
fn run(deadline: Instant, verifier: &mut MockVerifier) -> Result<Preflight<Succeeded>, Failure> {
    Preflight::<Build>::start(deadline, GENERATION.to_owned())
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)?
        .verify(verifier)
}

#[test]
fn success_reaches_succeeded_through_all_seven_stages() {
    let deadline = far_deadline();
    let mut mock = MockVerifier::default();

    let done = run(deadline, &mut mock).expect("all stages verify");

    let stages: Vec<StageKind> = mock.calls.iter().map(|call| call.0).collect();
    let expected: Vec<StageKind> = ALL_STAGES.iter().map(|entry| entry.0).collect();
    assert_eq!(stages, expected, "stages verified in declared order");
    assert_eq!(done.deadline(), deadline);
    assert_eq!(done.expected_generation(), GENERATION);
}

#[test]
fn failure_at_each_stage_is_typed_and_fail_closed() {
    for (index, (stage, failure)) in ALL_STAGES.iter().enumerate() {
        let mut mock = MockVerifier {
            fail_at: Some(*stage),
            ..MockVerifier::default()
        };

        let result = run(far_deadline(), &mut mock);

        assert_eq!(
            result.err(),
            Some(*failure),
            "failure at {stage:?} must name that stage and never succeed"
        );
        let called: Vec<StageKind> = mock.calls.iter().map(|call| call.0).collect();
        let expected: Vec<StageKind> = ALL_STAGES[..=index].iter().map(|entry| entry.0).collect();
        assert_eq!(called, expected, "no stage after {stage:?} may be called");
    }
}

#[test]
fn one_deadline_and_one_generation_span_every_stage() {
    let deadline = far_deadline();
    let mut mock = MockVerifier::default();

    run(deadline, &mut mock).expect("all stages verify");

    assert_eq!(mock.calls.len(), 7);
    for (stage, seen_deadline, seen_generation) in &mock.calls {
        assert_eq!(*seen_deadline, deadline, "deadline at {stage:?}");
        assert_eq!(seen_generation, GENERATION, "generation at {stage:?}");
    }
}

#[test]
fn expired_deadline_fails_first_stage_without_calling_verifier() {
    let mut mock = MockVerifier::default();
    let expired = Instant::now();

    let result = run(expired, &mut mock);

    assert_eq!(result.err(), Some(Failure::Build));
    assert!(mock.calls.is_empty(), "verifier must not be called");
}
