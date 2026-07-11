//! `engram eval` — retrieval + graph-recall evaluation CLI (081-F).
//!
//! Thin wrapper over the `run_retrieval_eval` MCP tool, following the
//! `report.rs` `run_tool` pattern. Engram owns the CLI + output contract;
//! autoharness owns invocation (mirrors the 064-F `engram verify` precedent).

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::cli::runner::run_tool;

/// `engram eval` → `run_retrieval_eval`
///
/// Emits the structured `RetrievalEvalReport` JSON to stdout via the shared
/// [`OutputFormatter`] and returns the tool exit code (`0` success, `1` tool
/// error, `2` connection/invocation failure). `--quiet` suppresses stdout;
/// callers rely on the exit code in that mode.
pub async fn run_eval_retrieval(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    run_tool("run_retrieval_eval", None, flags, formatter).await
}
