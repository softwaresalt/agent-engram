//! `engram eval` — retrieval + graph-recall evaluation CLI (081-F, 084.007-T).
//!
//! Thin wrapper over the `run_retrieval_eval` MCP tool, following the
//! `report.rs` `run_tool` pattern. Engram owns the CLI + output contract;
//! autoharness owns invocation (mirrors the 064-F `engram verify` precedent).

use serde_json::Value;

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::cli::runner::{INDEXING_TIMEOUT_SECS, run_tool_timed_capture};

/// Exit code when a run completes successfully but its report records a breached
/// threshold (084.007-T / 14B33F9F).
///
/// Kept distinct from the tool-error (`1`) and connection/invocation (`2`) codes
/// so a CI gate can tell a *metric regression* ("the run ran, but a configured
/// floor/ceiling was breached") apart from "the run failed to execute". This
/// mirrors the `engram verify` precedent of mapping a domain outcome to a
/// pinned, documented exit code.
const EXIT_THRESHOLDS_BREACHED: i32 = 3;

/// `engram eval` → `run_retrieval_eval`
///
/// Emits the structured `RetrievalEvalReport` JSON to stdout via the shared
/// [`OutputFormatter`] and returns the exit code:
/// - `0` — success, no configured threshold breached (includes disabled/empty
///   runs, which evaluate nothing and therefore cannot breach);
/// - `1` — tool error; `2` — connection/invocation failure;
/// - `3` — the run completed but a configured threshold was breached
///   ([`EXIT_THRESHOLDS_BREACHED`], 084.007-T).
///
/// `--quiet` suppresses stdout; callers rely on the exit code in that mode.
///
/// Uses the long-running command timeout ([`INDEXING_TIMEOUT_SECS`]) because a
/// run can parse every indexed source file and score up to `sample_size`
/// known-item queries; the global `--timeout` flag still overrides it.
pub async fn run_eval_retrieval(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    let (code, result) = run_tool_timed_capture(
        "run_retrieval_eval",
        None,
        flags,
        formatter,
        INDEXING_TIMEOUT_SECS,
    )
    .await;
    eval_exit_code(code, result.as_ref())
}

/// Map a successful eval run's report onto the final CLI exit code (084.007-T).
///
/// A non-zero `base_code` (tool/connection error) is surfaced unchanged — a
/// failed run is never masked by, nor re-labelled as, a threshold breach. On a
/// successful run (`base_code == 0`), a report whose `thresholds_breached` field
/// is `true` maps to [`EXIT_THRESHOLDS_BREACHED`]; everything else (pass,
/// disabled, empty, or a legacy report without the field) stays `0`.
fn eval_exit_code(base_code: i32, result: Option<&Value>) -> i32 {
    if base_code != 0 {
        return base_code;
    }
    let breached = result
        .and_then(|report| report.get("thresholds_breached"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if breached {
        EXIT_THRESHOLDS_BREACHED
    } else {
        base_code
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{EXIT_THRESHOLDS_BREACHED, eval_exit_code};

    #[test]
    fn breached_report_maps_to_nonzero_exit() {
        let report = json!({ "enabled": true, "thresholds_breached": true });
        assert_eq!(
            eval_exit_code(0, Some(&report)),
            EXIT_THRESHOLDS_BREACHED,
            "a breached run must map to a non-zero exit code"
        );
        assert_ne!(
            EXIT_THRESHOLDS_BREACHED, 0,
            "the breach code must be non-zero"
        );
    }

    #[test]
    fn passing_report_maps_to_exit_zero() {
        let report = json!({ "enabled": true, "thresholds_breached": false });
        assert_eq!(
            eval_exit_code(0, Some(&report)),
            0,
            "a passing run must exit 0"
        );
    }

    #[test]
    fn disabled_or_empty_report_maps_to_exit_zero() {
        // Disabled/empty runs report thresholds_breached=false ...
        let disabled = json!({ "enabled": false, "thresholds_breached": false });
        assert_eq!(eval_exit_code(0, Some(&disabled)), 0);
        // ... and a legacy report missing the field must not spuriously breach.
        let legacy = json!({ "enabled": true });
        assert_eq!(eval_exit_code(0, Some(&legacy)), 0);
    }

    #[test]
    fn tool_and_connection_errors_are_surfaced_unchanged() {
        // A tool error (1) or connection error (2) is never masked, and a breach
        // is never inferred when the run itself failed.
        let breached = json!({ "thresholds_breached": true });
        assert_eq!(eval_exit_code(1, None), 1, "tool error surfaced unchanged");
        assert_eq!(
            eval_exit_code(2, None),
            2,
            "connection error surfaced unchanged"
        );
        assert_eq!(
            eval_exit_code(1, Some(&breached)),
            1,
            "a failed run is not re-labelled as a breach"
        );
    }
}
