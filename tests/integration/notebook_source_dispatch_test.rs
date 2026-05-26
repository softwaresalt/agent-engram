//! Integration tests for notebook source type registration and dispatch (063.001-T).
//!
//! Verifies that the `notebook` content type is registered as a built-in type,
//! that a registry YAML referencing it parses without error, and that the
//! type is kept distinct from code-graph and other dedicated ingestion paths.
//!
//! Tests: S-NSD-01, S-NSD-02, S-NSD-03, S-NSD-04

use engram::models::registry::BUILT_IN_TYPES;
use engram::services::registry::parse_registry_yaml;

/// S-NSD-01: `notebook` appears in the built-in type list.
#[test]
fn notebook_is_built_in_type() {
    assert!(
        BUILT_IN_TYPES.contains(&"notebook"),
        "`notebook` must be registered as a built-in content type in BUILT_IN_TYPES"
    );
}

/// S-NSD-02: Registry YAML with `type: notebook` parses without error.
#[test]
fn notebook_source_parses_from_registry_yaml() {
    let yaml = "sources:\n  - type: notebook\n    path: notebooks\n";
    let config = parse_registry_yaml(yaml).expect("notebook registry source should parse");
    assert_eq!(config.sources.len(), 1);
    assert_eq!(config.sources[0].content_type, "notebook");
    assert_eq!(config.sources[0].path, "notebooks");
}

/// S-NSD-03: A `notebook` source is distinct from code, backlog, and Power BI sources.
#[test]
fn notebook_source_is_not_other_dedicated_types() {
    let yaml = "sources:\n  - type: notebook\n    path: analysis\n";
    let config = parse_registry_yaml(yaml).expect("notebook source should parse");
    let source = &config.sources[0];

    assert_ne!(
        source.content_type, "code",
        "`notebook` source must not be dispatched as a code source"
    );
    assert_ne!(
        source.content_type, "backlog",
        "`notebook` source must not be dispatched as a backlog source"
    );
    assert_ne!(
        source.content_type, "powerbi",
        "`notebook` source must not be dispatched as a Power BI source"
    );
    assert_eq!(
        source.content_type, "notebook",
        "`notebook` source must identify as `notebook`"
    );
}

/// S-NSD-04: Mixed registry with `notebook` alongside other source types parses correctly.
#[test]
fn mixed_registry_with_notebook_parses() {
    let yaml = "sources:\n  - type: docs\n    path: docs\n  - type: notebook\n    path: notebooks\n  - type: backlog\n    path: .backlogit/queue\n";
    let config = parse_registry_yaml(yaml).expect("mixed registry should parse");
    assert_eq!(config.sources.len(), 3);

    let notebook_src = config
        .sources
        .iter()
        .find(|source| source.content_type == "notebook")
        .expect("notebook source should be present");
    assert_eq!(notebook_src.path, "notebooks");
}
