//! Fixture-driven precision/recall metric for the Spark notebook data-lineage
//! subgraph (095-F, Unit U6, task 095.007-T).
//!
//! This is a **tests-only** unit: it validates the already-shipped extractor
//! surface (U2/U3/U4) against a hand-labeled fixture matrix and derives an
//! empirical precision/recall metric **from the index-time-persisted
//! `lineage_edge` rows** (not from `run_retrieval_eval`), so the number
//! under-reports rather than over-reports (091-F guardrail).
//!
//! Two invariants are exercised:
//!
//! * **Precision floor (A5, hard gate):** every fixture in the DROPPED set must
//!   persist **zero** `lineage_edge` rows. A single false edge fails the build.
//! * **Recall (Fork A GO/NO-GO, reported not gated):** the fraction of
//!   resolvable ground-truth edges that the extractor actually persisted. The
//!   number is printed for the feasibility verdict; per the plan we do **not**
//!   assert a recall target (real-corpus recall is a product decision, not a
//!   build failure). A discrimination guard asserts at least one resolvable
//!   edge is emitted so the precision floor is not trivially satisfied by an
//!   extractor that emits nothing.
//!
//! Each fixture is indexed in **isolation** (its own temp workspace + store) so
//! a persisted edge is unambiguously attributable to that fixture — the A5
//! "0 false edges on every dropped case" claim is per-fixture, not aggregate.

#![cfg(feature = "cozo-backend")]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use engram::db::{connect_db, queries::CodeGraphQueries};
use engram::models::lineage::LineageAuthorityContext;
use engram::models::registry::{ContentSource, ContentSourceStatus};
use engram::services::notebook_indexer::index_notebook_source;

/// A ground-truth dataset endpoint reference, resolved through the *same*
/// trusted context the indexer uses so the derived id matches byte-for-byte.
#[derive(Clone, Copy)]
enum Ref {
    Table(&'static str),
    Path(&'static str),
}

/// One hand-labeled fixture: a notebook and its expected directional edges.
///
/// `expected` holds `(target, source)` ground-truth edges — empty for the
/// DROPPED set (the fail-closed cases that must persist no lineage).
struct Fixture {
    name: &'static str,
    notebook: &'static str,
    expected: &'static [(Ref, Ref)],
}

/// The trusted authority context: catalog `main` bound to a metastore
/// authority, and `s3://bucket` as the only trusted storage authority. Matches
/// the U4 write-path fixtures so ids line up across the suite.
fn trusted_ctx() -> LineageAuthorityContext {
    let mut catalogs = BTreeMap::new();
    catalogs.insert("main".to_owned(), "metastore-prod".to_owned());
    LineageAuthorityContext::new(catalogs, vec!["s3://bucket".to_owned()])
}

/// Resolve a ground-truth reference to its persisted `dataset_node` id via the
/// trusted context (the exact resolver the indexer used).
fn resolve_ref(ctx: &LineageAuthorityContext, r: Ref) -> String {
    match r {
        Ref::Table(literal) => {
            ctx.resolve_table(literal)
                .expect("ground-truth table resolves under the trusted context")
                .id
        }
        Ref::Path(literal) => {
            ctx.resolve_path(literal)
                .expect("ground-truth path resolves under the trusted context")
                .id
        }
    }
}

fn notebook_source(path: &str) -> ContentSource {
    ContentSource {
        content_type: "notebook".to_string(),
        language: None,
        path: path.to_string(),
        pattern: None,
        optional: false,
        status: ContentSourceStatus::Active,
    }
}

fn write_notebook(workspace: &Path, json: &str) {
    let dir = workspace.join("notebooks");
    fs::create_dir_all(&dir).expect("create notebooks dir");
    fs::write(dir.join("fixture.ipynb"), json).expect("write notebook fixture");
}

/// Index one fixture notebook in an isolated workspace/store and return the
/// persisted `lineage_edge` rows as `(from_id, to_id)` = `(target_id,
/// source_id)` pairs (AR-05 orientation).
async fn index_edges(notebook_json: &str) -> Vec<(String, String)> {
    let root = tempfile::TempDir::new().expect("tempdir");
    write_notebook(root.path(), notebook_json);

    let db = connect_db(&root.path().join("data"), "lineage-metric")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    index_notebook_source(
        &notebook_source("notebooks"),
        root.path(),
        &queries,
        1_048_576,
        &trusted_ctx(),
    )
    .await
    .expect("index notebook source");

    queries
        .select_lineage_edges()
        .await
        .expect("select lineage edges")
}

// ── Resolvable set: v1-supported shapes that MUST emit their labeled edge ─────

const RESOLVABLE: &[Fixture] = &[
    // R1: Spark-SQL CTAS, 3-part names under the trusted metastore.
    Fixture {
        name: "sql_ctas_3part",
        notebook: r#"{"cells":[{"cell_type":"code","source":"%%sql\nCREATE TABLE main.sales.summary AS SELECT x FROM main.sales.orders"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[(
            Ref::Table("main.sales.summary"),
            Ref::Table("main.sales.orders"),
        )],
    },
    // R2: Spark-SQL INSERT OVERWRITE TABLE (Spark `TABLE`-keyword normalized).
    Fixture {
        name: "sql_insert_overwrite",
        notebook: r#"{"cells":[{"cell_type":"code","source":"%%sql\nINSERT OVERWRITE TABLE main.sales.summary SELECT x FROM main.sales.orders"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[(
            Ref::Table("main.sales.summary"),
            Ref::Table("main.sales.orders"),
        )],
    },
    // R3: PySpark absolute-path read -> df -> write within one cell (`.save`).
    Fixture {
        name: "pyspark_path_single_cell",
        notebook: r#"{"cells":[{"cell_type":"code","source":"df = spark.read.parquet(\"s3://bucket/in\")\ndf.write.save(\"s3://bucket/out\")"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[(Ref::Path("s3://bucket/out"), Ref::Path("s3://bucket/in"))],
    },
    // R4: PySpark single-cell table read -> df -> saveAsTable write.
    Fixture {
        name: "pyspark_table_single_cell",
        notebook: r#"{"cells":[{"cell_type":"code","source":"df = spark.table(\"main.sales.orders\")\ndf.write.saveAsTable(\"main.sales.summary\")"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[(
            Ref::Table("main.sales.summary"),
            Ref::Table("main.sales.orders"),
        )],
    },
];

