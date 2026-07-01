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

use std::path::{Component, Path, PathBuf};

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
    // Resolve the workspace root for containment (Constitution Principle III).
    let workspace = match flags.resolve_workspace() {
        Ok(root) => root,
        Err(err) => {
            fmt.cli_error(&format!("cannot resolve workspace: {err}"));
            return EXIT_ERROR;
        }
    };

    // Resolve the target against the workspace root and enforce containment.
    let target = match contain_path(&path, &workspace) {
        Ok(target) => target,
        Err(err) => {
            fmt.cli_error(&err);
            return EXIT_ERROR;
        }
    };

    // Non-markdown targets carry no graph markdown to validate in Phase 1a.
    if !is_markdown_path(&target.display) {
        emit_summary(&target.display, true, &[], flags, fmt);
        return EXIT_CONFORMANT;
    }

    let content = match tokio::fs::read_to_string(&target.read).await {
        Ok(text) => text,
        Err(err) => {
            fmt.cli_error(&format!("cannot read '{}': {err}", target.display));
            return EXIT_ERROR;
        }
    };

    let report = match verify::verify_markdown(&target.display, &content) {
        Ok(report) => report,
        Err(err) => {
            fmt.cli_error(&format!(
                "verification error for '{}': {err}",
                target.display
            ));
            return EXIT_ERROR;
        }
    };

    if !report.conformant {
        for finding in &report.findings {
            emit_finding_to_stderr(finding);
        }
    }
    emit_summary(
        &target.display,
        report.conformant,
        &report.findings,
        flags,
        fmt,
    );

    if report.conformant {
        EXIT_CONFORMANT
    } else {
        EXIT_NON_CONFORMANT
    }
}

/// A verify target resolved against the workspace root.
///
/// `read` is the absolute path actually opened; a relative `<path>` is joined
/// under the workspace root — never the process CWD. `display` is the
/// forward-slash convention string used in the stdout summary and stderr
/// diagnostics so output stays stable and platform-independent.
struct ResolvedTarget {
    read: PathBuf,
    display: String,
}

/// Resolve `path` against the `workspace` root and enforce containment
/// (Constitution Principle III/IV).
///
/// The established `.replace('\\', "/")` convention is applied so Windows-style
/// backslash paths resolve identically on Linux and drive the forward-slash
/// `display` string. A relative `<path>` is joined under `workspace` — not the
/// process CWD — so containment holds even when `--workspace` /
/// `ENGRAM_WORKSPACE` differs from the CWD.
///
/// Any `..` parent-directory component is rejected outright. The resolved target
/// (canonicalized when it exists, lexically joined under the canonical root when
/// missing) must remain within the canonicalized workspace root; an absolute
/// path — or any resolution — outside the root is rejected so the gate cannot be
/// pointed at files beyond the workspace.
fn contain_path(path: &str, workspace: &Path) -> Result<ResolvedTarget, String> {
    let normalized = path.replace('\\', "/");
    let candidate = Path::new(&normalized);

    // Reject explicit parent-directory traversal outright.
    if candidate
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!(
            "path '{path}' escapes the workspace root via a '..' component"
        ));
    }

    // Canonicalize the workspace root so containment comparisons are stable
    // across symlinks and platform-specific path forms (e.g. Windows 8.3 names).
    let workspace_root = workspace.canonicalize().map_err(|err| {
        format!(
            "cannot resolve workspace root '{}': {err}",
            workspace.display()
        )
    })?;

    // A relative <path> is resolved under the workspace root, not the CWD, so a
    // workspace that differs from the CWD cannot be bypassed.
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        workspace_root.join(candidate)
    };

    // Enforce containment on the resolved target: canonicalize when it exists;
    // otherwise keep the lexical join (already free of `..`) so a missing file
    // yields a clean exit-2 read error rather than escaping the root.
    let read = if joined.exists() {
        joined
            .canonicalize()
            .map_err(|err| format!("cannot resolve path '{path}': {err}"))?
    } else {
        joined
    };

    if !read.starts_with(&workspace_root) {
        return Err(format!("path '{path}' is outside the workspace root"));
    }

    Ok(ResolvedTarget {
        read,
        display: normalized,
    })
}

/// Whether `path` names a markdown document (`.md` / `.markdown`).
fn is_markdown_path(path: &str) -> bool {
    Path::new(path)
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
