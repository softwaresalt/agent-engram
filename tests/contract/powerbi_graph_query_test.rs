//! Contract tests for Power BI graph schema and `CozoDB` CRUD operations (`061.005-T`).
//!
//! Tests: schema bootstrap includes `powerbi` tables, upsert nodes/edges,
//! query nodes back, deletion sweep, and `query_graph` traversal returns
//! Power BI paths.

#[cfg(feature = "cozo-backend")]
mod powerbi_graph_query_tests {
    use chrono::Utc;
    use tempfile::TempDir;

    use engram::db::connect_db;
    use engram::db::cozo_backend::schema;
    use engram::db::queries::CodeGraphQueries;
    use engram::models::TraversalDirection;
    use engram::models::powerbi_graph::{
        PowerBiEdge, PowerBiEdgeType, PowerBiNode, PowerBiNodeKind,
    };

    async fn make_db() -> (TempDir, engram::db::Db) {
        let tmp = TempDir::new().expect("tempdir");
        let db = connect_db(tmp.path(), "test-powerbi-branch")
            .await
            .expect("connect_db");
        (tmp, db)
    }

    fn report_node(id: &str, name: &str, path: &str) -> PowerBiNode {
        PowerBiNode {
            id: id.to_string(),
            name: name.to_string(),
            kind: PowerBiNodeKind::Report,
            file_path: path.to_string(),
            source_path: "reports".to_string(),
            content_hash: "aaa".to_string(),
            ingested_at: Utc::now(),
        }
    }

    fn page_node(id: &str, name: &str, path: &str) -> PowerBiNode {
        PowerBiNode {
            id: id.to_string(),
            name: name.to_string(),
            kind: PowerBiNodeKind::Page,
            file_path: path.to_string(),
            source_path: "reports".to_string(),
            content_hash: "bbb".to_string(),
            ingested_at: Utc::now(),
        }
    }

    fn visual_node(id: &str, name: &str, path: &str) -> PowerBiNode {
        PowerBiNode {
            id: id.to_string(),
            name: name.to_string(),
            kind: PowerBiNodeKind::Visual,
            file_path: path.to_string(),
            source_path: "reports".to_string(),
            content_hash: "ccc".to_string(),
            ingested_at: Utc::now(),
        }
    }

    /// S-PBGQ-01: Schema constants for powerbi relations are non-empty.
    #[tokio::test]
    async fn powerbi_schema_constants_populated() {
        assert!(
            !schema::CREATE_POWERBI_NODE.is_empty(),
            "CREATE_POWERBI_NODE must be non-empty"
        );
        assert!(
            !schema::CREATE_POWERBI_EDGE.is_empty(),
            "CREATE_POWERBI_EDGE must be non-empty"
        );
    }

    /// S-PBGQ-02: Upsert Power BI nodes and query them back.
    #[tokio::test]
    async fn upsert_and_select_powerbi_nodes() {
        let (_tmp, db) = make_db().await;
        let q = CodeGraphQueries::new(db);

        let nodes = vec![
            report_node("rpt:001", "Sales Report", "Sales.Report/report.json"),
            page_node(
                "pg:001",
                "Overview",
                "Sales.Report/definition/pages/Overview/page.json",
            ),
        ];
        q.upsert_powerbi_nodes(&nodes)
            .await
            .expect("upsert_powerbi_nodes must succeed");

        let results = q
            .select_powerbi_nodes(None)
            .await
            .expect("select_powerbi_nodes must succeed");
        assert_eq!(results.len(), 2, "both nodes should be retrievable");
    }

    /// S-PBGQ-03: Upsert Power BI edges succeeds.
    #[tokio::test]
    async fn upsert_powerbi_edges() {
        let (_tmp, db) = make_db().await;
        let q = CodeGraphQueries::new(db);

        let nodes = vec![
            report_node("rpt:002", "Revenue Report", "Revenue.Report/report.json"),
            page_node(
                "pg:002",
                "Summary",
                "Revenue.Report/pages/Summary/page.json",
            ),
        ];
        q.upsert_powerbi_nodes(&nodes).await.expect("upsert nodes");

        let edges = vec![PowerBiEdge {
            from_id: "rpt:002".to_string(),
            to_id: "pg:002".to_string(),
            edge_type: PowerBiEdgeType::Contains,
            source_path: "reports".to_string(),
        }];
        q.upsert_powerbi_edges(&edges)
            .await
            .expect("upsert_powerbi_edges must succeed");
    }

    /// S-PBGQ-04: Source-scoped selection returns only nodes for that source.
    #[tokio::test]
    async fn select_powerbi_nodes_by_source() {
        let (_tmp, db) = make_db().await;
        let q = CodeGraphQueries::new(db);

        let n1 = PowerBiNode {
            id: "rpt:src-a".to_string(),
            name: "Report A".to_string(),
            kind: PowerBiNodeKind::Report,
            file_path: "a/report.json".to_string(),
            source_path: "source-a".to_string(),
            content_hash: "h1".to_string(),
            ingested_at: Utc::now(),
        };
        let n2 = PowerBiNode {
            id: "rpt:src-b".to_string(),
            name: "Report B".to_string(),
            kind: PowerBiNodeKind::Report,
            file_path: "b/report.json".to_string(),
            source_path: "source-b".to_string(),
            content_hash: "h2".to_string(),
            ingested_at: Utc::now(),
        };
        q.upsert_powerbi_nodes(&[n1, n2]).await.expect("upsert");

        let a_nodes = q
            .select_powerbi_nodes(Some("source-a"))
            .await
            .expect("select by source");
        assert_eq!(a_nodes.len(), 1, "only source-a node expected");
        assert_eq!(a_nodes[0].id, "rpt:src-a");
    }

