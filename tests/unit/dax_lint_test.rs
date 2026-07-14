//! Unit tests for the Tier-1 DAX linter and `VerifyFinding.severity` (P5,
//! `085.005-T`).
//!
//! Covers: `Severity` default + legacy/round-trip serde back-compat, and each
//! Tier-1 rule (`dax.empty_expression`, `dax.divide_operator`,
//! `dax.deprecated_function`, `dax.malformed_ref`) with a positive and a
//! negative fixture, plus end-to-end `verify_tmdl_dax` conformance mapping.
//!
//! Tests: S-DAXLINT-01 through S-DAXLINT-14.

use engram::services::dax_lint::{lint_dax_expression, verify_tmdl_dax};
use engram::services::verify::{Severity, VerifyFinding};

fn rules(findings: &[VerifyFinding]) -> Vec<&str> {
    findings.iter().map(|f| f.rule.as_str()).collect()
}

// ── Severity serde back-compat ────────────────────────────────────────────

/// S-DAXLINT-01: `Severity::default()` is `Error` (required by `serde(default)`).
#[test]
fn severity_defaults_to_error() {
    assert_eq!(Severity::default(), Severity::Error);
}

/// S-DAXLINT-02: a legacy payload serialized WITHOUT `severity` deserializes to
/// `severity = Error`, preserving pre-severity blocking behaviour.
#[test]
fn legacy_finding_without_severity_deserializes_as_error() {
    let legacy = r#"{"rule":"frontmatter.malformed","message":"m","line":1}"#;
    let finding: VerifyFinding =
        serde_json::from_str(legacy).expect("legacy payload must deserialize");
    assert_eq!(finding.severity, Severity::Error);
    assert_eq!(finding.rule, "frontmatter.malformed");
}

/// S-DAXLINT-03: a full `VerifyFinding` round-trips through JSON unchanged.
#[test]
fn finding_round_trips_through_json() {
    let finding = VerifyFinding {
        rule: "dax.divide_operator".to_string(),
        message: "m".to_string(),
        line: Some(3),
        severity: Severity::Info,
    };
    let json = serde_json::to_string(&finding).expect("serialize");
    let back: VerifyFinding = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(finding, back);
}

/// S-DAXLINT-04: `Severity` serializes as a lowercase `snake_case` string.
#[test]
fn severity_serializes_snake_case() {
    let json = serde_json::to_string(&Severity::Warning).expect("serialize");
    assert_eq!(json, "\"warning\"");
}

// ── dax.empty_expression ──────────────────────────────────────────────────

/// S-DAXLINT-05 (positive): a comment-only expression is effectively empty.
#[test]
fn empty_expression_flags_comment_only_body() {
    let findings = lint_dax_expression("m.tmdl", "Sales[Blank] (measure)", "/* nothing yet */");
    assert!(
        rules(&findings).contains(&"dax.empty_expression"),
        "comment-only expression should flag dax.empty_expression: {findings:?}"
    );
    let flagged = findings
        .iter()
        .find(|f| f.rule == "dax.empty_expression")
        .expect("empty finding present");
    assert_eq!(flagged.severity, Severity::Warning);
}

/// S-DAXLINT-06 (negative): a real expression is not flagged as empty.
#[test]
fn empty_expression_ignores_real_body() {
    let findings = lint_dax_expression("m.tmdl", "Sales[Total] (measure)", "SUM(Sales[Amount])");
    assert!(
        !rules(&findings).contains(&"dax.empty_expression"),
        "non-empty expression must not flag dax.empty_expression: {findings:?}"
    );
}

// ── dax.divide_operator ───────────────────────────────────────────────────

/// S-DAXLINT-07 (positive): bare `/` division is flagged (Info).
#[test]
fn divide_operator_flags_bare_slash() {
    let findings = lint_dax_expression(
        "m.tmdl",
        "Sales[Ratio] (measure)",
        "Sales[Amount] / Sales[Qty]",
    );
    assert!(
        rules(&findings).contains(&"dax.divide_operator"),
        "bare '/' should flag dax.divide_operator: {findings:?}"
    );
    let flagged = findings
        .iter()
        .find(|f| f.rule == "dax.divide_operator")
        .expect("divide finding present");
    assert_eq!(flagged.severity, Severity::Info);
}

