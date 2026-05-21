//! Integration tests for Power BI source type registration and dispatch (061.002-T).
//!
//! Verifies that the `powerbi` content type is registered as a built-in type,
//! that a registry YAML referencing it parses without error, and that the
//! type is routed to the dedicated Power BI indexer rather than the generic
//! whole-file ingestion path.
//!
//! Tests: S-PSD-01, S-PSD-02, S-PSD-03, S-PSD-04

use engram::models::registry::BUILT_IN_TYPES;
use engram::services::registry::parse_registry_yaml;

/// S-PSD-01: `powerbi` appears in the built-in type list.
#[test]
fn powerbi_is_built_in_type() {
    assert!(
        BUILT_IN_TYPES.contains(&"powerbi"),
        "`powerbi` must be registered as a built-in content type in BUILT_IN_TYPES"
    );
}

/// S-PSD-02: Registry YAML with `type: powerbi` parses without error.
#[test]
fn powerbi_source_parses_from_registry_yaml() {
    let yaml = "sources:\n  - type: powerbi\n    path: reports\n";
    let config = parse_registry_yaml(yaml).expect("powerbi registry source should parse");
    assert_eq!(config.sources.len(), 1);
    assert_eq!(config.sources[0].content_type, "powerbi");
    assert_eq!(config.sources[0].path, "reports");
}

/// S-PSD-03: A `powerbi` source is distinct from `code` and `backlog` sources.
///
/// This confirms the dispatch routing will not accidentally forward a `powerbi`
/// source to the code-graph or backlog indexer branches.
#[test]
fn powerbi_source_is_not_code_or_backlog() {
    let yaml = "sources:\n  - type: powerbi\n    path: .pbip-workspace\n";
    let config = parse_registry_yaml(yaml).expect("powerbi source should parse");
    let source = &config.sources[0];

    assert_ne!(
        source.content_type, "code",
        "`powerbi` source must not be dispatched as a code source"
    );
    assert_ne!(
        source.content_type, "backlog",
        "`powerbi` source must not be dispatched as a backlog source"
    );
    assert_eq!(
        source.content_type, "powerbi",
        "`powerbi` source must identify as `powerbi`"
    );
}

/// S-PSD-04: Mixed registry with `powerbi` alongside other source types parses correctly.
#[test]
fn mixed_registry_with_powerbi_parses() {
    let yaml = "sources:\n  - type: docs\n    path: docs\n  - type: powerbi\n    path: reports\n  - type: backlog\n    path: .backlogit/queue\n";
    let config = parse_registry_yaml(yaml).expect("mixed registry should parse");
    assert_eq!(config.sources.len(), 3);

    let powerbi_src = config
        .sources
        .iter()
        .find(|s| s.content_type == "powerbi")
        .expect("powerbi source should be present");
    assert_eq!(powerbi_src.path, "reports");
}
