//! Structural conformance linter for the `engram verify` gate (Phase 1a).
//!
//! Provides [`verify_markdown`], a deterministic per-file check that a markdown
//! document is conformant for `CozoDB` graph ingestion. It is pure and local:
//! no daemon, no database. The `engram verify` CLI subcommand drives this
//! service and maps [`VerifyReport::conformant`] to a process exit code so the
//! autoharness `pre_task_completion` gate can block on a non-zero exit.
//!
//! Phase 1a conformance is bound to what `services::ingestion` requires:
//! parseable frontmatter (present-but-malformed YAML is a hard failure that
//! closes the silent-`None` gap in [`crate::services::parsing::frontmatter`]),
//! a non-empty body, and no unresolved `{{...}}` template variables. Absent
//! frontmatter is permitted.

use crate::errors::EngramError;
use crate::services::parsing::frontmatter;

/// A single structural conformance finding produced by [`verify_markdown`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyFinding {
    /// Stable machine-readable rule identifier (e.g. `frontmatter.malformed`).
    pub rule: String,
    /// Human-readable diagnostic, written to stderr for agent context.
    pub message: String,
    /// One-based source line the finding refers to, when known.
    pub line: Option<usize>,
}

/// The outcome of verifying a markdown document for graph-ingestion conformance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyReport {
    /// `true` when the document is conformant (no blocking findings).
    pub conformant: bool,
    /// All structural findings discovered during verification.
    pub findings: Vec<VerifyFinding>,
}

impl VerifyReport {
    /// Build a report from a set of findings, deriving `conformant` from whether
    /// any findings were produced.
    fn from_findings(findings: Vec<VerifyFinding>) -> Self {
        Self {
            conformant: findings.is_empty(),
            findings,
        }
    }
}

/// Verify that a markdown document is structurally conformant for graph ingestion.
///
/// `rel_path` is the logical path of the document (used only to contextualise
/// diagnostics); `content` is the full document text. The returned
/// [`VerifyReport`] is conformant when it carries no findings.
///
/// Applied rules:
/// - `frontmatter.malformed`: the document opens with `---` and has a closing
///   `---`, but the enclosed YAML fails to deserialize. This distinguishes a
///   *present-but-malformed* block (blocked) from *absent* frontmatter
///   (permitted), closing the silent-`None` gap in `parsing::frontmatter`.
/// - `body.empty`: no ingestible body text remains after the frontmatter.
/// - `template.unresolved`: an unresolved `{{...}}` template variable remains.
///
/// # Errors
///
/// Returns [`EngramError`] to preserve a fallible contract for future
/// conformance rules; the Phase 1a rule set is infallible and always returns
/// `Ok`.
#[allow(clippy::unnecessary_wraps)]
pub fn verify_markdown(rel_path: &str, content: &str) -> Result<VerifyReport, EngramError> {
    let mut findings: Vec<VerifyFinding> = Vec::new();

    // Rule 1 — present-but-malformed frontmatter is a hard failure.
    if let Some(block) = present_frontmatter_block(content) {
        if !block.trim().is_empty() && serde_yaml::from_str::<serde_yaml::Mapping>(&block).is_err()
        {
            findings.push(VerifyFinding {
                rule: "frontmatter.malformed".to_string(),
                message: format!(
                    "{rel_path}: frontmatter delimiters present but YAML failed to parse"
                ),
                line: Some(1),
            });
        }
    }

    // Rule 2 — an empty body carries no ingestible content.
    let document = frontmatter::parse(content);
    if document.body.trim().is_empty() {
        findings.push(VerifyFinding {
            rule: "body.empty".to_string(),
            message: format!("{rel_path}: document body is empty after frontmatter"),
            line: None,
        });
    }

    // Rule 3 — unresolved `{{...}}` template variables block ingestion.
    findings.extend(unresolved_template_findings(rel_path, content));

    Ok(VerifyReport::from_findings(findings))
}

/// Return the raw YAML block when the document *presents* frontmatter — the
/// first line is `---` and a later line is a closing `---`. Returns `None` when
/// frontmatter is absent, mirroring the split in `parsing::frontmatter` so
/// verify can tell *absent* (permitted) from *present-but-malformed* (blocked).
fn present_frontmatter_block(content: &str) -> Option<String> {
    let mut lines = content.lines();
    if lines.next() != Some("---") {
        return None;
    }
    let mut yaml_lines: Vec<&str> = Vec::new();
    for line in lines {
        if line == "---" {
            return Some(yaml_lines.join("\n"));
        }
        yaml_lines.push(line);
    }
    None
}

/// Produce one `template.unresolved` finding per line containing a `{{...}}`
/// template marker.
fn unresolved_template_findings(rel_path: &str, content: &str) -> Vec<VerifyFinding> {
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| line_has_template_variable(line))
        .map(|(index, _)| VerifyFinding {
            rule: "template.unresolved".to_string(),
            message: format!(
                "{rel_path}: unresolved template variable on line {}",
                index + 1
            ),
            line: Some(index + 1),
        })
        .collect()
}

/// Detect an unresolved `{{...}}` template marker within a single line.
fn line_has_template_variable(line: &str) -> bool {
    line.find("{{")
        .is_some_and(|open| line[open + 2..].contains("}}"))
}
