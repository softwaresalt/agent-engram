//! Unit tests for the `engram verify` structural linter core (064.001-T).
//!
//! Validates four conformance scenarios for `services::verify::verify_markdown`:
//! 1. valid markdown with well-formed frontmatter is conformant;
//! 2. present-but-malformed YAML frontmatter is a hard finding
//!    (closes the silent-`None` gap in `frontmatter::parse`);
//! 3. an unresolved `{{...}}` template variable yields a finding;
//! 4. valid markdown without any frontmatter is conformant.

use engram::services::verify::verify_markdown;

/// S-VC-01: well-formed frontmatter + body is conformant with no findings.
#[test]
fn valid_markdown_with_frontmatter_is_conformant() {
    let content = "---\nid: 001-T\ntitle: My Task\nstatus: queued\n---\n\n# Heading\n\nBody content here.\n";
    let report = verify_markdown("docs/example.md", content).expect("verify must not error");

    assert!(
        report.conformant,
        "well-formed frontmatter + body should be conformant, findings: {:?}",
        report.findings
    );
    assert!(
        report.findings.is_empty(),
        "conformant document should carry no findings"
    );
}

/// S-VC-02: present-but-malformed YAML frontmatter is a non-conformant finding.
#[test]
fn present_but_malformed_frontmatter_is_non_conformant() {
    // Opening + closing `---` are present, but the YAML block is malformed.
    let content = "---\n: invalid: yaml: {\n---\n\n# Body\n\nText.\n";
    let report = verify_markdown("docs/broken.md", content).expect("verify must not error");

    assert!(
        !report.conformant,
        "malformed frontmatter must be non-conformant"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule == "frontmatter.malformed"),
        "expected a frontmatter.malformed finding, got: {:?}",
        report.findings
    );
}

/// S-VC-03: an unresolved `{{...}}` template variable yields a finding.
#[test]
fn unresolved_template_variable_is_finding() {
    let content = "# Title\n\nHello {{NAME}} welcome to {{PLACE}}.\n";
    let report = verify_markdown("docs/template.md", content).expect("verify must not error");

    assert!(
        !report.conformant,
        "unresolved template variables must be non-conformant"
    );
    let template_finding = report
        .findings
        .iter()
        .find(|f| f.rule == "template.unresolved")
        .expect("expected a template.unresolved finding");
    assert_eq!(
        template_finding.line,
        Some(3),
        "finding should report the 1-based line of the template variable"
    );
}

/// S-VC-04: valid markdown without frontmatter is conformant.
#[test]
fn valid_markdown_without_frontmatter_is_conformant() {
    let content = "# Heading\n\nJust body text, no frontmatter block at all.\n";
    let report = verify_markdown("docs/plain.md", content).expect("verify must not error");

    assert!(
        report.conformant,
        "absent frontmatter is permitted in Phase 1a, findings: {:?}",
        report.findings
    );
    assert!(
        report.findings.is_empty(),
        "plain markdown should carry no findings"
    );
}
