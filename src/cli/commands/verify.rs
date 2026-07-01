//! `engram verify <path>` — structural conformance gate (Phase 1a).
//!
//! Local, no-daemon command (a `manifest` analog) that reads a single file,
//! runs [`crate::services::verify::verify_markdown`], and maps the result to a
//! pinned process exit-code contract consumed by the autoharness
//! `pre_task_completion` gate:
//!
//! - `0` — conformant (or a non-markdown target, which has nothing to validate);
//! - `1` — non-conformant (structural findings; each is written to stderr);
//! - `2` — I/O or usage error (missing / unreadable file, bad arguments).
//!
//! Findings are written to **stderr** so autoharness can inject them into the
//! agent's context window; a machine-readable summary envelope is written to
//! stdout.

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::services::verify::{self, VerifyFinding};

/// Conformant document (or non-markdown target).
const EXIT_CONFORMANT: i32 = 0;
/// Non-conformant document — blocks the autoharness gate.
const EXIT_NON_CONFORMANT: i32 = 1;
/// I/O or usage error.
const EXIT_ERROR: i32 = 2;

/// Run `engram verify <path>`: verify one file for graph-ingestion conformance.
///
/// Returns the pinned exit code (`0`/`1`/`2`); see the module documentation for
/// the contract. Runs locally with no daemon and no database.
pub async fn run_verify(path: String, flags: &GlobalFlags, fmt: &OutputFormatter) -> i32 {
    // Non-markdown targets carry no graph markdown to validate in Phase 1a.
    if !is_markdown_path(&path) {
        emit_summary(&path, true, &[], flags, fmt);
        return EXIT_CONFORMANT;
    }

    let content = match tokio::fs::read_to_string(&path).await {
        Ok(text) => text,
        Err(err) => {
            fmt.cli_error(&format!("cannot read '{path}': {err}"));
            return EXIT_ERROR;
        }
    };

    let report = match verify::verify_markdown(&path, &content) {
        Ok(report) => report,
        Err(err) => {
            fmt.cli_error(&format!("verification error for '{path}': {err}"));
            return EXIT_ERROR;
        }
    };

    if !report.conformant {
        for finding in &report.findings {
            emit_finding_to_stderr(finding);
        }
    }
    emit_summary(&path, report.conformant, &report.findings, flags, fmt);

    if report.conformant {
        EXIT_CONFORMANT
    } else {
        EXIT_NON_CONFORMANT
    }
}

/// Whether `path` names a markdown document (`.md` / `.markdown`).
fn is_markdown_path(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

/// Write a single finding to stderr so autoharness can inject it into context.
fn emit_finding_to_stderr(finding: &VerifyFinding) {
    if let Some(line) = finding.line {
        eprintln!("[{}] {} (line {line})", finding.rule, finding.message);
    } else {
        eprintln!("[{}] {}", finding.rule, finding.message);
    }
}

/// Emit a machine-readable result envelope to stdout summarising the outcome.
fn emit_summary(
    path: &str,
    conformant: bool,
    findings: &[VerifyFinding],
    flags: &GlobalFlags,
    fmt: &OutputFormatter,
) {
    let findings_json: Vec<serde_json::Value> = findings
        .iter()
        .map(|finding| {
            serde_json::json!({
                "rule": finding.rule,
                "message": finding.message,
                "line": finding.line,
            })
        })
        .collect();
    fmt.success(
        flags.id_value(),
        serde_json::json!({
            "path": path,
            "conformant": conformant,
            "findings": findings_json,
        }),
    );
}
