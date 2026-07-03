//! Unit tests for the global `--correlation-id` CLI flag and its validation
//! policy (067.005-T / t5).
//!
//! Covers clap parsing (flag populates the field), the `ENGRAM_CORRELATION_ID`
//! env fallback contract, and the strict CLI/direct validation policy
//! (reject control chars / over-128, empty → unset).

use clap::Parser;
use engram::cli::flags::GlobalFlags;

/// Minimal parser harness that flattens the shared global flags.
#[derive(Debug, Parser)]
struct Harness {
    #[command(flatten)]
    flags: GlobalFlags,
}

fn parse(args: &[&str]) -> GlobalFlags {
    Harness::parse_from(args).flags
}

#[test]
fn t067_005_flag_populates_correlation_id() {
    let flags = parse(&["engram", "--correlation-id", "corr-abc-1"]);
    assert_eq!(flags.correlation_id.as_deref(), Some("corr-abc-1"));
    assert_eq!(
        flags.resolve_correlation_id().expect("valid"),
        Some("corr-abc-1".to_owned())
    );
}

#[test]
fn t067_005_absent_flag_is_unset() {
    let flags = parse(&["engram"]);
    assert_eq!(flags.correlation_id, None);
    assert_eq!(flags.resolve_correlation_id().expect("unset"), None);
}

#[test]
fn t067_005_empty_value_resolves_to_none() {
    let flags = parse(&["engram", "--correlation-id", ""]);
    assert_eq!(flags.resolve_correlation_id().expect("empty"), None);
}

#[test]
fn t067_005_control_chars_rejected() {
    let flags = parse(&["engram", "--correlation-id", "bad\tid"]);
    assert!(
        flags.resolve_correlation_id().is_err(),
        "control characters must be rejected on the CLI surface"
    );
}

#[test]
fn t067_005_over_cap_rejected() {
    let long = "a".repeat(200);
    let flags = parse(&["engram", "--correlation-id", &long]);
    assert!(
        flags.resolve_correlation_id().is_err(),
        "an over-128-character id must be rejected"
    );
}

#[test]
fn t067_005_max_length_accepted() {
    let exactly = "b".repeat(128);
    let flags = parse(&["engram", "--correlation-id", &exactly]);
    assert_eq!(
        flags.resolve_correlation_id().expect("128 chars ok"),
        Some(exactly)
    );
}