    /// S-PBGQ-05: Delete Power BI nodes by source removes all associated entries.
    #[tokio::test]
    async fn delete_powerbi_nodes_by_source_removes_all() {
        let (_tmp, db) = make_db().await;
        let q = CodeGraphQueries::new(db);

        let nodes = vec![
            PowerBiNode {
                id: "rpt:keep".to_string(),
                name: "Keep".to_string(),
                kind: PowerBiNodeKind::Report,
                file_path: "keep/report.json".to_string(),
                source_path: "keep-src".to_string(),
                content_hash: "k".to_string(),
                ingested_at: Utc::now(),
            },
            PowerBiNode {
                id: "rpt:del".to_string(),
                name: "Delete".to_string(),
                kind: PowerBiNodeKind::Report,
                file_path: "del/report.json".to_string(),
                source_path: "del-src".to_string(),
                content_hash: "d".to_string(),
                ingested_at: Utc::now(),
            },
        ];
        q.upsert_powerbi_nodes(&nodes).await.expect("upsert");

        let edges = vec![PowerBiEdge {
            from_id: "rpt:del".to_string(),
            to_id: "pg:del".to_string(),
            edge_type: PowerBiEdgeType::Contains,
            source_path: "del-src".to_string(),
        }];
        q.upsert_powerbi_edges(&edges).await.expect("upsert edges");

        q.delete_powerbi_nodes_by_source("del-src")
            .await
            .expect("delete_powerbi_nodes_by_source");

        let remaining = q
            .select_powerbi_nodes(None)
            .await
            .expect("select after delete");
        assert_eq!(remaining.len(), 1, "only keep-src node should remain");
        assert_eq!(remaining[0].id, "rpt:keep");
    }

    /// S-PBGQ-06: `query_graph_neighborhood` traverses Power BI contains edges.
    #[tokio::test]
    async fn query_graph_traverses_powerbi_contains_edges() {
        let (_tmp, db) = make_db().await;
        let q = CodeGraphQueries::new(db);

        // report -> page -> visual
        let nodes = vec![
            report_node("rpt:t1", "Test Report", "Test.Report/report.json"),
            page_node("pg:t1", "Main Page", "Test.Report/pages/Main/page.json"),
            visual_node(
                "vis:t1",
                "Bar Chart",
                "Test.Report/pages/Main/visuals/v1/visual.json",
            ),
        ];
        q.upsert_powerbi_nodes(&nodes).await.expect("upsert nodes");

        let edges = vec![
            PowerBiEdge {
                from_id: "rpt:t1".to_string(),
                to_id: "pg:t1".to_string(),
                edge_type: PowerBiEdgeType::Contains,
                source_path: "reports".to_string(),
            },
            PowerBiEdge {
                from_id: "pg:t1".to_string(),
                to_id: "vis:t1".to_string(),
                edge_type: PowerBiEdgeType::Contains,
                source_path: "reports".to_string(),
            },
        ];
        q.upsert_powerbi_edges(&edges).await.expect("upsert edges");

        let result = q
            .query_graph_neighborhood(
                "rpt:t1",
                TraversalDirection::Outgoing,
                3,
                50,
                &["pbi_contains"],
            )
            .await
            .expect("query_graph_neighborhood must succeed");

        let node_ids: Vec<&str> = result.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(
            node_ids.contains(&"pg:t1"),
            "page should be reachable from report"
        );
        assert!(
            node_ids.contains(&"vis:t1"),
            "visual should be reachable from report via page"
        );
    }

    /// S-PBGQ-07: `query_graph_neighborhood` traverses Power BI `relates_to_table` edges.
    #[tokio::test]
    async fn query_graph_traverses_powerbi_relates_to_table_edges() {
        let (_tmp, db) = make_db().await;
        let q = CodeGraphQueries::new(db);

        let nodes = vec![
            PowerBiNode {
                id: "tbl:orders".to_string(),
                name: "Orders".to_string(),
                kind: PowerBiNodeKind::Table,
                file_path: "Sales.SemanticModel/model.bim".to_string(),
                source_path: "models".to_string(),
                content_hash: "horders".to_string(),
                ingested_at: Utc::now(),
            },
            PowerBiNode {
                id: "tbl:customers".to_string(),
                name: "Customers".to_string(),
                kind: PowerBiNodeKind::Table,
                file_path: "Sales.SemanticModel/model.bim".to_string(),
                source_path: "models".to_string(),
                content_hash: "hcust".to_string(),
                ingested_at: Utc::now(),
            },
        ];
        q.upsert_powerbi_nodes(&nodes).await.expect("upsert nodes");

        let edges = vec![PowerBiEdge {
            from_id: "tbl:orders".to_string(),
            to_id: "tbl:customers".to_string(),
            edge_type: PowerBiEdgeType::RelatesToTable,
            source_path: "models".to_string(),
        }];
        q.upsert_powerbi_edges(&edges).await.expect("upsert edges");

        let result = q
            .query_graph_neighborhood(
                "tbl:orders",
                TraversalDirection::Outgoing,
                2,
                50,
                &["pbi_relates_to_table"],
            )
            .await
            .expect("query_graph_neighborhood must succeed");

        let node_ids: Vec<&str> = result.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(
            node_ids.contains(&"tbl:customers"),
            "customers table should be reachable from orders via relates_to_table"
        );
    }
}