// ── Dropped set: fail-closed cases that MUST persist zero edges (A5) ──────────

const DROPPED: &[Fixture] = &[
    // D1: temp view is not a durable dataset; no transitive lineage through it
    // within the same cell (CREATE VIEW emits nothing; the CTAS reads the
    // 1-part `tmp` which does not resolve).
    Fixture {
        name: "sql_temp_view_same_cell",
        notebook: r#"{"cells":[{"cell_type":"code","source":"%%sql\nCREATE TEMPORARY VIEW tmp AS SELECT x FROM main.sales.orders;\nCREATE TABLE main.sales.summary AS SELECT x FROM tmp"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[],
    },
    // D2: temp view materialized in one cell, consumed in another — cross-cell
    // dataflow is OUT for v1; the SQL cell reads the 1-part `tmp` and drops.
    Fixture {
        name: "pyspark_temp_view_cross_cell",
        notebook: r#"{"cells":[{"cell_type":"code","source":"df = spark.table(\"main.sales.orders\")\ndf.createOrReplaceTempView(\"tmp\")"},{"cell_type":"code","source":"%%sql\nCREATE TABLE main.sales.summary AS SELECT x FROM tmp"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[],
    },
    // D3: 1-part names cannot be bound to a metastore authority — drop.
    Fixture {
        name: "sql_one_part_name",
        notebook: r#"{"cells":[{"cell_type":"code","source":"%%sql\nCREATE TABLE summary AS SELECT x FROM orders"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[],
    },
    // D4: 2-part names are still authority-ambiguous — drop.
    Fixture {
        name: "sql_two_part_name",
        notebook: r#"{"cells":[{"cell_type":"code","source":"%%sql\nCREATE TABLE sales.summary AS SELECT x FROM sales.orders"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[],
    },
    // D5: 3-part but the catalog `other` is not a trusted authority — drop.
    Fixture {
        name: "sql_untrusted_catalog",
        notebook: r#"{"cells":[{"cell_type":"code","source":"%%sql\nCREATE TABLE other.sales.summary AS SELECT x FROM other.sales.orders"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[],
    },
    // D6: relative path literals bind to no storage authority — drop.
    Fixture {
        name: "pyspark_relative_path",
        notebook: r#"{"cells":[{"cell_type":"code","source":"df = spark.read.parquet(\"data/in\")\ndf.write.save(\"data/out\")"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[],
    },
    // D7: an f-string read argument is non-literal — the read never binds `df`,
    // so the downstream write has no source — drop.
    Fixture {
        name: "pyspark_fstring_path",
        notebook: r#"{"cells":[{"cell_type":"code","source":"df = spark.read.parquet(f\"s3://bucket/{name}\")\ndf.write.save(\"s3://bucket/out\")"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[],
    },
    // D8 (AR-09): `spark.sql(...)` is deferred for v1 — even a fully resolvable
    // literal SQL string must persist zero edges.
    Fixture {
        name: "pyspark_spark_sql_literal",
        notebook: r#"{"cells":[{"cell_type":"code","source":"spark.sql(\"CREATE TABLE main.sales.summary AS SELECT x FROM main.sales.orders\")"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[],
    },
    // D9: a widget/config-derived write target is non-literal — drop.
    Fixture {
        name: "pyspark_widget_write",
        notebook: r#"{"cells":[{"cell_type":"code","source":"df = spark.table(\"main.sales.orders\")\ndf.write.saveAsTable(dbutils.widgets.get(\"t\"))"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[],
    },
    // D10: `s3://untrusted` is not a trusted storage authority — drop.
    Fixture {
        name: "pyspark_untrusted_storage",
        notebook: r#"{"cells":[{"cell_type":"code","source":"df = spark.read.parquet(\"s3://untrusted/in\")\ndf.write.save(\"s3://untrusted/out\")"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[],
    },
    // D11: cross-cell DataFrame dataflow is OUT for v1 — the write cell has no
    // in-cell binding for `df` — drop.
    Fixture {
        name: "pyspark_multicell_df_flow",
        notebook: r#"{"cells":[{"cell_type":"code","source":"df = spark.table(\"main.sales.orders\")"},{"cell_type":"code","source":"df.write.saveAsTable(\"main.sales.summary\")"}],"metadata":{"language_info":{"name":"python"}}}"#,
        expected: &[],
    },
];

