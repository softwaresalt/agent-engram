//! Integration tests for the Tier-2 (schema-aware) DAX linter (P6, `085.006-T`).
//!
//! Tier-2 rules resolve each measure / calculated-column DAX reference against a
//! MODEL-SCOPE aggregated schema (unioned across every sibling `.tmdl` file keyed
//! by `canonical_tmdl_model_path`), rebuilt by reparsing the files at lint time.
//! These tests drive [`lint_indexed_models`] over temp-dir fixtures, covering
//! each Tier-2 rule, measure-cycle safety, model-scope aggregation / isolation,
//! the sibling-change reparse (stale-caught / add-cleared), the `model_path`
//! selector, and the unindexed-path error.
//!
//! Tests: S-DAXT2-01 through S-DAXT2-10.

use std::fs;
use std::path::Path;

use engram::services::dax_lint::{LintError, lint_indexed_models};
use engram::services::verify::{Severity, VerifyFinding, VerifyReport};

const SOURCES: &[&str] = &["models"];

/// Return the `tables` directory of a named semantic model under `models/`.
fn model_tables(workspace: &Path, model: &str) -> std::path::PathBuf {
    workspace
        .join("models")
        .join(model)
        .join("definition")
        .join("tables")
}

fn sources() -> Vec<String> {
    SOURCES.iter().map(ToString::to_string).collect()
}

fn lint(workspace: &Path) -> VerifyReport {
    lint_indexed_models(workspace, &sources(), None).expect("lint should succeed")
}

fn rules(report: &VerifyReport) -> Vec<&str> {
    report.findings.iter().map(|f| f.rule.as_str()).collect()
}

fn findings_for<'a>(report: &'a VerifyReport, rule: &str) -> Vec<&'a VerifyFinding> {
    report.findings.iter().filter(|f| f.rule == rule).collect()
}

/// S-DAXT2-01: a broken column ref and a broken measure ref both fire, while a
/// VALID cross-table reference whose target column lives in a sibling `.tmdl` is
/// NOT reported broken (proves both rules fire without a false positive on the
/// aggregated model scope).
#[test]
fn broken_refs_fire_but_valid_cross_table_ref_is_clean() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let tables = model_tables(workspace, "Sales.SemanticModel");
    fs::create_dir_all(&tables).expect("create dirs");

    // `CrossValid` references [Total] (a measure) and 'Date'[Year] (a column in
    // the sibling Date.tmdl) — both valid. `BrokenCol` references a nonexistent
    // qualified column; `BrokenMeasure` references a nonexistent unqualified name.
    fs::write(
        tables.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure Total = SUM(Sales[Amount])\n\
         \x20\x20measure CrossValid = CALCULATE([Total], 'Date'[Year])\n\
         \x20\x20measure BrokenCol = SUM(Sales[Nonexistent])\n\
         \x20\x20measure BrokenMeasure = [DoesNotExist] + 1\n",
    )
    .expect("write Sales.tmdl");
    fs::write(
        tables.join("Date.tmdl"),
        "table Date\n\
         \x20\x20column Year\n\
         \x20\x20\x20\x20dataType: int64\n",
    )
    .expect("write Date.tmdl");

    let report = lint(workspace);
    assert!(
        !report.conformant,
        "broken refs must make the report non-conformant"
    );

    let broken_col = findings_for(&report, "dax.broken_column_ref");
    assert_eq!(
        broken_col.len(),
        1,
        "exactly one broken column ref expected (Sales[Nonexistent]); got {:?}",
        rules(&report)
    );
    assert!(
        broken_col[0].message.contains("Nonexistent"),
        "broken column finding must name the offending column; got {}",
        broken_col[0].message
    );

    let broken_measure = findings_for(&report, "dax.broken_measure_ref");
    assert_eq!(
        broken_measure.len(),
        1,
        "exactly one broken measure ref expected ([DoesNotExist]); got {:?}",
        rules(&report)
    );
    assert!(
        broken_measure[0].message.contains("DoesNotExist"),
        "broken measure finding must name the offending reference; got {}",
        broken_measure[0].message
    );

    // The valid cross-table reference must not be reported broken.
    assert!(
        !report.findings.iter().any(|f| f.message.contains("Year")),
        "valid cross-table 'Date'[Year] reference must not fire any finding; got {:?}",
        report.findings
    );
}

