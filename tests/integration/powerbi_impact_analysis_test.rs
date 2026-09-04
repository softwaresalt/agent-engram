//! Integration + contract tests for the Power BI span of `impact_analysis`
//! (P4, `085.004-T`).
//!
//! Covers `find_powerbi_nodes_by_name` resolution, the additive
//! `powerbi_node_id` stable root selector, the edge- and node-kind-aware
//! `powerbi_neighborhood` traversal (dependent measures + onward
//! visuals/pages/reports, EXCLUDING the root's owner table/model), the
//! `root_kind` discriminator, and disambiguation via the selector for a
//! same-model duplicate column name and a code-symbol / Power BI name
//! collision.

#![cfg(feature = "cozo-backend")]

use engram::config::StaleStrategy;
use engram::models::config::DaemonMode;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::json;
use tempfile::TempDir;

use engram::db::{connect_db, queries::CodeGraphQueries};
use engram::models::Function;
use engram::models::powerbi_graph::{PowerBiEdge, PowerBiEdgeType, PowerBiNode, PowerBiNodeKind};
use engram::models::registry::{ContentSource, ContentSourceStatus};
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::services::powerbi_indexer::index_powerbi_source;
use engram::tools;

const MAX_FILE_SIZE: u64 = 1_048_576;

const SALES_TMDL: &str = "table Sales\n\
     \x20\x20column Amount\n\
     \x20\x20\x20\x20dataType: double\n\
     \x20\x20column Key\n\
     \x20\x20\x20\x20dataType: int64\n\
     \x20\x20measure RevenueByYear = CALCULATE(SUM(Sales[Amount]), 'Date'[Year])\n";

const DATE_TMDL: &str = "table Date\n\
     \x20\x20column Year\n\
     \x20\x20\x20\x20dataType: int64\n\
     \x20\x20column Key\n\
     \x20\x20\x20\x20dataType: int64\n";

fn powerbi_source(path: &str) -> ContentSource {
    ContentSource {
        content_type: "powerbi".to_string(),
        language: None,
        path: path.to_string(),
        pattern: None,
        optional: false,
        status: ContentSourceStatus::Active,
    }
}

fn snapshot(data_dir: PathBuf, branch: &str, path: &str) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: "pbi-impact".to_string(),
        workspace_uuid: "uuid-pbi-impact".to_string(),
        branch: branch.to_string(),
        data_dir,
        path: path.to_string(),
        last_flush: None,
        stale_files: false,
        connection_count: 1,
        file_mtimes: std::collections::HashMap::new(),
    }
}

/// Materialise the two-file TMDL fixture under `models/…/tables`.
fn write_fixture(workspace: &Path) {
    let tables = workspace
        .join("models")
        .join("Sales.SemanticModel")
        .join("definition")
        .join("tables");
    std::fs::create_dir_all(&tables).expect("create tables dir");
    std::fs::write(tables.join("Sales.tmdl"), SALES_TMDL).expect("write Sales.tmdl");
    std::fs::write(tables.join("Date.tmdl"), DATE_TMDL).expect("write Date.tmdl");
}

fn find_node<'a>(
    nodes: &'a [PowerBiNode],
    name: &str,
    kind: PowerBiNodeKind,
) -> Option<&'a PowerBiNode> {
    nodes.iter().find(|n| n.name == name && n.kind == kind)
}

fn find_node_in_file<'a>(
    nodes: &'a [PowerBiNode],
    name: &str,
    kind: PowerBiNodeKind,
    file_substr: &str,
) -> Option<&'a PowerBiNode> {
    nodes
        .iter()
        .find(|n| n.name == name && n.kind == kind && n.file_path.contains(file_substr))
}

