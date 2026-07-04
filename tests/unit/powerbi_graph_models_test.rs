//! Unit tests for Power BI graph data models (061.001-T).
//!
//! Tests: struct construction, serde roundtrip, node kind and edge type
//! string stability, and index-result defaults.

use engram::models::powerbi_graph::{
    PowerBiEdge, PowerBiEdgeType, PowerBiGraphIndexResult, PowerBiNode, PowerBiNodeKind,
};

/// S-PBG-01: `PowerBiNodeKind` string values are stable.
#[test]
fn powerbi_node_kind_as_str_stable() {
    assert_eq!(PowerBiNodeKind::Report.as_str(), "report");
    assert_eq!(PowerBiNodeKind::Page.as_str(), "page");
    assert_eq!(PowerBiNodeKind::Visual.as_str(), "visual");
    assert_eq!(PowerBiNodeKind::SemanticModel.as_str(), "semantic_model");
    assert_eq!(PowerBiNodeKind::Table.as_str(), "table");
    assert_eq!(PowerBiNodeKind::Column.as_str(), "column");
    assert_eq!(PowerBiNodeKind::Measure.as_str(), "measure");
    assert_eq!(PowerBiNodeKind::Expression.as_str(), "expression");
    assert_eq!(PowerBiNodeKind::Relationship.as_str(), "relationship");
    assert_eq!(PowerBiNodeKind::DataSource.as_str(), "data_source");
    assert_eq!(PowerBiNodeKind::Partition.as_str(), "partition");
}

/// S-PBG-02: `PowerBiNode` constructs with expected field values.
#[test]
fn powerbi_node_construct() {
    let node = PowerBiNode {
        id: "pbi:rpt:abc123".to_string(),
        name: "Sales Report".to_string(),
        kind: PowerBiNodeKind::Report,
        file_path: "Sales.Report/report.json".to_string(),
        source_path: "src/powerbi".to_string(),
        content_hash: "deadbeef".to_string(),
        ingested_at: chrono::Utc::now(),
    };
    assert_eq!(node.id, "pbi:rpt:abc123");
    assert_eq!(node.name, "Sales Report");
    assert_eq!(node.kind, PowerBiNodeKind::Report);
    assert_eq!(node.kind.as_str(), "report");
}

/// S-PBG-03: `PowerBiNode` serializes and deserializes correctly.
#[test]
fn powerbi_node_serde_roundtrip() {
    let node = PowerBiNode {
        id: "pbi:tbl:def456".to_string(),
        name: "Customers".to_string(),
        kind: PowerBiNodeKind::Table,
        file_path: "Sales.SemanticModel/model.bim".to_string(),
        source_path: "src/powerbi".to_string(),
        content_hash: "cafebabe".to_string(),
        ingested_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string(&node).expect("should serialize");
    let roundtripped: PowerBiNode = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(roundtripped.id, node.id);
    assert_eq!(roundtripped.name, node.name);
    assert_eq!(roundtripped.kind, PowerBiNodeKind::Table);
}

/// S-PBG-04: `PowerBiEdgeType` string values are stable and use `pbi_` prefix.
#[test]
fn powerbi_edge_type_as_str_stable() {
    assert_eq!(PowerBiEdgeType::Contains.as_str(), "pbi_contains");
    assert_eq!(PowerBiEdgeType::UsesField.as_str(), "pbi_uses_field");
    assert_eq!(
        PowerBiEdgeType::DependsOnModel.as_str(),
        "pbi_depends_on_model"
    );
    assert_eq!(
        PowerBiEdgeType::BelongsToReport.as_str(),
        "pbi_belongs_to_report"
    );
    assert_eq!(
        PowerBiEdgeType::RelatesToTable.as_str(),
        "pbi_relates_to_table"
    );
}

/// S-PBG-05: `PowerBiEdge` constructs with correct fields.
#[test]
fn powerbi_edge_construct() {
    let edge = PowerBiEdge {
        from_id: "pbi:rpt:abc123".to_string(),
        to_id: "pbi:pg:page1".to_string(),
        edge_type: PowerBiEdgeType::Contains,
        source_path: "src/powerbi".to_string(),
    };
    assert_eq!(edge.from_id, "pbi:rpt:abc123");
    assert_eq!(edge.to_id, "pbi:pg:page1");
    assert_eq!(edge.edge_type.as_str(), "pbi_contains");
}

/// S-PBG-06: `PowerBiEdge` serializes and deserializes correctly.
#[test]
fn powerbi_edge_serde_roundtrip() {
    let edge = PowerBiEdge {
        from_id: "pbi:tbl:t1".to_string(),
        to_id: "pbi:tbl:t2".to_string(),
        edge_type: PowerBiEdgeType::RelatesToTable,
        source_path: "data/sales".to_string(),
    };
    let json = serde_json::to_string(&edge).expect("should serialize");
    let roundtripped: PowerBiEdge = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(roundtripped.from_id, edge.from_id);
    assert_eq!(roundtripped.edge_type, PowerBiEdgeType::RelatesToTable);
}

/// S-PBG-07: `PowerBiGraphIndexResult` tracks node and edge counts from default.
#[test]
fn powerbi_graph_index_result_default() {
    let result = PowerBiGraphIndexResult::default();
    assert_eq!(result.nodes_written, 0);
    assert_eq!(result.edges_written, 0);
    assert_eq!(result.nodes_removed, 0);
}

/// S-PBG-08: `PowerBiNodeKind::Display` output matches `as_str`.
#[test]
fn powerbi_node_kind_display_matches_as_str() {
    for kind in [
        PowerBiNodeKind::Report,
        PowerBiNodeKind::Page,
        PowerBiNodeKind::Visual,
        PowerBiNodeKind::SemanticModel,
        PowerBiNodeKind::Table,
        PowerBiNodeKind::Column,
        PowerBiNodeKind::Measure,
        PowerBiNodeKind::Expression,
        PowerBiNodeKind::Relationship,
        PowerBiNodeKind::DataSource,
        PowerBiNodeKind::Partition,
    ] {
        assert_eq!(kind.to_string(), kind.as_str());
    }
}

/// S-PBG-09: `PowerBiEdgeType::Display` output matches `as_str`.
#[test]
fn powerbi_edge_type_display_matches_as_str() {
    for et in [
        PowerBiEdgeType::Contains,
        PowerBiEdgeType::UsesField,
        PowerBiEdgeType::DependsOnModel,
        PowerBiEdgeType::BelongsToReport,
        PowerBiEdgeType::RelatesToTable,
    ] {
        assert_eq!(et.to_string(), et.as_str());
    }
}

/// S-PBG-10: Workspace-relative file path is preserved through serde.
#[test]
fn powerbi_node_file_path_preserved() {
    let node = PowerBiNode {
        id: "pbi:pg:xyz".to_string(),
        name: "Overview".to_string(),
        kind: PowerBiNodeKind::Page,
        file_path: "Reports/Sales.Report/definition/pages/ReportSection/page.json".to_string(),
        source_path: "Reports".to_string(),
        content_hash: "abc".to_string(),
        ingested_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string(&node).unwrap();
    let rt: PowerBiNode = serde_json::from_str(&json).unwrap();
    assert_eq!(rt.file_path, node.file_path);
    assert_eq!(rt.source_path, node.source_path);
}
