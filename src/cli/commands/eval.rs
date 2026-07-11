//! `engram eval` — retrieval + graph-recall evaluation CLI (081-F).
//!
//! Thin wrapper over the `run_retrieval_eval` MCP tool, following the
//! `report.rs` `run_tool` pattern. Engram owns the CLI + output contract;
//! autoharness owns invocation (mirrors the 064-F `engram verify` precedent).

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::cli::runner::{INDEXING_TIMEOUT_SECS, run_tool_timed};

/// `engram eval` → `run_retrieval_eval`
///
/// Emits the structured `RetrievalEvalReport` JSON to stdout via the shared
/// [`OutputFormatter`] and returns the tool exit code (`0` success, `1` tool
/// error, `2` connection/invocation failure). `--quiet` suppresses stdout;
/// callers rely on the exit code in that mode.
///
/// Uses the long-running command timeout ([`INDEXING_TIMEOUT_SECS`]) because a
/// run can parse every indexed source file and score up to `sample_size`
/// known-item queries; the global `--timeout` flag still overrides it.
pub async fn run_eval_retrieval(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    run_tool_timed(
        "run_retrieval_eval",
        None,
        flags,
        formatter,
        INDEXING_TIMEOUT_SECS,
    )
    .await
}