/// Inject a synthetic report → page → visual subgraph plus a
/// `visual → measure` `pbi_uses_field` edge. The report extractor does not
/// capture field references, so the visual→measure link is added directly to
/// exercise the onward-containment traversal.
async fn inject_report_subgraph(queries: &CodeGraphQueries, measure_id: &str) {
    let report = PowerBiNode {
        id: "pbi_report_sales".to_string(),
        name: "SalesReport".to_string(),
        kind: PowerBiNodeKind::Report,
        file_path: "models/report.json".to_string(),
        source_path: "models".to_string(),
        content_hash: "h".to_string(),
        ingested_at: chrono::Utc::now(),
    };
    let page = PowerBiNode {
        id: "pbi_page_overview".to_string(),
        name: "Overview".to_string(),
        kind: PowerBiNodeKind::Page,
        file_path: "models/report.json".to_string(),
        source_path: "models".to_string(),
        content_hash: "h".to_string(),
        ingested_at: chrono::Utc::now(),
    };
    let visual = PowerBiNode {
        id: "pbi_visual_card".to_string(),
        name: "RevenueCard".to_string(),
        kind: PowerBiNodeKind::Visual,
        file_path: "models/report.json".to_string(),
        source_path: "models".to_string(),
        content_hash: "h".to_string(),
        ingested_at: chrono::Utc::now(),
    };
    queries
        .upsert_powerbi_nodes(&[report.clone(), page.clone(), visual.clone()])
        .await
        .expect("upsert report nodes");
    let edges = vec![
        PowerBiEdge {
            from_id: report.id.clone(),
            to_id: page.id.clone(),
            edge_type: PowerBiEdgeType::Contains,
            source_path: "models".to_string(),
        },
        PowerBiEdge {
            from_id: page.id.clone(),
            to_id: visual.id.clone(),
            edge_type: PowerBiEdgeType::Contains,
            source_path: "models".to_string(),
        },
        PowerBiEdge {
            from_id: visual.id.clone(),
            to_id: measure_id.to_string(),
            edge_type: PowerBiEdgeType::UsesField,
            source_path: "models".to_string(),
        },
    ];
    queries
        .upsert_powerbi_edges(&edges)
        .await
        .expect("upsert report edges");
}

fn synthetic_function(name: &str) -> Function {
    Function {
        id: format!("function:{name}"),
        name: name.to_string(),
        file_path: "src/lib.rs".to_string(),
        line_start: 1,
        line_end: 3,
        signature: format!("fn {name}()"),
        docstring: None,
        body: format!("fn {name}() {{}}"),
        body_hash: "bh".to_string(),
        token_count: 4,
        embed_type: "explicit_code".to_string(),
        embedding: Vec::new(),
        summary: format!("fn {name}"),
    }
}

/// End-to-end: index the fixture, inject the report subgraph, then return the
/// bound `AppState`, the resolved node ids, and keep the workspace alive.
struct Harness {
    _root: TempDir,
    state: Arc<AppState>,
    nodes: Vec<PowerBiNode>,
}

async fn build_harness(inject_function: Option<&str>) -> Harness {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    write_fixture(&workspace);
    let data_dir = root.path().join("data");
    let branch = "main";

    let db = connect_db(&data_dir, branch).await.expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    index_powerbi_source(
        &powerbi_source("models"),
        &workspace,
        &queries,
        MAX_FILE_SIZE,
    )
    .await
    .expect("index tmdl");

    let nodes = queries
        .select_powerbi_nodes(Some("models"))
        .await
        .expect("select nodes");
    let measure = find_node(&nodes, "RevenueByYear", PowerBiNodeKind::Measure)
        .expect("RevenueByYear measure indexed");
    inject_report_subgraph(&queries, &measure.id).await;

    if let Some(fname) = inject_function {
        queries
            .upsert_function(&synthetic_function(fname))
            .await
            .expect("upsert function");
    }

    // Re-read nodes so the caller sees the injected report nodes too.
    let nodes = queries
        .select_powerbi_nodes(Some("models"))
        .await
        .expect("select nodes 2");

    // Release the DB handle so the tool dispatch reconnects cleanly.
    drop(queries);

    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    state
        .set_workspace(snapshot(
            data_dir,
            branch,
            workspace.to_string_lossy().as_ref(),
        ))
        .await
        .expect("set workspace");

    Harness {
        _root: root,
        state,
        nodes,
    }
}

