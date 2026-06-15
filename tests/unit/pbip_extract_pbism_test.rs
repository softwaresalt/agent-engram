//! Unit tests for PBIP `.pbism` descriptor extraction (062.003-T).
//!
//! Verifies that the new pbip extractor can parse `.pbism` JSON descriptors
//! from the project-definition layout into a stable `PbismDescriptor`.
//!
//! Tests: S-PBM-01, S-PBM-02, S-PBM-03, S-PBM-04, S-PBM-05

use engram::services::pbip_extract::parse_pbism;

/// S-PBM-01: A real-fixture `.pbism` descriptor parses and yields its version.
#[test]
fn parse_pbism_returns_version() {
    let content = r#"{
  "version": "4.0",
  "settings": {}
}
"#;
    let descriptor = parse_pbism(content).expect("real .pbism fixture should parse");
    assert_eq!(descriptor.version, "4.0");
}

/// S-PBM-02: A `.pbism` without a `version` field is rejected as not a valid descriptor.
#[test]
fn parse_pbism_returns_none_without_version() {
    let content = r#"{ "settings": {} }"#;
    assert!(
        parse_pbism(content).is_none(),
        ".pbism without a version field should not be treated as a descriptor"
    );
}

/// S-PBM-03: Plain non-JSON text is rejected.
#[test]
fn parse_pbism_returns_none_for_non_json() {
    assert!(parse_pbism("not json").is_none());
}

/// S-PBM-04: Whitespace-only content is rejected.
#[test]
fn parse_pbism_returns_none_for_empty_input() {
    assert!(parse_pbism("").is_none());
    assert!(parse_pbism("   ").is_none());
}

/// S-PBM-05: An object whose `version` is not a string is rejected.
#[test]
fn parse_pbism_returns_none_for_non_string_version() {
    let content = r#"{ "version": 4.0 }"#;
    assert!(
        parse_pbism(content).is_none(),
        "non-string version field should not be coerced"
    );
}
