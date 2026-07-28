//! Indexing subcommands: sync and index (the critical preloading commands).

use serde_json::json;

use crate::cli::direct::run_direct_sync;
use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::cli::runner::{INDEXING_TIMEOUT_SECS, run_tool, run_tool_timed};

/// `engram sync [--full] [--force] [--direct]` — incremental or full workspace index.
///
/// Without `--direct`, routes through the IPC daemon (auto-spawned if needed).
/// With `--direct`, acquires the daemon lock and runs service functions in-process,
/// then exits when complete. Useful for pre-loading the index from a startup script
/// before launching an MCP host.
///
/// `--force` re-parses and re-embeds all discovered files (bypassing the
/// content-hash skip) and implies the full-scan path.
// This is a thin CLI-forwarding shim: each bool maps 1:1 to a documented sync
// flag (`--full`, `--force`, `--backfill-python-canonical`, `--direct`), so a
// two-variant enum would add indirection without improving call-site clarity.
#[allow(clippy::fn_params_excessive_bools)]
pub async fn run_sync(
    full: bool,
    force: bool,
    backfill_python_canonical: bool,
    revalidate_code_graph: bool,
    direct: bool,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    if direct {
        let workspace = match flags.resolve_workspace() {
            Ok(p) => p,
            Err(e) => return formatter.cli_error(&e),
        };
        let correlation_id = match flags.resolve_correlation_id() {
            Ok(v) => v,
            Err(e) => return formatter.cli_error(&format!("invalid --correlation-id: {e}")),
        };
        return run_direct_sync(
            &workspace,
            full,
            force,
            backfill_python_canonical,
            revalidate_code_graph,
            flags.id_value(),
            correlation_id,
            formatter,
        )
        .await;
    }
    // `--backfill-python-canonical` / `--revalidate-code-graph` on the full-scan
    // path imply `--force` (parity with `engram index`): both migrations are a
    // forced re-extraction, so `sync --full --<gate>` must re-extract rather than
    // silently hash-skip and drop the flag. The bare incremental `sync --<gate>`
    // (no `--full`) keeps its gated path below.
    let force = force || (full && (backfill_python_canonical || revalidate_code_graph));
    if full || force {
        // Full re-index can take minutes on large workspaces — use extended timeout.
        run_tool_timed(
            "index_workspace",
            force_params(force),
            flags,
            formatter,
            INDEXING_TIMEOUT_SECS,
        )
        .await
    } else {
        // Incremental sync: the gated T7 backfill (096.010-T) and the 101-F
        // code-graph revalidation are the only paths on which a stale marker
        // re-extracts already-indexed files. Without a flag, a stale marker is a
        // no-op (routine sync never silently re-extracts or churns — C12-5).
        run_tool(
            "sync_workspace",
            sync_params(backfill_python_canonical, revalidate_code_graph),
            flags,
            formatter,
        )
        .await
    }
}

/// `engram index [--force] [--backfill-python-canonical] [--revalidate-code-graph] [--direct]`
/// — full scan; alias for `engram sync --full`.
#[allow(clippy::fn_params_excessive_bools)]
pub async fn run_index(
    force: bool,
    backfill_python_canonical: bool,
    revalidate_code_graph: bool,
    direct: bool,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    // On the full-scan index path both migrations are a forced re-extraction, so
    // `--backfill-python-canonical` and `--revalidate-code-graph` imply `--force`
    // here (parity with `sync`).
    let force = force || backfill_python_canonical || revalidate_code_graph;
    if direct {
        let workspace = match flags.resolve_workspace() {
            Ok(p) => p,
            Err(e) => return formatter.cli_error(&e),
        };
        let correlation_id = match flags.resolve_correlation_id() {
            Ok(v) => v,
            Err(e) => return formatter.cli_error(&format!("invalid --correlation-id: {e}")),
        };
        return run_direct_sync(
            &workspace,
            true,
            force,
            backfill_python_canonical,
            revalidate_code_graph,
            flags.id_value(),
            correlation_id,
            formatter,
        )
        .await;
    }
    // Full re-index can take minutes on large workspaces — use extended timeout.
    run_tool_timed(
        "index_workspace",
        force_params(force),
        flags,
        formatter,
        INDEXING_TIMEOUT_SECS,
    )
    .await
}

/// Build `index_workspace` params for the `force` flag: `Some({"force": true})`
/// when forcing a re-parse, `None` otherwise (preserving the default fast path).
fn force_params(force: bool) -> Option<serde_json::Value> {
    if force {
        Some(json!({ "force": true }))
    } else {
        None
    }
}

/// Build `sync_workspace` params for the incremental-sync gates
/// (`--backfill-python-canonical`, `--revalidate-code-graph`): an object
/// carrying whichever gates were requested, or `None` when neither is set
/// (preserving the default incremental fast path).
fn sync_params(
    backfill_python_canonical: bool,
    revalidate_code_graph: bool,
) -> Option<serde_json::Value> {
    let mut map = serde_json::Map::new();
    if backfill_python_canonical {
        map.insert("backfill_python_canonical".to_owned(), json!(true));
    }
    if revalidate_code_graph {
        map.insert("revalidate_code_graph".to_owned(), json!(true));
    }
    if map.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(map))
    }
}

// Routing behaviour is covered by tests/integration/cli_direct_test.rs which
// runs the binary as a subprocess and verifies the actual dispatch paths.
