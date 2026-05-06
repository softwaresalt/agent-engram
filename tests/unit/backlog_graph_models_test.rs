//! Unit tests for backlog graph data models (002.002-T).
//!
//! Tests: struct construction, serde roundtrip, edge type string representation.

use engram::models::backlog_graph::{
    BacklogContentRecord, BacklogEdge, BacklogEdgeType, BacklogIndexResult, BacklogNode,
};

/// S-BGM-01: `BacklogNode` can be constructed and has expected field values.
#[test]
fn backlog_node_construct() {
    let node = BacklogNode {
        id: "001-T".to_string(),
        title: "Test Task".to_string(),
        kind: "task".to_string(),
        status: "queued".to_string(),
        labels: vec!["alpha".to_string(), "beta".to_string()],
        file_path: ".backlogit/queue/001-T.md".to_string(),
        content_hash: "abc123".to_string(),
        source_path: ".backlogit/queue".to_string(),
        ingested_at: chrono::Utc::now(),
    };
    assert_eq!(node.id, "001-T");
    assert_eq!(node.kind, "task");
    assert_eq!(node.labels.len(), 2);
}

/// S-BGM-02: `BacklogEdge` has correct fields and `edge_type` round-trips.
#[test]
fn backlog_edge_construct_and_edge_type() {
    let edge = BacklogEdge {
        from_id: "001-F".to_string(),
        to_id: "001.001-T".to_string(),
        edge_type: BacklogEdgeType::ParentOf,
        source_path: ".backlogit/queue".to_string(),
    };
    assert_eq!(edge.edge_type.as_str(), "parent_of");
    assert_eq!(BacklogEdgeType::DependsOn.as_str(), "depends_on");
    assert_eq!(BacklogEdgeType::References.as_str(), "references");
}

/// S-BGM-03: `BacklogNode` serializes and deserializes correctly.
#[test]
fn backlog_node_serde_roundtrip() {
    let node = BacklogNode {
        id: "007-F".to_string(),
        title: "Feature seven".to_string(),
        kind: "feature".to_string(),
        status: "active".to_string(),
        labels: vec!["infra".to_string()],
        file_path: ".backlogit/queue/007-F.md".to_string(),
        content_hash: "deadbeef".to_string(),
        source_path: ".backlogit/queue".to_string(),
        ingested_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string(&node).expect("should serialize");
    let roundtripped: BacklogNode = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(roundtripped.id, node.id);
    assert_eq!(roundtripped.title, node.title);
    assert_eq!(roundtripped.labels, node.labels);
}

/// S-BGM-04: `BacklogIndexResult` aggregates nodes, edges, and records.
#[test]
fn backlog_index_result_aggregates() {
    let result = BacklogIndexResult {
        nodes: vec![],
        edges: vec![],
        records: vec![],
        ingested: 0,
        unchanged: 0,
        removed: 0,
    };
    assert_eq!(result.nodes.len(), 0);
    assert_eq!(result.edges.len(), 0);
    assert_eq!(result.records.len(), 0);
}

/// S-BGM-05: `BacklogContentRecord` constructs with expected fields.
#[test]
fn backlog_content_record_construct() {
    let record = BacklogContentRecord {
        file_path: ".backlogit/queue/001-T.md".to_string(),
        content_type: "backlog".to_string(),
        content_hash: "feedface".to_string(),
        content: "## Description\n\nSome task content.".to_string(),
        source_path: ".backlogit/queue".to_string(),
        ingested_at: chrono::Utc::now(),
    };
    assert_eq!(record.content_type, "backlog");
    assert!(record.content.contains("task content"));
}
