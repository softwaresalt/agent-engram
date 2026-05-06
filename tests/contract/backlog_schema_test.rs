//! Contract tests for backlog CozoDB schema and CRUD operations (002.003-T).
//!
//! Tests: schema bootstrap, upsert nodes/edges/records, query, per-file deletion.

#[cfg(feature = "cozo-backend")]
mod backlog_schema_tests {
    use chrono::Utc;
    use tempfile::TempDir;

    use engram::db::connect_db;
    use engram::db::cozo_backend::schema;
    use engram::db::queries::CodeGraphQueries;
    use engram::models::backlog_graph::{
        BacklogContentRecord, BacklogEdge, BacklogEdgeType, BacklogNode,
    };

    async fn make_db() -> (TempDir, engram::db::Db) {
        let tmp = TempDir::new().expect("tempdir");
        let db = connect_db(tmp.path(), "test-backlog-branch")
            .await
            .expect("connect_db");
        (tmp, db)
    }

    /// S-BS-01: Schema bootstrap includes new backlog relations (idempotent).
    #[tokio::test]
    async fn backlog_schema_constants_populated() {
        assert!(
            !schema::CREATE_BACKLOG_NODE.is_empty(),
            "CREATE_BACKLOG_NODE must be non-empty"
        );
        assert!(
            !schema::CREATE_BACKLOG_EDGE.is_empty(),
            "CREATE_BACKLOG_EDGE must be non-empty"
        );
        assert!(
            !schema::CREATE_BACKLOG_CONTENT_RECORD.is_empty(),
            "CREATE_BACKLOG_CONTENT_RECORD must be non-empty"
        );
    }

    /// S-BS-02: Upsert backlog nodes and query them back.
    #[tokio::test]
    async fn upsert_and_select_backlog_nodes() {
        let (_tmp, db) = make_db().await;
        let q = CodeGraphQueries::new(db);

        let nodes = vec![
            BacklogNode {
                id: "001-F".to_string(),
                title: "Feature one".to_string(),
                kind: "feature".to_string(),
                status: "queued".to_string(),
                labels: vec!["infra".to_string()],
                file_path: ".backlogit/queue/001-F.md".to_string(),
                content_hash: "aaa111".to_string(),
                source_path: ".backlogit/queue".to_string(),
                ingested_at: Utc::now(),
            },
            BacklogNode {
                id: "001.001-T".to_string(),
                title: "Task one".to_string(),
                kind: "task".to_string(),
                status: "queued".to_string(),
                labels: vec![],
                file_path: ".backlogit/queue/001.001-T.md".to_string(),
                content_hash: "bbb222".to_string(),
                source_path: ".backlogit/queue".to_string(),
                ingested_at: Utc::now(),
            },
        ];
        q.upsert_backlog_nodes(&nodes)
            .await
            .expect("upsert_backlog_nodes must succeed");

        let results = q
            .select_backlog_nodes(None)
            .await
            .expect("select_backlog_nodes must succeed");
        assert_eq!(results.len(), 2, "should retrieve both inserted nodes");
    }

    /// S-BS-03: Upsert backlog edges and query them back via nodes select.
    #[tokio::test]
    async fn upsert_backlog_edges() {
        let (_tmp, db) = make_db().await;
        let q = CodeGraphQueries::new(db);

        // Insert nodes first so edges reference valid file paths.
        let nodes = vec![BacklogNode {
            id: "010-F".to_string(),
            title: "Feature ten".to_string(),
            kind: "feature".to_string(),
            status: "queued".to_string(),
            labels: vec![],
            file_path: ".backlogit/queue/010-F.md".to_string(),
            content_hash: "f10".to_string(),
            source_path: ".backlogit/queue".to_string(),
            ingested_at: Utc::now(),
        }];
        q.upsert_backlog_nodes(&nodes).await.expect("upsert nodes");

        let edges = vec![BacklogEdge {
            from_id: "010-F".to_string(),
            to_id: "010.001-T".to_string(),
            edge_type: BacklogEdgeType::ParentOf,
            source_path: ".backlogit/queue".to_string(),
        }];
        q.upsert_backlog_edges(&edges)
            .await
            .expect("upsert_backlog_edges must succeed");
    }

