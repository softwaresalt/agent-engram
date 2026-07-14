//! `engram lint-dax [<model.tmdl>]` — daemon-backed DAX lint (P7, `085.007-T`).
//!
//! Thin CLI mirror of the `lint_dax` MCP tool (P6). Unlike the local, no-daemon
//! `engram verify`, the Tier-2 semantic lint needs the resolved model-scope
//! schema, so this subcommand is daemon-backed — modelled on `engram impact`
//! (which mirrors `impact_analysis`). The optional `<model.tmdl>` argument maps
//! to the tool's `model_path` selector (canonicalized to one model scope; NOT
//! the shared `source_path` content-source directory).
//!
//! The `{ conformant, findings[] }` result is rendered by the shared runner and
//! mapped onto the `engram verify` exit-code contract:
//! - `0` — conformant (no findings);
//! - `1` — findings present;
//! - `2` — error (unindexed `model_path`, unbound workspace, or daemon failure).

use serde_json::{Value, json};

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::cli::runner::run_tool_timed_capture;

/// Conformant model(s): no findings.
const EXIT_CONFORMANT: i32 = 0;
/// Findings present (non-conformant).
const EXIT_FINDINGS: i32 = 1;
/// Error: unindexed model, unbound workspace, or daemon/connection failure.
const EXIT_ERROR: i32 = 2;
/// Per-command IPC timeout default (seconds), matching the shared CLI default.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Run `engram lint-dax [<model.tmdl>]`: call the daemon `lint_dax` tool and map
/// its report to the pinned exit-code contract (`0`/`1`/`2`).
///
/// `model_path` is the optional TMDL model path forwarded as the tool's
/// `model_path` selector; `None` lints every indexed model in the bound
/// workspace.
pub async fn run_lint_dax(
    model_path: Option<String>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    let params = model_path.map(|path| json!({ "model_path": path }));
    let (_base, result) =
        run_tool_timed_capture("lint_dax", params, flags, formatter, DEFAULT_TIMEOUT_SECS).await;
    match result {
        Some(value) => {
            if value
                .get("conformant")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                EXIT_CONFORMANT
            } else {
                EXIT_FINDINGS
            }
        }
        // `run_tool_timed_capture` returns `None` on both tool errors (e.g. an
        // unindexed `model_path` → `WorkspaceNotFound`, or an unbound workspace)
        // and connection failures; both collapse to the verify error exit.
        None => EXIT_ERROR,
    }
}