/// S-DAXLINT-08 (negative): `DIVIDE()` and `/` inside comments/strings are not
/// flagged.
#[test]
fn divide_operator_ignores_divide_function_and_literals() {
    let divide_fn = lint_dax_expression(
        "m.tmdl",
        "Sales[Ratio] (measure)",
        "DIVIDE(Sales[Amount], Sales[Qty])",
    );
    assert!(
        !rules(&divide_fn).contains(&"dax.divide_operator"),
        "DIVIDE() must not flag dax.divide_operator: {divide_fn:?}"
    );

    let in_comment =
        lint_dax_expression("m.tmdl", "Sales[X] (measure)", "SUM(Sales[Amount]) // a/b");
    assert!(
        !rules(&in_comment).contains(&"dax.divide_operator"),
        "'/' inside a comment must not flag dax.divide_operator: {in_comment:?}"
    );

    let in_string = lint_dax_expression(
        "m.tmdl",
        "Sales[X] (measure)",
        "\"http://example\" & Sales[Name]",
    );
    assert!(
        !rules(&in_string).contains(&"dax.divide_operator"),
        "'/' inside a string must not flag dax.divide_operator: {in_string:?}"
    );
}

// ── dax.deprecated_function ───────────────────────────────────────────────

/// S-DAXLINT-09 (positive): a call to a deprecated function is flagged (Warning).
#[test]
fn deprecated_function_flags_earlier() {
    let findings = lint_dax_expression(
        "m.tmdl",
        "Sales[Rank] (calculated column)",
        "CALCULATE(SUM(Sales[Amount]), EARLIER(Sales[Region]))",
    );
    assert!(
        rules(&findings).contains(&"dax.deprecated_function"),
        "EARLIER() should flag dax.deprecated_function: {findings:?}"
    );
    let flagged = findings
        .iter()
        .find(|f| f.rule == "dax.deprecated_function")
        .expect("deprecated finding present");
    assert_eq!(flagged.severity, Severity::Warning);
}

/// S-DAXLINT-10 (negative): non-deprecated functions are not flagged.
#[test]
fn deprecated_function_ignores_modern_functions() {
    let findings = lint_dax_expression(
        "m.tmdl",
        "Sales[Total] (measure)",
        "CALCULATE(SUM(Sales[Amount]))",
    );
    assert!(
        !rules(&findings).contains(&"dax.deprecated_function"),
        "modern functions must not flag dax.deprecated_function: {findings:?}"
    );
}

// ── dax.malformed_ref (from the extractor diagnostics seam) ────────────────

/// S-DAXLINT-11 (positive): an unterminated bracket is flagged (Error).
#[test]
fn malformed_ref_flags_unterminated_bracket() {
    let findings = lint_dax_expression("m.tmdl", "Sales[Broken] (measure)", "SUM(Sales[Amount");
    assert!(
        rules(&findings).contains(&"dax.malformed_ref"),
        "unterminated bracket should flag dax.malformed_ref: {findings:?}"
    );
    let flagged = findings
        .iter()
        .find(|f| f.rule == "dax.malformed_ref")
        .expect("malformed finding present");
    assert_eq!(flagged.severity, Severity::Error);
}

/// S-DAXLINT-12 (negative): a well-formed reference yields no malformed finding.
#[test]
fn malformed_ref_ignores_well_formed_reference() {
    let findings = lint_dax_expression("m.tmdl", "Sales[Total] (measure)", "SUM(Sales[Amount])");
    assert!(
        !rules(&findings).contains(&"dax.malformed_ref"),
        "well-formed reference must not flag dax.malformed_ref: {findings:?}"
    );
}

// ── verify_tmdl_dax end-to-end conformance ────────────────────────────────

/// S-DAXLINT-13: a clean TMDL model is conformant with no findings.
#[test]
fn verify_tmdl_dax_clean_model_is_conformant() {
    let content = "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure Total = SUM(Sales[Amount])\n";
    let report = verify_tmdl_dax("models/Sales.tmdl", content);
    assert!(
        report.conformant,
        "clean model should be conformant, findings: {:?}",
        report.findings
    );
}

/// S-DAXLINT-14: a TMDL model with a Tier-1 violation is non-conformant.
#[test]
fn verify_tmdl_dax_flags_violation() {
    let content = "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure Ratio = Sales[Amount] / 2\n";
    let report = verify_tmdl_dax("models/Sales.tmdl", content);
    assert!(
        !report.conformant,
        "model with '/' division should be non-conformant"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule == "dax.divide_operator"),
        "expected dax.divide_operator finding: {:?}",
        report.findings
    );
}