/// S-DAXT2-02: an unqualified column reference that resolves on the current
/// table fires `dax.unqualified_column` (Warning), not an error.
#[test]
fn unqualified_column_reference_is_a_warning() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let tables = model_tables(workspace, "Sales.SemanticModel");
    fs::create_dir_all(&tables).expect("create dirs");

    fs::write(
        tables.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20column Doubled = [Amount] * 2\n",
    )
    .expect("write Sales.tmdl");

    let report = lint(workspace);
    let unqualified = findings_for(&report, "dax.unqualified_column");
    assert_eq!(unqualified.len(), 1, "got rules {:?}", rules(&report));
    assert_eq!(unqualified[0].severity, Severity::Warning);
}

/// S-DAXT2-03: a table-qualified reference to a name that is actually a measure
/// fires `dax.qualified_measure` (Warning).
#[test]
fn qualified_measure_reference_is_a_warning() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let tables = model_tables(workspace, "Sales.SemanticModel");
    fs::create_dir_all(&tables).expect("create dirs");

    fs::write(
        tables.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure Total = SUM(Sales[Amount])\n\
         \x20\x20measure Qualified = Sales[Total] + 1\n",
    )
    .expect("write Sales.tmdl");

    let report = lint(workspace);
    let qualified = findings_for(&report, "dax.qualified_measure");
    assert_eq!(qualified.len(), 1, "got rules {:?}", rules(&report));
    assert_eq!(qualified[0].severity, Severity::Warning);
    assert!(
        qualified[0].message.contains("Total"),
        "finding should name the measure; got {}",
        qualified[0].message
    );
}

/// S-DAXT2-04: a self-referential measure fires `dax.measure_cycle` and the
/// cycle-safe traversal terminates (the test completing IS the safety proof).
#[test]
fn self_referential_measure_cycle_is_detected_and_terminates() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let tables = model_tables(workspace, "Sales.SemanticModel");
    fs::create_dir_all(&tables).expect("create dirs");

    fs::write(
        tables.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure Loop = [Loop] + 1\n",
    )
    .expect("write Sales.tmdl");

    let report = lint(workspace);
    let cycles = findings_for(&report, "dax.measure_cycle");
    assert_eq!(cycles.len(), 1, "self-ref should fire one cycle finding");
    assert_eq!(cycles[0].severity, Severity::Error);
}

/// S-DAXT2-05: mutually-referential measures both fire `dax.measure_cycle` and
/// the traversal terminates.
#[test]
fn mutual_measure_cycle_is_detected_and_terminates() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let tables = model_tables(workspace, "Sales.SemanticModel");
    fs::create_dir_all(&tables).expect("create dirs");

    fs::write(
        tables.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure A = [B] + 1\n\
         \x20\x20measure B = [A] + 1\n",
    )
    .expect("write Sales.tmdl");

    let report = lint(workspace);
    let cycles = findings_for(&report, "dax.measure_cycle");
    assert_eq!(
        cycles.len(),
        2,
        "both measures in the mutual cycle should fire; got {:?}",
        rules(&report)
    );
}

/// S-DAXT2-06: renaming a column in a sibling `.tmdl` makes a peer file's
/// previously-valid reference fire `dax.broken_column_ref` on re-lint (the
/// reparse reads the CURRENT on-disk model-scope schema).
#[test]
fn sibling_column_rename_makes_peer_reference_stale() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let tables = model_tables(workspace, "Sales.SemanticModel");
    fs::create_dir_all(&tables).expect("create dirs");

    fs::write(
        tables.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure Total = SUM(Sales[Amount])\n\
         \x20\x20measure UsesYear = CALCULATE([Total], 'Date'[Year])\n",
    )
    .expect("write Sales.tmdl");
    fs::write(
        tables.join("Date.tmdl"),
        "table Date\n\
         \x20\x20column Year\n\
         \x20\x20\x20\x20dataType: int64\n",
    )
    .expect("write Date.tmdl");

    let before = lint(workspace);
    assert!(
        findings_for(&before, "dax.broken_column_ref").is_empty(),
        "'Date'[Year] must resolve before the rename; got {:?}",
        before.findings
    );

    // Rename Year -> FiscalYear in the sibling. Sales.tmdl is untouched.
    fs::write(
        tables.join("Date.tmdl"),
        "table Date\n\
         \x20\x20column FiscalYear\n\
         \x20\x20\x20\x20dataType: int64\n",
    )
    .expect("update Date.tmdl");

    let after = lint(workspace);
    let broken = findings_for(&after, "dax.broken_column_ref");
    assert_eq!(
        broken.len(),
        1,
        "the unchanged Sales sibling must now report the stale 'Date'[Year] ref; got {:?}",
        rules(&after)
    );
    assert!(broken[0].message.contains("Year"));
}

