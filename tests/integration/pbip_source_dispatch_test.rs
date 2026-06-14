//! Integration tests for `pbip` source type registration and dispatch (062.001-T).
//!
//! Verifies that the `pbip` content type is registered as a built-in type,
//! that a registry YAML referencing it parses without error, and that the
//! type is kept distinct from `powerbi`, code-graph, and other dedicated
//! ingestion paths.
//!
//! Tests: S-PSD-01, S-PSD-02, S-PSD-03, S-PSD-04, S-PSD-05

use engram::models::registry::BUILT_IN_TYPES;
use engram::services::registry::parse_registry_yaml;

/// S-PSD-01: `pbip` appears in the built-in type list.
#[test]
fn pbip_is_built_in_type() {
    assert!(
        BUILT_IN_TYPES.contains(&"pbip"),
        "`pbip` must be registered as a built-in content type in BUILT_IN_TYPES"
    );
}

/// S-PSD-02: Registry YAML with `type: pbip` parses without error.
#[test]
fn pbip_source_parses_from_registry_yaml() {
    let yaml = "sources:\n  - type: pbip\n    path: pbip\n";
    let config = parse_registry_yaml(yaml).expect("pbip registry source should parse");
    assert_eq!(config.sources.len(), 1);
    assert_eq!(config.sources[0].content_type, "pbip");
    assert_eq!(config.sources[0].path, "pbip");
}

/// S-PSD-03: A `pbip` source is distinct from `powerbi`, code, backlog, and notebook sources.
#[test]
fn pbip_source_is_not_other_dedicated_types() {
    let yaml = "sources:\n  - type: pbip\n    path: reports\n";
    let config = parse_registry_yaml(yaml).expect("pbip source should parse");
    let source = &config.sources[0];

    assert_ne!(
        source.content_type, "powerbi",
        "`pbip` source must not be dispatched as a legacy Power BI source"
    );
    assert_ne!(
        source.content_type, "code",
        "`pbip` source must not be dispatched as a code source"
    );
    assert_ne!(
        source.content_type, "backlog",
        "`pbip` source must not be dispatched as a backlog source"
    );
    assert_ne!(
        source.content_type, "notebook",
        "`pbip` source must not be dispatched as a notebook source"
    );
    assert_eq!(
        source.content_type, "pbip",
        "`pbip` source must identify as `pbip`"
    );
}

/// S-PSD-04: Mixed registry with `pbip` alongside `powerbi` and other source types parses correctly.
///
/// This guards the spike conclusion that `pbip` is a new dedicated type that
/// must coexist with the legacy `powerbi` source rather than replace it.
#[test]
fn mixed_registry_with_pbip_and_powerbi_parses() {
    let yaml = "sources:\n  - type: docs\n    path: docs\n  - type: powerbi\n    path: legacy/powerbi\n  - type: pbip\n    path: pbip\n  - type: backlog\n    path: .backlogit/queue\n";
    let config = parse_registry_yaml(yaml).expect("mixed registry should parse");
    assert_eq!(config.sources.len(), 4);

    let pbip_src = config
        .sources
        .iter()
        .find(|source| source.content_type == "pbip")
        .expect("pbip source should be present");
    assert_eq!(pbip_src.path, "pbip");

    let powerbi_src = config
        .sources
        .iter()
        .find(|source| source.content_type == "powerbi")
        .expect("legacy powerbi source should still be present");
    assert_eq!(powerbi_src.path, "legacy/powerbi");
}

/// S-PSD-05: Legacy `powerbi` source type continues to be recognized as a built-in.
///
/// Guards the spike conclusion that the new `pbip` type does not displace
/// `powerbi`; the legacy JSON/BIM path must remain available.
#[test]
fn legacy_powerbi_is_still_built_in() {
    assert!(
        BUILT_IN_TYPES.contains(&"powerbi"),
        "`powerbi` must remain a built-in content type alongside `pbip`"
    );
}
