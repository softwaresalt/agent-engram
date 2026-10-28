//! Stage-set coverage for NEW-4 (142.072-T): `RELAY_STAGES` equals `stage_name` lines.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use engram::cli::commands::preflight::{RELAY_STAGES, normalize_verdict};
use engram_indexer::preflight::Failure as F;
use engram_indexer::preflight_verdict::stage_name;

/// Exhaustive guard: adding a `Failure` variant breaks this match.
#[rustfmt::skip]
const fn idx(f: F) -> usize {
    match f { F::Build => 0, F::Seal => 1, F::Publish => 2, F::DaemonVerified => 3, F::HealthVerified => 4, F::CliProbeVerified => 5, F::McpProbeVerified => 6, }
}

#[rustfmt::skip]
const ALL: [F; 7] = [F::Build, F::Seal, F::Publish, F::DaemonVerified, F::HealthVerified, F::CliProbeVerified, F::McpProbeVerified];

fn line(f: F) -> String {
    format!(r#"{{"state":"Failed","stage":"{}"}}"#, stage_name(f))
}

#[test]
#[rustfmt::skip]
fn relay_stages_equal_stage_name_lines_both_ways() {
    assert_eq!(normalize_verdict(line(F::Build).as_bytes(), Some(1)), (RELAY_STAGES[0], 1));
    let derived: BTreeSet<String> = ALL.iter().map(|f| line(*f)).collect();
    let relay: BTreeSet<String> = RELAY_STAGES.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!((derived.len(), relay.len()), (7, 7), "distinct");
    assert_eq!(derived, relay);
    assert!(ALL.iter().enumerate().all(|(i, f)| idx(*f) == i), "variant order");
    assert_eq!(RELAY_STAGES[0], line(F::Build), "Build is element 0");
}

#[test]
#[rustfmt::skip]
fn normalize_verdict_returns_each_failed_line_unchanged() {
    for f in ALL {
        let l = line(f);
        let el = RELAY_STAGES.iter().copied().find(|s| *s == l).expect("element");
        assert_eq!(normalize_verdict(l.as_bytes(), Some(1)), (el, 1), "{l}");
        assert_eq!(normalize_verdict(format!("{l}\r\n").as_bytes(), Some(23)), (el, 1), "{l}");
    }
}