fn neighborhood_names(resp: &serde_json::Value) -> Vec<String> {
    resp["powerbi_neighborhood"]
        .as_array()
        .expect("powerbi_neighborhood array")
        .iter()
        .map(|n| n["name"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn neighborhood_kinds(resp: &serde_json::Value) -> Vec<String> {
    resp["powerbi_neighborhood"]
        .as_array()
        .expect("powerbi_neighborhood array")
        .iter()
        .map(|n| n["kind"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// P4 core: blast radius over a column surfaces dependent measures and onward
/// visuals/pages/reports, and EXCLUDES the root column's owner table + model.
#[tokio::test]
async fn impact_analysis_powerbi_column_blast_radius_excludes_owner() {
    let harness = build_harness(None).await;
    let year = find_node(&harness.nodes, "Year", PowerBiNodeKind::Column).expect("Year column");

    let resp = tools::dispatch(
        harness.state.clone(),
        "impact_analysis",
        Some(json!({ "powerbi_node_id": year.id, "depth": 5, "max_nodes": 50 })),
    )
    .await
    .expect("impact_analysis powerbi root");

    assert_eq!(resp["root_kind"], "powerbi_entity");
    assert_eq!(
        resp["symbol"]["id"],
        serde_json::Value::String(year.id.clone())
    );

    let names = neighborhood_names(&resp);
    let kinds = neighborhood_kinds(&resp);

    // Dependent measure + onward visual/page/report.
    for expected in ["RevenueByYear", "RevenueCard", "Overview", "SalesReport"] {
        assert!(
            names.contains(&expected.to_string()),
            "expected {expected:?} in blast radius; got {names:?}"
        );
    }

    // NEGATIVE: the owner table and semantic model must be excluded.
    assert!(
        !kinds.iter().any(|k| k == "powerbi_table"),
        "owner table must be excluded; got kinds {kinds:?}"
    );
    assert!(
        !kinds.iter().any(|k| k == "powerbi_semantic_model"),
        "owner semantic model must be excluded; got kinds {kinds:?}"
    );
    assert!(
        !names.contains(&"Date".to_string()),
        "owner table 'Date' must be excluded; got {names:?}"
    );
    // The sibling column Sales.Amount is an upstream dependency of the measure,
    // not a dependent of the root column, so it must not appear.
    assert!(
        !names.contains(&"Amount".to_string()),
        "upstream column 'Amount' must be excluded; got {names:?}"
    );
}

/// A code-symbol root returns `root_kind = code_symbol` and NO
/// `powerbi_neighborhood`; existing fields stay intact.
#[tokio::test]
async fn impact_analysis_code_symbol_root_has_no_powerbi_neighborhood() {
    let harness = build_harness(Some("HelperOnlyInCode")).await;

    let resp = tools::dispatch(
        harness.state.clone(),
        "impact_analysis",
        Some(json!({ "symbol_name": "HelperOnlyInCode" })),
    )
    .await
    .expect("impact_analysis code root");

    assert_eq!(resp["root_kind"], "code_symbol");
    assert!(resp.get("powerbi_neighborhood").is_none());
    assert!(resp.get("code_neighborhood").is_some());
}

/// Disambiguation 1: a same-model duplicate column name resolves to exactly one
/// root via the `powerbi_node_id` selector.
#[tokio::test]
async fn impact_analysis_duplicate_column_name_pinned_by_selector() {
    let harness = build_harness(None).await;
    let date_key = find_node_in_file(&harness.nodes, "Key", PowerBiNodeKind::Column, "Date.tmdl")
        .expect("Date.Key column");
    let sales_key = find_node_in_file(&harness.nodes, "Key", PowerBiNodeKind::Column, "Sales.tmdl")
        .expect("Sales.Key column");
    assert_ne!(date_key.id, sales_key.id);

    // Name-based call is ambiguous → surfaces candidates.
    let ambiguous = tools::dispatch(
        harness.state.clone(),
        "impact_analysis",
        Some(json!({ "symbol_name": "Key", "depth": 3 })),
    )
    .await
    .expect("ambiguous powerbi name");
    assert_eq!(ambiguous["root_kind"], "powerbi_entity");
    let candidates = ambiguous["powerbi_candidates"]
        .as_array()
        .expect("powerbi_candidates present on ambiguity");
    assert_eq!(candidates.len(), 2, "both Key columns are candidates");

    // Selector pins exactly one root.
    let pinned = tools::dispatch(
        harness.state.clone(),
        "impact_analysis",
        Some(json!({ "powerbi_node_id": date_key.id, "depth": 3 })),
    )
    .await
    .expect("pinned powerbi root");
    assert_eq!(pinned["root_kind"], "powerbi_entity");
    assert_eq!(
        pinned["symbol"]["id"],
        serde_json::Value::String(date_key.id.clone())
    );
}

/// Disambiguation 2: a code-symbol / Power BI name collision resolves to
/// exactly one root. The default name path prefers the code symbol
/// (back-compat); the `powerbi_node_id` selector pins the Power BI entity.
#[tokio::test]
async fn impact_analysis_code_vs_powerbi_collision_pinned_by_selector() {
    // Inject a code function that collides with the measure name.
    let harness = build_harness(Some("RevenueByYear")).await;
    let measure = find_node(&harness.nodes, "RevenueByYear", PowerBiNodeKind::Measure)
        .expect("RevenueByYear measure");

    // Default name resolution → code symbol wins.
    let by_name = tools::dispatch(
        harness.state.clone(),
        "impact_analysis",
        Some(json!({ "symbol_name": "RevenueByYear" })),
    )
    .await
    .expect("name resolves to code symbol");
    assert_eq!(by_name["root_kind"], "code_symbol");

    // Selector pins the Power BI measure despite the collision.
    let by_selector = tools::dispatch(
        harness.state.clone(),
        "impact_analysis",
        Some(json!({ "powerbi_node_id": measure.id, "depth": 3 })),
    )
    .await
    .expect("selector resolves to powerbi entity");
    assert_eq!(by_selector["root_kind"], "powerbi_entity");
    assert_eq!(
        by_selector["symbol"]["id"],
        serde_json::Value::String(measure.id.clone())
    );
}

/// `find_powerbi_nodes_by_name` resolves by name and honours the optional
/// `kind` and `source_path` filters.
#[tokio::test]
async fn find_powerbi_nodes_by_name_filters_by_kind_and_source() {
    let root = TempDir::new().expect("tempdir");
    let workspace = root.path().join("workspace");
    write_fixture(&workspace);
    let db = connect_db(&root.path().join("data"), "main")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    index_powerbi_source(
        &powerbi_source("models"),
        &workspace,
        &queries,
        MAX_FILE_SIZE,
    )
    .await
    .expect("index tmdl");

    // Ambiguous name → both Key columns returned, sorted by id (deterministic).
    let keys = queries
        .find_powerbi_nodes_by_name("Key", None, None)
        .await
        .expect("find Key");
    assert_eq!(keys.len(), 2, "two columns named Key");
    assert!(keys.windows(2).all(|w| w[0].id <= w[1].id), "sorted by id");

    // Kind filter narrows to the measure, not any column.
    let measures = queries
        .find_powerbi_nodes_by_name("RevenueByYear", Some("measure"), None)
        .await
        .expect("find measure");
    assert_eq!(measures.len(), 1);
    assert_eq!(measures[0].kind, PowerBiNodeKind::Measure);

    // source_path filter matches the indexed scope; a bogus scope yields none.
    let scoped = queries
        .find_powerbi_nodes_by_name("Year", None, Some("models"))
        .await
        .expect("find Year scoped");
    assert_eq!(scoped.len(), 1);
    let missing = queries
        .find_powerbi_nodes_by_name("Year", None, Some("nope"))
        .await
        .expect("find Year wrong scope");
    assert!(missing.is_empty());

    // find_powerbi_node_by_id round-trips.
    let by_id = queries
        .find_powerbi_node_by_id(&scoped[0].id)
        .await
        .expect("find by id");
    assert_eq!(by_id.map(|n| n.name), Some("Year".to_string()));
}
