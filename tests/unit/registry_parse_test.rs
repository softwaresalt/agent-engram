//! Unit tests for registry YAML parsing and validation (T011).
//!
//! Covers scenarios: S001, S005, S008, S011, S012, S014.

use engram::services::registry::parse_registry_yaml;

#[test]
fn valid_three_source_registry() {
    let yaml = r"
sources:
  - type: code
    language: rust
    path: src
  - type: tests
    language: rust
    path: tests
  - type: docs
    language: markdown
    path: docs
";
    let config = parse_registry_yaml(yaml).expect("valid YAML should parse");
    assert_eq!(config.sources.len(), 3);
    assert_eq!(config.sources[0].content_type, "code");
    assert_eq!(config.sources[0].language.as_deref(), Some("rust"));
    assert_eq!(config.sources[0].path, "src");
    assert_eq!(config.sources[1].content_type, "tests");
    assert_eq!(config.sources[2].content_type, "docs");
    assert_eq!(config.max_file_size_bytes, 1_048_576);
    assert_eq!(config.batch_size, 50);
}

#[test]
fn empty_sources_fallback() {
    let yaml = "sources: []\n";
    let config = parse_registry_yaml(yaml).expect("empty sources should parse");
    assert!(config.sources.is_empty());
}

#[test]
fn invalid_yaml_syntax_returns_error() {
    let yaml = "sources:\n  - type: code\n    path src\n  invalid: {[}";
    let err = parse_registry_yaml(yaml).expect_err("malformed YAML should fail");
    let msg = err.to_string();
    assert!(
        msg.contains("parse") || msg.contains("YAML") || msg.contains("registry"),
        "Error should mention parsing: {msg}"
    );
}

#[test]
fn max_file_size_zero_rejected() {
    let yaml = "sources: []\nmax_file_size_bytes: 0\n";
    let err = parse_registry_yaml(yaml).expect_err("zero max_file_size should fail");
    assert!(err.to_string().contains("greater than 0"));
}

#[test]
fn max_file_size_over_100mb_rejected() {
    let yaml = "sources: []\nmax_file_size_bytes: 209715200\n";
    let err = parse_registry_yaml(yaml).expect_err("200MB should exceed limit");
    assert!(err.to_string().contains("100 MB"));
}

#[test]
fn max_file_size_at_100mb_accepted() {
    let yaml = "sources: []\nmax_file_size_bytes: 104857600\n";
    let config = parse_registry_yaml(yaml).expect("100MB exactly should be accepted");
    assert_eq!(config.max_file_size_bytes, 104_857_600);
}

#[test]
fn batch_size_zero_rejected() {
    let yaml = "sources: []\nbatch_size: 0\n";
    let err = parse_registry_yaml(yaml).expect_err("zero batch_size should fail");
    assert!(err.to_string().contains("greater than 0"));
}

#[test]
fn batch_size_over_500_rejected() {
    let yaml = "sources: []\nbatch_size: 501\n";
    let err = parse_registry_yaml(yaml).expect_err("501 batch_size should fail");
    assert!(err.to_string().contains("500"));
}

#[test]
fn batch_size_at_500_accepted() {
    let yaml = "sources: []\nbatch_size: 500\n";
    let config = parse_registry_yaml(yaml).expect("500 should be accepted");
    assert_eq!(config.batch_size, 500);
}

#[test]
fn custom_content_type_accepted() {
    let yaml = r"
sources:
  - type: tracking
    path: .copilot-tracking
";
    let config = parse_registry_yaml(yaml).expect("custom type should be accepted");
    assert_eq!(config.sources[0].content_type, "tracking");
}

#[test]
fn built_in_type_accepted() {
    let yaml = r"
sources:
  - type: code
    language: rust
    path: src
";
    let config = parse_registry_yaml(yaml).expect("built-in type should parse");
    assert_eq!(config.sources[0].content_type, "code");
}

#[test]
fn pattern_field_parsed_correctly() {
    let yaml = r"
sources:
  - type: spec
    path: .backlog/documents
    pattern: '*-research.md'
  - type: backlog
    path: .backlog/tasks
  - type: memory
    path: .backlog/archive
";
    let config = parse_registry_yaml(yaml).expect("pattern should parse");
    assert_eq!(config.sources.len(), 3);
    assert_eq!(config.sources[0].pattern.as_deref(), Some("*-research.md"));
    assert!(config.sources[1].pattern.is_none());
    assert!(config.sources[2].pattern.is_none());
}

#[test]
fn defaults_applied_when_omitted() {
    let yaml = r"
sources:
  - type: docs
    path: docs
";
    let config = parse_registry_yaml(yaml).expect("defaults should apply");
    assert_eq!(config.max_file_size_bytes, 1_048_576);
    assert_eq!(config.batch_size, 50);
    assert!(config.sources[0].language.is_none());
}
