//! Integration tests for the lineage read surface (095-F, Unit U8,
//! task 095.010-T).
//!
//! Seeds a `dataset_node` + `lineage_derives_from` subgraph via the U1a′
//! writers and exercises the `query_graph` traversal surface: an outgoing
//! neighborhood / `find_path` from a write TARGET reaches its upstream SOURCES,
//! an incoming neighborhood from a SOURCE reaches its downstream CONSUMERS
//! (AR-06), and a code-only edge filter never traverses lineage (no regression).

#[cfg(feature = "cozo-backend")]
use engram::db::{connect_db, queries::CodeGraphQueries};
#[cfg(feature = "cozo-backend")]
use engram::models::TraversalDirection;
#[cfg(feature = "cozo-backend")]
use engram::models::lineage::{DatasetKind, LineageEndpoint};

/// Build a table endpoint with an authority-embedded id.
#[cfg(feature = "cozo-backend")]
fn endpoint(id: &str, name: &str) -> LineageEndpoint {
    LineageEndpoint {
        id: id.to_owned(),
        name: name.to_owned(),
        kind: DatasetKind::Table,
    }
}

/// Seed `summary` (target) derives-from `orders` (source): edge
/// `from_id=summary`, `to_id=orders` (AR-05). Returns `(summary_id, orders_id)`.
#[cfg(feature = "cozo-backend")]
async fn seed(queries: &CodeGraphQueries) -> (String, String) {
    let summary = endpoint(
        "table::metastore-prod::main.sales.summary",
        "main.sales.summary",
    );
    let orders = endpoint(
        "table::metastore-prod::main.sales.orders",
        "main.sales.orders",
    );
    queries
        .upsert_dataset_nodes(&[summary.clone(), orders.clone()])
        .await
        .expect("seed dataset nodes");
    queries
        .upsert_lineage_edges(&[(summary.id.clone(), orders.id.clone())])
        .await
        .expect("seed lineage edge");
    (summary.id, orders.id)
}

#[cfg(feature = "cozo-backend")]
async fn open(label: &str) -> (tempfile::TempDir, CodeGraphQueries) {
    let root = tempfile::TempDir::new().expect("tempdir");
    let db = connect_db(&root.path().join("data"), label)
        .await
        .expect("connect_db");
    (root, CodeGraphQueries::new(db))
}

/// U8-1 (AR-06): an OUTGOING neighborhood from a write target reaches its
/// upstream source dataset, projecting the `dataset_node` kind and the
/// `lineage_derives_from` edge.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn outgoing_neighborhood_from_target_reaches_upstream_source() {
    let (_root, queries) = open("lineage-out").await;
    let (summary, orders) = seed(&queries).await;

    let result = queries
        .query_graph_neighborhood(&summary, TraversalDirection::Outgoing, 3, 50, &[])
        .await
        .expect("neighborhood");

    let orders_node = result
        .nodes
        .iter()
        .find(|n| n.id == orders)
        .expect("upstream source node is reached");
    assert_eq!(
        orders_node.kind, "dataset_table",
        "dataset_node kind projected"
    );
    assert_eq!(orders_node.name, "main.sales.orders");
    assert!(
        result.edges.iter().any(|e| {
            e.edge_type == "lineage_derives_from" && e.from == summary && e.to == orders
        }),
        "the derives edge is oriented target→source"
    );
}

/// U8-2 (AR-06): an INCOMING neighborhood from a source dataset reaches the
/// downstream consumer(s) that derive from it.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn incoming_neighborhood_from_source_reaches_downstream_consumer() {
    let (_root, queries) = open("lineage-in").await;
    let (summary, orders) = seed(&queries).await;

    let result = queries
        .query_graph_neighborhood(&orders, TraversalDirection::Incoming, 3, 50, &[])
        .await
        .expect("neighborhood");

    assert!(
        result.nodes.iter().any(|n| n.id == summary),
        "an incoming traversal from the source reaches its downstream consumer (AR-06)"
    );
    assert!(
        result.edges.iter().any(|e| {
            e.edge_type == "lineage_derives_from" && e.from == summary && e.to == orders
        }),
        "the derives edge is reported"
    );
}

/// U8-3: `find_path` (outgoing) from a target to its source finds the derives
/// path.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn find_path_from_target_traverses_to_source() {
    let (_root, queries) = open("lineage-path").await;
    let (summary, orders) = seed(&queries).await;

    let fp = queries
        .find_path(&summary, &orders, 3, &[])
        .await
        .expect("find_path");
    assert!(fp.found, "a derives path exists from target to source");
    assert_eq!(fp.path, vec![summary, orders]);
}

/// U8-4 (no regression): a code-only edge filter never traverses lineage edges,
/// so existing code/backlog traversals are unaffected by the lineage branch.
#[cfg(feature = "cozo-backend")]
#[tokio::test]
async fn code_only_edge_filter_excludes_lineage() {
    let (_root, queries) = open("lineage-filter").await;
    let (summary, _orders) = seed(&queries).await;

    let result = queries
        .query_graph_neighborhood(&summary, TraversalDirection::Outgoing, 3, 50, &["calls"])
        .await
        .expect("neighborhood");
    assert!(
        result.nodes.is_empty() && result.edges.is_empty(),
        "a code-only edge filter must not pull in lineage edges"
    );

    // An explicit lineage filter DOES traverse the derives edge.
    let scoped = queries
        .query_graph_neighborhood(
            &summary,
            TraversalDirection::Outgoing,
            3,
            50,
            &["lineage_derives_from"],
        )
        .await
        .expect("scoped neighborhood");
    assert!(
        scoped
            .nodes
            .iter()
            .any(|n| n.id == "table::metastore-prod::main.sales.orders"),
        "an explicit lineage filter reaches the source"
    );
}
