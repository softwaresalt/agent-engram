//! Unit tests for the YAML frontmatter parser (002.001-T).
//!
//! Validates four scenarios: valid frontmatter, no frontmatter,
//! malformed YAML, and empty body.

use engram::services::parsing::frontmatter::{FrontmatterDocument, parse};

/// S-FM-01: valid frontmatter is parsed into metadata + body.
#[test]
fn valid_frontmatter_parsed() {
    let input = "---\nid: 001-T\ntitle: My Task\nstatus: queued\n---\n\n## Body\n\nContent here.";
    let doc = parse(input);

    assert!(doc.metadata.is_some(), "metadata should be present");
    let meta = doc.metadata.unwrap();
    assert_eq!(
        meta.get("id").and_then(|v| v.as_str()),
        Some("001-T"),
        "id field should be parseable"
    );
    assert_eq!(meta.get("title").and_then(|v| v.as_str()), Some("My Task"),);
    assert!(
        doc.body.contains("Content here."),
        "body text should follow the frontmatter block"
    );
}

/// S-FM-02: file without frontmatter delimiter returns None metadata.
#[test]
fn no_frontmatter_returns_none_metadata() {
    let input = "# Just a markdown heading\n\nSome paragraph text.";
    let doc = parse(input);

    assert!(
        doc.metadata.is_none(),
        "metadata should be None when no --- delimiter is present"
    );
    assert!(
        doc.body.contains("Just a markdown heading"),
        "body should contain the full original content"
    );
}

/// S-FM-03: malformed YAML between delimiters returns None metadata.
#[test]
fn malformed_yaml_returns_none_metadata() {
    let input = "---\n: invalid: yaml: {\n---\n\nBody text.";
    let doc = parse(input);

    assert!(
        doc.metadata.is_none(),
        "malformed YAML should produce None metadata, not a panic"
    );
    assert!(
        doc.body.contains("Body text."),
        "body should still be extracted after malformed YAML"
    );
}

/// S-FM-04: empty body after valid frontmatter returns empty string body.
#[test]
fn empty_body_after_frontmatter() {
    let input = "---\nid: 002-T\n---\n";
    let doc = parse(input);

    assert!(doc.metadata.is_some(), "metadata should parse successfully");
    assert!(
        doc.body.trim().is_empty(),
        "body should be empty when nothing follows the closing ---"
    );
}

/// S-FM-05: FrontmatterDocument is cloneable and debuggable.
#[test]
fn frontmatter_document_is_debug_clone() {
    let doc = FrontmatterDocument {
        metadata: None,
        body: "hello".to_string(),
    };
    let cloned = doc.clone();
    assert_eq!(cloned.body, "hello");
    let _ = format!("{doc:?}");
}
