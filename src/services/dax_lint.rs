//! Tier-1 (syntactic) DAX linter driving the `engram verify <model.tmdl>` gate.
//!
//! Tier 1 is deterministic, pure, and local (no daemon, no database): it parses
//! a TMDL document, extracts each measure and calculated-column DAX expression,
//! and applies a small set of syntactic rules that produce
//! [`VerifyFinding`]s. The `engram verify` CLI maps the resulting
//! [`VerifyReport::conformant`] flag onto its pinned exit-code contract so the
//! linter is usable as a pre-commit / autoharness gate.
//!
//! Rules (all namespaced `dax.*`):
//! - `dax.empty_expression` ([`Severity::Warning`]): the expression has no
//!   content once comments and whitespace are removed.
//! - `dax.divide_operator` ([`Severity::Info`]): the expression divides with the
//!   `/` operator instead of the `DIVIDE()` function (unguarded division).
//! - `dax.deprecated_function` ([`Severity::Warning`]): the expression calls a
//!   function flagged as legacy / discouraged.
//! - `dax.malformed_ref` ([`Severity::Error`]): driven by the P1 extractor's
//!   diagnostics seam (unterminated string / quoted identifier / bracket /
//!   block comment) rather than re-lexing.

use powerbi_tmdl_parser::{DaxDiagnostic, extract_dax_references};

use crate::services::powerbi_tmdl::extract_tmdl_semantic_model;
use crate::services::verify::{Severity, VerifyFinding, VerifyReport};

/// DAX functions flagged as legacy / discouraged.
///
/// Row-context iteration via `EARLIER` / `EARLIEST` is flagged by common DAX
/// best-practice analyzers in favour of `VAR` / `RETURN` variables, which are
/// clearer and avoid nested-row-context pitfalls.
const DEPRECATED_FUNCTIONS: &[&str] = &["EARLIER", "EARLIEST"];

/// Run the Tier-1 DAX lint over every measure and calculated column in a TMDL
/// document.
///
/// Returns a [`VerifyReport`] whose `conformant` flag mirrors the existing
/// verify exit-code contract (conformant iff no findings). A document with no
/// extractable semantic model (e.g. a non-model TMDL fragment) yields a
/// conformant, empty report.
#[must_use]
pub fn verify_tmdl_dax(rel_path: &str, content: &str) -> VerifyReport {
    let mut findings: Vec<VerifyFinding> = Vec::new();

    if let Some(model) = extract_tmdl_semantic_model(content, rel_path) {
        for table in &model.tables {
            for measure in &table.measures {
                if let Some(expression) = measure.expression.as_deref() {
                    let location = format!("{}[{}] (measure)", table.name, measure.name);
                    findings.extend(lint_dax_expression(rel_path, &location, expression));
                }
            }
            for column in &table.columns {
                if let Some(expression) = column.expression.as_deref() {
                    let location = format!("{}[{}] (calculated column)", table.name, column.name);
                    findings.extend(lint_dax_expression(rel_path, &location, expression));
                }
            }
        }
    }

    VerifyReport::from_findings(findings)
}

/// Apply the Tier-1 rule set to a single DAX `expression`.
///
/// `location` contextualises the diagnostic (e.g. `Sales[Total Sales]
/// (measure)`). Findings are dropped, never fabricated: `dax.malformed_ref` is
/// emitted only from the extractor's diagnostics seam, not by re-lexing.
#[must_use]
pub fn lint_dax_expression(rel_path: &str, location: &str, expression: &str) -> Vec<VerifyFinding> {
    let mut findings: Vec<VerifyFinding> = Vec::new();
    let references = extract_dax_references(expression);

    if is_effectively_empty(expression) {
        findings.push(finding(
            rel_path,
            location,
            "dax.empty_expression",
            "DAX expression is empty".to_string(),
            Severity::Warning,
        ));
    }

    if contains_bare_division(expression) {
        findings.push(finding(
            rel_path,
            location,
            "dax.divide_operator",
            "uses the '/' operator; prefer DIVIDE() for guarded division".to_string(),
            Severity::Info,
        ));
    }

    for function in &references.functions {
        if DEPRECATED_FUNCTIONS
            .iter()
            .any(|deprecated| deprecated.eq_ignore_ascii_case(function))
        {
            findings.push(finding(
                rel_path,
                location,
                "dax.deprecated_function",
                format!("calls deprecated function {function}()"),
                Severity::Warning,
            ));
        }
    }

    for diagnostic in &references.diagnostics {
        findings.push(finding(
            rel_path,
            location,
            "dax.malformed_ref",
            malformed_message(diagnostic).to_string(),
            Severity::Error,
        ));
    }

    findings
}

/// Build a `dax.*` [`VerifyFinding`]. Line is unknown (`None`) because the model
/// adapter does not preserve per-expression source spans; the `location`
/// carried in `message` identifies the offending member instead.
fn finding(
    rel_path: &str,
    location: &str,
    rule: &str,
    message: String,
    severity: Severity,
) -> VerifyFinding {
    VerifyFinding {
        rule: rule.to_string(),
        message: format!("{rel_path}: {location}: {message}"),
        line: None,
        severity,
    }
}

/// Map an extractor diagnostic to a stable `dax.malformed_ref` message.
fn malformed_message(diagnostic: &DaxDiagnostic) -> &'static str {
    match diagnostic {
        DaxDiagnostic::UnterminatedString => "unterminated string literal in DAX expression",
        DaxDiagnostic::UnterminatedQuotedIdentifier => {
            "unterminated quoted table identifier in DAX expression"
        }
        DaxDiagnostic::UnterminatedBlockComment => "unterminated block comment in DAX expression",
        DaxDiagnostic::UnterminatedBracket => "unterminated bracketed reference in DAX expression",
    }
}

/// Whether an expression is empty once DAX comments and whitespace are removed.
fn is_effectively_empty(expression: &str) -> bool {
    strip_comments(expression).trim().is_empty()
}

/// Remove `//` line comments and `/* */` block comments from a DAX expression.
///
/// This is a coarse pass used only for the emptiness check; it does not attempt
/// to preserve string literals, which is acceptable because any surviving
/// string content still makes the expression non-empty.
fn strip_comments(expression: &str) -> String {
    let chars: Vec<char> = expression.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(chars.len());
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// Whether the expression uses a bare `/` division operator.
///
/// Strings, quoted identifiers, bracketed references, and comments are skipped
/// so a `/` inside a literal or a `//` / `/*` comment is not misreported.
fn contains_bare_division(expression: &str) -> bool {
    let chars: Vec<char> = expression.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' => {
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    i += 1;
                }
                i += 1;
            }
            '\'' => {
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
            }
            '[' => {
                i += 1;
                while i < chars.len() && chars[i] != ']' {
                    i += 1;
                }
                i += 1;
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '/' => {
                i += 2;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
            }
            '/' => return true,
            _ => i += 1,
        }
    }
    false
}