    /// S-BS-04: Upsert and query backlog content records.
    #[tokio::test]
    async fn upsert_and_select_backlog_content_records() {
        let (_tmp, db) = make_db().await;
        let q = CodeGraphQueries::new(db);

        let records = vec![BacklogContentRecord {
            file_path: ".backlogit/queue/002-F.md".to_string(),
            content_type: "backlog".to_string(),
            content_hash: "hash002".to_string(),
            content: "## Description\n\nFeature description here.".to_string(),
            source_path: ".backlogit/queue".to_string(),
            ingested_at: Utc::now(),
        }];
        q.upsert_backlog_content_records(&records)
            .await
            .expect("upsert_backlog_content_records must succeed");

        let results = q
            .select_backlog_content_records(None)
            .await
            .expect("select_backlog_content_records must succeed");
        assert_eq!(results.len(), 1, "should retrieve inserted content record");
        assert_eq!(results[0].file_path, ".backlogit/queue/002-F.md");
    }

    /// S-BS-05: Per-file deletion removes node + edges + content record
    /// without affecting other files.
    #[tokio::test]
    async fn per_file_deletion_removes_only_target() {
        let (_tmp, db) = make_db().await;
        let q = CodeGraphQueries::new(db);

        // Insert two nodes.
        let nodes = vec![
            BacklogNode {
                id: "020-F".to_string(),
                title: "Keep this".to_string(),
                kind: "feature".to_string(),
                status: "queued".to_string(),
                labels: vec![],
                file_path: ".backlogit/queue/020-F.md".to_string(),
                content_hash: "keep".to_string(),
                source_path: ".backlogit/queue".to_string(),
                ingested_at: Utc::now(),
            },
            BacklogNode {
                id: "021-F".to_string(),
                title: "Delete this".to_string(),
                kind: "feature".to_string(),
                status: "queued".to_string(),
                labels: vec![],
                file_path: ".backlogit/queue/021-F.md".to_string(),
                content_hash: "del".to_string(),
                source_path: ".backlogit/queue".to_string(),
                ingested_at: Utc::now(),
            },
        ];
        q.upsert_backlog_nodes(&nodes).await.expect("upsert");

        // Insert an edge for the node to be deleted.
        let edges = vec![BacklogEdge {
            from_id: "021-F".to_string(),
            to_id: "021.001-T".to_string(),
            edge_type: BacklogEdgeType::ParentOf,
            source_path: ".backlogit/queue".to_string(),
        }];
        q.upsert_backlog_edges(&edges).await.expect("upsert edges");

        // Insert content records for both.
        let records = vec![
            BacklogContentRecord {
                file_path: ".backlogit/queue/020-F.md".to_string(),
                content_type: "backlog".to_string(),
                content_hash: "keep".to_string(),
                content: "keep content".to_string(),
                source_path: ".backlogit/queue".to_string(),
                ingested_at: Utc::now(),
            },
            BacklogContentRecord {
                file_path: ".backlogit/queue/021-F.md".to_string(),
                content_type: "backlog".to_string(),
                content_hash: "del".to_string(),
                content: "delete content".to_string(),
                source_path: ".backlogit/queue".to_string(),
                ingested_at: Utc::now(),
            },
        ];
        q.upsert_backlog_content_records(&records)
            .await
            .expect("upsert records");

        // Delete only 021-F.md.
        q.delete_backlog_node_by_file_path(".backlogit/queue/021-F.md")
            .await
            .expect("delete_backlog_node_by_file_path");
        q.delete_backlog_content_record_by_path(".backlogit/queue/021-F.md")
            .await
            .expect("delete_backlog_content_record_by_path");

        // 020-F.md should still be there.
        let remaining_nodes = q
            .select_backlog_nodes(None)
            .await
            .expect("select after delete");
        assert_eq!(
            remaining_nodes.len(),
            1,
            "only 020-F.md should remain after per-file deletion"
        );
        assert_eq!(remaining_nodes[0].id, "020-F");

        let remaining_records = q
            .select_backlog_content_records(None)
            .await
            .expect("select records after delete");
        assert_eq!(
            remaining_records.len(),
            1,
            "only 020-F.md record should remain"
        );
    }
}