/// **A5 precision floor (hard gate).** Every DROPPED fixture, indexed in
/// isolation, must persist zero `lineage_edge` rows. A single false edge is a
/// fail-closed violation and fails the build.
#[tokio::test]
async fn precision_floor_zero_false_edges_on_every_dropped_case() {
    for fixture in DROPPED {
        let edges = index_edges(fixture.notebook).await;
        assert!(
            edges.is_empty(),
            "DROPPED fixture '{}' must persist 0 lineage edges (A5 precision floor / fail-closed), \
             but got {edges:?}",
            fixture.name,
        );
    }
}

/// **Fork A recall (reported, not gated).** Compute the fraction of resolvable
/// ground-truth edges the extractor persisted and print it for the feasibility
/// verdict. No recall target is asserted (per plan). A discrimination guard
/// asserts at least one resolvable edge is emitted so the precision floor is
/// not trivially satisfied by an all-drop extractor.
#[tokio::test]
#[allow(clippy::cast_precision_loss)] // fixture counts are tiny; f64 is exact here
async fn fixture_recall_over_resolvable_set_is_measured_and_reported() {
    let ctx = trusted_ctx();
    let mut total = 0usize;
    let mut matched = 0usize;

    for fixture in RESOLVABLE {
        let edges = index_edges(fixture.notebook).await;
        for &(target, source) in fixture.expected {
            total += 1;
            let want = (resolve_ref(&ctx, target), resolve_ref(&ctx, source));
            if edges.contains(&want) {
                matched += 1;
            } else {
                println!(
                    "FORK-A-MISS: fixture='{}' expected={want:?} persisted={edges:?}",
                    fixture.name,
                );
            }
        }
    }

    let recall = matched as f64 / total as f64;
    println!("FORK-A-RECALL: matched={matched} total={total} recall={recall:.3}");

    assert_eq!(total, 4, "the resolvable set has 4 ground-truth edges");
    assert!(
        matched >= 1,
        "the extractor must persist at least one resolvable edge (discrimination guard: proves \
         the precision floor is not satisfied merely by an extractor that never emits)",
    );
}
