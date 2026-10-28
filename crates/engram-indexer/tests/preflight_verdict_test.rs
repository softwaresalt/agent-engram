//! Tests for the supervisor preflight verdict runner.

use std::time::{Duration, Instant};

use engram_indexer::preflight::{Failure, StageKind, VerificationError, Verifier};
use engram_indexer::preflight_verdict::{Verdict, run, stage_name};

struct Mock {
    fail_at: Option<StageKind>,
    calls: Vec<StageKind>,
}

impl Verifier for Mock {
    fn verify(
        &mut self,
        stage: StageKind,
        _deadline: Instant,
        _expected_generation: &str,
    ) -> Result<(), VerificationError> {
        self.calls.push(stage);
        if self.fail_at == Some(stage) {
            Err(VerificationError::new("mock failure"))
        } else {
            Ok(())
        }
    }
}

fn far_deadline() -> Instant {
    Instant::now() + Duration::from_secs(3600)
}

#[test]
fn all_pass_yields_succeeded() {
    let mut mock = Mock {
        fail_at: None,
        calls: Vec::new(),
    };
    let verdict = run(far_deadline(), "gen".to_string(), &mut mock);
    assert_eq!(verdict, Verdict::Succeeded);
    assert_eq!(verdict.json_line(), r#"{"state":"Succeeded"}"#);
    assert_eq!(verdict.exit_code(), 0);
    assert_eq!(mock.calls.len(), 7);
}

#[test]
fn each_stage_failure_yields_failed_and_stops() {
    let stages = [
        (StageKind::Build, Failure::Build, "Build"),
        (StageKind::Seal, Failure::Seal, "Seal"),
        (StageKind::Publish, Failure::Publish, "Publish"),
        (
            StageKind::DaemonVerified,
            Failure::DaemonVerified,
            "DaemonVerified",
        ),
        (
            StageKind::HealthVerified,
            Failure::HealthVerified,
            "HealthVerified",
        ),
        (
            StageKind::CliProbeVerified,
            Failure::CliProbeVerified,
            "CliProbeVerified",
        ),
        (
            StageKind::McpProbeVerified,
            Failure::McpProbeVerified,
            "McpProbeVerified",
        ),
    ];
    for (index, (kind, failure, name)) in stages.into_iter().enumerate() {
        let mut mock = Mock {
            fail_at: Some(kind),
            calls: Vec::new(),
        };
        let verdict = run(far_deadline(), "gen".to_string(), &mut mock);
        assert_eq!(verdict, Verdict::Failed(failure));
        assert_eq!(
            verdict.json_line(),
            format!(r#"{{"state":"Failed","stage":"{name}"}}"#)
        );
        assert_eq!(verdict.exit_code(), 1);
        assert_eq!(mock.calls.len(), index + 1);
        assert_eq!(mock.calls.last(), Some(&kind));
    }

    let mut mock = Mock {
        fail_at: None,
        calls: Vec::new(),
    };
    let verdict = run(Instant::now(), "gen".to_string(), &mut mock);
    assert_eq!(verdict, Verdict::Failed(Failure::Build));
    assert_eq!(verdict.json_line(), r#"{"state":"Failed","stage":"Build"}"#);
    assert_eq!(verdict.exit_code(), 1);
    assert!(mock.calls.is_empty());
}

#[test]
fn stage_name_maps_every_variant() {
    for failure in [
        Failure::Build,
        Failure::Seal,
        Failure::Publish,
        Failure::DaemonVerified,
        Failure::HealthVerified,
        Failure::CliProbeVerified,
        Failure::McpProbeVerified,
    ] {
        let expected = match failure {
            Failure::Build => "Build",
            Failure::Seal => "Seal",
            Failure::Publish => "Publish",
            Failure::DaemonVerified => "DaemonVerified",
            Failure::HealthVerified => "HealthVerified",
            Failure::CliProbeVerified => "CliProbeVerified",
            Failure::McpProbeVerified => "McpProbeVerified",
        };
        assert_eq!(stage_name(failure), expected);
        let line = Verdict::Failed(failure).json_line();
        assert_eq!(
            line,
            format!(r#"{{"state":"Failed","stage":"{}"}}"#, stage_name(failure))
        );
        assert!(!line.ends_with('\n'));
    }
}