/// S-DAXT2-07: adding a column to a sibling `.tmdl` clears a previously-broken
/// reference in a peer file on re-lint.
#[test]
fn sibling_column_add_clears_broken_reference() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let tables = model_tables(workspace, "Sales.SemanticModel");
    fs::create_dir_all(&tables).expect("create dirs");

    fs::write(
        tables.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure Total = SUM(Sales[Amount])\n\
         \x20\x20measure UsesQuarter = CALCULATE([Total], 'Date'[Quarter])\n",
    )
    .expect("write Sales.tmdl");
    fs::write(
        tables.join("Date.tmdl"),
        "table Date\n\
         \x20\x20column Year\n\
         \x20\x20\x20\x20dataType: int64\n",
    )
    .expect("write Date.tmdl");

    let before = lint(workspace);
    assert_eq!(
        findings_for(&before, "dax.broken_column_ref").len(),
        1,
        "'Date'[Quarter] must be broken before the column is added; got {:?}",
        rules(&before)
    );

    // Add the Quarter column to the sibling. Sales.tmdl is untouched.
    fs::write(
        tables.join("Date.tmdl"),
        "table Date\n\
         \x20\x20column Year\n\
         \x20\x20\x20\x20dataType: int64\n\
         \x20\x20column Quarter\n\
         \x20\x20\x20\x20dataType: int64\n",
    )
    .expect("update Date.tmdl");

    let after = lint(workspace);
    assert!(
        findings_for(&after, "dax.broken_column_ref").is_empty(),
        "adding 'Date'[Quarter] must clear the previously-broken ref; got {:?}",
        after.findings
    );
}

/// S-DAXT2-08: two semantic models under one registered source resolve to
/// DISTINCT scopes — a name that exists only in model B is broken in model A.
#[test]
fn two_models_under_one_source_are_isolated_scopes() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let sales = model_tables(workspace, "Sales.SemanticModel");
    let inventory = model_tables(workspace, "Inventory.SemanticModel");
    fs::create_dir_all(&sales).expect("create sales dirs");
    fs::create_dir_all(&inventory).expect("create inventory dirs");

    // Sales references [StockLevel], a measure that exists only in the Inventory
    // model. If scopes leaked, this would resolve; isolated, it is broken.
    fs::write(
        sales.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure Total = SUM(Sales[Amount])\n\
         \x20\x20measure Leaky = [StockLevel] + [Total]\n",
    )
    .expect("write Sales.tmdl");
    fs::write(
        inventory.join("Inventory.tmdl"),
        "table Inventory\n\
         \x20\x20column Units\n\
         \x20\x20\x20\x20dataType: int64\n\
         \x20\x20measure StockLevel = SUM(Inventory[Units])\n",
    )
    .expect("write Inventory.tmdl");

    let report = lint(workspace);
    let broken_measure = findings_for(&report, "dax.broken_measure_ref");
    assert_eq!(
        broken_measure.len(),
        1,
        "the cross-model [StockLevel] ref must be broken in the Sales scope; got {:?}",
        rules(&report)
    );
    assert!(broken_measure[0].message.contains("StockLevel"));
    assert!(
        broken_measure[0].message.contains("Sales.SemanticModel"),
        "the broken finding must belong to the Sales model file; got {}",
        broken_measure[0].message
    );
}

/// S-DAXT2-09: the `model_path` selector filters to exactly one model scope.
#[test]
fn model_path_selector_filters_to_one_scope() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let sales = model_tables(workspace, "Sales.SemanticModel");
    let inventory = model_tables(workspace, "Inventory.SemanticModel");
    fs::create_dir_all(&sales).expect("create sales dirs");
    fs::create_dir_all(&inventory).expect("create inventory dirs");

    // Each model has its own broken ref so we can tell the scopes apart.
    fs::write(
        sales.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure SalesBroken = SUM(Sales[MissingSalesCol])\n",
    )
    .expect("write Sales.tmdl");
    fs::write(
        inventory.join("Inventory.tmdl"),
        "table Inventory\n\
         \x20\x20column Units\n\
         \x20\x20\x20\x20dataType: int64\n\
         \x20\x20measure InvBroken = SUM(Inventory[MissingInvCol])\n",
    )
    .expect("write Inventory.tmdl");

    let sales_scope = "models/Sales.SemanticModel/definition/tables/Sales.tmdl";
    let report = lint_indexed_models(workspace, &sources(), Some(sales_scope))
        .expect("filtered lint should succeed");
    assert_eq!(
        report.findings.len(),
        1,
        "only the Sales scope should be linted; got {:?}",
        report.findings
    );
    assert!(
        report.findings[0].message.contains("MissingSalesCol"),
        "filtered lint must report only the Sales model finding; got {}",
        report.findings[0].message
    );
}

/// S-DAXT2-10: a `model_path` that matches no indexed model is an error result.
#[test]
fn unindexed_model_path_is_an_error() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let tables = model_tables(workspace, "Sales.SemanticModel");
    fs::create_dir_all(&tables).expect("create dirs");
    fs::write(
        tables.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n",
    )
    .expect("write Sales.tmdl");

    let bogus = "models/DoesNotExist.SemanticModel/definition/tables/Ghost.tmdl";
    let err = lint_indexed_models(workspace, &sources(), Some(bogus))
        .expect_err("an unindexed model_path must be an error");
    assert_eq!(err, LintError::ModelPathNotIndexed(bogus.to_string()));
}

/// S-DAXT2-12: an indexed `.tmdl` file in an active model that cannot be decoded
/// as UTF-8 is surfaced as [`LintError::FileUnreadable`] (carrying the offending
/// path) rather than silently skipped, so a whole-workspace lint never reports a
/// false `conformant: true` while an active model went unexamined.
#[test]
fn undecodable_model_file_is_an_error() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let tables = model_tables(workspace, "Sales.SemanticModel");
    fs::create_dir_all(&tables).expect("create dirs");
    // Invalid UTF-8 bytes with a `.tmdl` extension inside an active source.
    fs::write(tables.join("Broken.tmdl"), [0xFF, 0xFE, 0x00, 0x9F])
        .expect("write invalid utf-8 tmdl");

    let err = lint_indexed_models(workspace, &sources(), None)
        .expect_err("an undecodable model file must be an error");
    match err {
        LintError::FileUnreadable { path, .. } => {
            assert!(
                path.ends_with("Broken.tmdl"),
                "error path should name the offending file, got {path:?}"
            );
        }
        LintError::ModelPathNotIndexed(path) => {
            panic!("expected FileUnreadable, got ModelPathNotIndexed({path:?})")
        }
    }
}

/// S-DAXT2-11: DAX identifiers are case-insensitive, so references whose casing
/// differs from the declared table / column / measure names must resolve — a
/// lowercase `sales[amount]` against a declared `Sales`/`Amount`, an uppercase
/// `[TOTAL]` against a declared `Total` measure, and a mixed-case measure-cycle —
/// none of which may be mis-flagged broken or missed.
#[test]
fn dax_references_resolve_case_insensitively() {
    let root = tempfile::TempDir::new().expect("tempdir");
    let workspace = root.path();
    let tables = model_tables(workspace, "Sales.SemanticModel");
    fs::create_dir_all(&tables).expect("create dirs");

    // Declared casing: table `Sales`, column `Amount`, measures `Total`, `Net`.
    // `CaseCol` references the column with different casing (`sales[amount]`);
    // `CaseMeasure` references the measure with different casing (`[TOTAL]`).
    // Both must resolve cleanly (no broken-ref finding). `CycleA`/`CycleB` form a
    // mutual cycle referenced with mismatched casing to prove cycle detection
    // folds case too.
    fs::write(
        tables.join("Sales.tmdl"),
        "table Sales\n\
         \x20\x20column Amount\n\
         \x20\x20\x20\x20dataType: double\n\
         \x20\x20measure Total = SUM(Sales[Amount])\n\
         \x20\x20measure Net = [total] - 1\n\
         \x20\x20measure CaseCol = SUM(sales[amount])\n\
         \x20\x20measure CaseMeasure = [TOTAL] + 1\n\
         \x20\x20measure CycleA = [cycleb] + 1\n\
         \x20\x20measure CycleB = [CYCLEA] + 1\n",
    )
    .expect("write Sales.tmdl");

    let report = lint(workspace);

    // No broken-column / broken-measure findings: every mismatched-case reference
    // resolves against the case-folded model scope.
    assert!(
        findings_for(&report, "dax.broken_column_ref").is_empty(),
        "case-insensitive column refs must not be flagged broken; got {:?}",
        rules(&report)
    );
    assert!(
        findings_for(&report, "dax.broken_measure_ref").is_empty(),
        "case-insensitive measure refs must not be flagged broken; got {:?}",
        rules(&report)
    );

    // The mixed-case mutual cycle (CycleA↔CycleB) must still be detected.
    let cycles = findings_for(&report, "dax.measure_cycle");
    assert!(
        cycles.iter().any(|f| f.message.contains("CycleA"))
            && cycles.iter().any(|f| f.message.contains("CycleB")),
        "case-mismatched measure cycle must be detected; got {:?}",
        rules(&report)
    );
}
