//! TMDL safe-parser differential-evaluation harness (066.008-T, tree-sitter decision gate).
//!
//! This harness runs a representative TMDL corpus through the SAFE line/indent
//! parser (`powerbi_tmdl_parser::parse_tmdl_document`) and asserts the produced
//! [`TmdlModel`] against expected structure. It is a **decision gate, not a
//! grammar build**: it quantifies where the safe parser drops, truncates, or
//! mis-scopes real-world TMDL structure so the 069-F tree-sitter question can be
//! decided on measured evidence. It is measurement-only — no production parser
//! behavior is changed, no dependency is added, and no grammar is introduced.
//!
//! Each `#[test]` pins the parser's **current** (measured) behavior. PASS tests
//! prove the safe parser captures a construct faithfully; MISS tests pin the
//! exact lossy output and classify the failure mode. Because the assertions pin
//! current behavior, any future parser improvement (e.g. modeling calculated
//! columns) will fail these tests and force this finding to be re-derived.
//!
//! ## Per-construct correctness delta (2026-07-04)
//!
//! | Fixture | Construct | Verdict | Failure mode | Grammar would fix? |
//! |---|---|---|---|---|
//! | S-PTM-20 | multi-object `model.tmdl` (name/culture/defaultMode/lineageTag/annotation/refs) | PASS | — | n/a |
//! | S-PTM-21 | block-form `relationships.tmdl` endpoints (incl. quoted table) | PASS | — | n/a |
//! | S-PTM-21 | relationship qualifiers (`isActive`/`crossFilteringBehavior`/`joinOnDateBehavior`) | MISS | dropped | NO — model-richness gap (no field to hold them) |
//! | S-PTM-22 | complex table core (columns+dataType, multiline DAX measure, partition + fenced M, scoped annotations/lineageTag) | PASS | — | n/a |
//! | S-PTM-23 | nested member block `hierarchy`/`level` | MISS | dropped (cleanly, no corruption) | NO — model-richness gap (no `hierarchies` field) |
//! | S-PTM-24 | calculated column (`column X = <DAX>`) | PASS | fixed in 070-S: name split on `=`, DAX captured into `TmdlColumn.expression` | n/a |
//! | S-PTM-25 | measure DAX body containing a `:` (e.g. `FORMAT(..,"HH:mm:ss")`) | MISS | truncated (whole body dropped) | YES — but incrementally fixable (refine property heuristic) |
//! | S-PTM-26 | `calculationGroup`/`calculationItem` | MISS | dropped | NO — model-richness gap (no calc-group type) |
//! | S-PTM-27 | RLS `role`/`tablePermission` | MISS | dropped | NO — model-richness gap (no role type) |
//!
//! ## Finding (baked-in decision rule)
//!
//! The material misses split into two classes: (1) **model-richness gaps** —
//! hierarchies, calculation groups, roles, and relationship qualifiers are
//! dropped because [`TmdlModel`] has no field to hold them; a tree-sitter
//! grammar does NOT surface these without also extending the model types and the
//! `src/services/powerbi_tmdl.rs` adapter, so parse technology is not the
//! bottleneck. (2) **heuristic parse bugs** — the calculated-column mis-scope
//! and the colon-in-DAX truncation are the only two cases where the line/indent
//! heuristic mis-parses structure that IS in the model; both are small and
//! incrementally fixable in the safe parser without a grammar, external
//! indentation scanner, or ABI pinning. No miss is a "material structural
//! mis-parse that is hard to fix incrementally." Recommendation: **DECLINE** —
//! the safe parser is sufficient; a tree-sitter grammar is not ROI-positive.
//! See `docs/decisions/2026-07-04-tmdl-eval-gate-finding.md`.

use engram::services::powerbi_tmdl::extract_tmdl_semantic_model;
use powerbi_tmdl_parser::{TmdlModel, parse_tmdl_document};

// ── Corpus fixtures ──────────────────────────────────────────────────────────

/// Multi-object `model.tmdl`: model metadata plus model-scope `ref` statements.
fn fixture_model_tmdl() -> &'static str {
    r#"
model My Model
  culture: en-US
  defaultMode: import
  lineageTag: model-guid
  annotation PBI_QueryOrder = ["Sales","Date"]

ref table Sales
ref table 'Date'
ref relationship SalesToDate
ref cultureInfo en-US
ref expression SynapseServer
"#
}

/// Block-form `relationships.tmdl`: named `relationship` blocks carrying
/// `fromColumn:`/`toColumn:` plus qualifier properties the parser does not model.
fn fixture_relationships() -> &'static str {
    r"
relationship SalesToDate
  fromColumn: Sales.OrderDateKey
  toColumn: 'Date'.DateKey
  crossFilteringBehavior: bothDirections
  isActive: false
  joinOnDateBehavior: datePartOnly

relationship SalesToProduct
  fromColumn: Sales.ProductKey
  toColumn: Product.ProductKey
"
}

/// Complex table: columns, a multiline DAX measure, a nested `hierarchy`/`level`
/// member block, and a `partition` with a fenced M body, with scoped
/// `annotation`/`lineageTag:` lines throughout.
fn fixture_complex_table() -> &'static str {
    r##"
table Sales
  lineageTag: table-guid
  annotation IsHidden = false

  column Amount
    dataType: double
    lineageTag: col-amount
    annotation Format = "#,0"

  measure 'Total Sales' =
    SUMX (
      Sales,
      Sales[Amount] * Sales[Qty]
    )
    lineageTag: measure-total

  hierarchy 'Calendar'
    lineageTag: hier-guid
    annotation HierAnno = hval
    level Year
      column: Year
    level Month
      column: Month

  partition Sales = m
    mode: import
    source = ```
      let
        Source = Sql.Database("srv", "db")
      in
        Source
      ```
"##
}

/// Calculated columns: single-line and multi-line `column X = <DAX>` forms.
fn fixture_calc_column() -> &'static str {
    r#"
table Sales
  column Margin = [SalesAmount] - [TotalCost]
    dataType: decimal
    lineageTag: col-margin

  column FullName =
    [FirstName] & " " & [LastName]
    dataType: string
"#
}

/// A measure whose multiline DAX body contains a colon inside a string literal.
fn fixture_colon_in_dax() -> &'static str {
    r#"
table Sales
  measure 'Clock' =
    FORMAT ( NOW (), "HH:mm:ss" )

  measure 'AfterClock' = 1 + 1
"#
}

/// A calculation-group table (`calculationGroup`/`calculationItem`).
fn fixture_calc_group() -> &'static str {
    r"
table 'Time Intelligence'
  calculationGroup
    calculationItem YTD = CALCULATE ( SELECTEDMEASURE (), DATESYTD ( 'Date'[Date] ) )
    calculationItem MTD = CALCULATE ( SELECTEDMEASURE (), DATESMTD ( 'Date'[Date] ) )

  column 'Name'
    dataType: string
"
}

/// A row-level-security `role` block preceding a table.
fn fixture_role() -> &'static str {
    r#"
role SalesReader
  modelPermission: read
  tablePermission Sales = Sales[Region] = "West"

table Sales
  column Region
    dataType: string
"#
}

fn parse(src: &str) -> TmdlModel {
    parse_tmdl_document(src).expect("fixture should parse into a TmdlModel")
}

// ── PASS: constructs the safe parser captures faithfully ─────────────────────

/// S-PTM-20: a multi-object `model.tmdl` is captured in full — model name,
/// `culture`/`defaultMode`/`lineageTag`, a model-scope `annotation`, and every
/// `ref` (table/relationship/cultureInfo/expression), quotes stripped.
#[test]
fn s_ptm_20_model_tmdl_fully_captured() {
    let model = parse(fixture_model_tmdl());

    assert_eq!(model.model_name.as_deref(), Some("My Model"));
    assert_eq!(model.culture.as_deref(), Some("en-US"));
    assert_eq!(model.default_mode.as_deref(), Some("import"));
    assert_eq!(model.lineage_tag.as_deref(), Some("model-guid"));

    assert_eq!(model.annotations.len(), 1);
    assert_eq!(model.annotations[0].name, "PBI_QueryOrder");
    assert_eq!(
        model.annotations[0].value.as_deref(),
        Some(r#"["Sales","Date"]"#)
    );

    let refs: Vec<(&str, &str)> = model
        .refs
        .iter()
        .map(|r| (r.kind.as_str(), r.name.as_str()))
        .collect();
    assert_eq!(
        refs,
        vec![
            ("table", "Sales"),
            ("table", "Date"),
            ("relationship", "SalesToDate"),
            ("cultureInfo", "en-US"),
            ("expression", "SynapseServer"),
        ]
    );
}

/// S-PTM-21 (PASS half): block-form relationship endpoints are captured for
/// every block, including a quoted target table (`'Date'` → `Date`).
#[test]
fn s_ptm_21_block_relationship_endpoints_captured() {
    let model = parse(fixture_relationships());

    assert_eq!(model.relationships.len(), 2);

    let first = &model.relationships[0];
    assert_eq!(first.name, "SalesToDate");
    assert_eq!(first.from_table, "Sales");
    assert_eq!(first.from_column, "OrderDateKey");
    assert_eq!(first.to_table, "Date");
    assert_eq!(first.to_column, "DateKey");

    let second = &model.relationships[1];
    assert_eq!(second.name, "SalesToProduct");
    assert_eq!(second.from_table, "Sales");
    assert_eq!(second.to_table, "Product");
}

/// S-PTM-21 (MISS half): relationship qualifier properties (`isActive`,
/// `crossFilteringBehavior`, `joinOnDateBehavior`) are DROPPED — the inactive
/// relationship is structurally indistinguishable from the active one.
///
/// Failure mode: **dropped**. Model-richness gap: `TmdlRelationship` carries
/// only endpoints, so a grammar would not surface these without extending the
/// model type.
#[test]
fn s_ptm_21_relationship_qualifiers_dropped() {
    let model = parse(fixture_relationships());

    // `SalesToDate` is declared `isActive: false`, `crossFilteringBehavior:
    // bothDirections`, and `joinOnDateBehavior: datePartOnly`, but
    // `TmdlRelationship` carries only endpoints, so every qualifier is dropped.
    // Pin the loss: none of the distinctive qualifier tokens survive anywhere in
    // the parsed model. If the parser were ever extended to capture a qualifier,
    // its value would appear here and this test would fail, forcing the finding
    // to be re-derived.
    let dump = format!("{model:#?}");
    assert!(
        !dump.contains("bothDirections"),
        "crossFilteringBehavior value must be dropped, found: {dump}"
    );
    assert!(
        !dump.contains("datePartOnly"),
        "joinOnDateBehavior value must be dropped, found: {dump}"
    );
    assert!(
        !dump.contains("false"),
        "isActive:false must be dropped — no active/inactive flag is represented"
    );

    // Consequently the inactive relationship is indistinguishable from the
    // active one in every represented field except its declared name.
    let inactive = &model.relationships[0];
    let active = &model.relationships[1];
    assert_eq!(inactive.from_table, active.from_table);
    assert_ne!(inactive.name, active.name);
}

/// S-PTM-22: the complex-table core is captured — columns with `dataType:`, a
/// multiline DAX measure body, a `partition` with its fenced M body, and
/// `annotation`/`lineageTag:` lines correctly scoped to table/column/measure.
#[test]
fn s_ptm_22_complex_table_core_captured() {
    let model = parse(fixture_complex_table());

    assert_eq!(model.tables.len(), 1);
    let table = &model.tables[0];
    assert_eq!(table.name, "Sales");
    assert_eq!(table.lineage_tag.as_deref(), Some("table-guid"));
    assert_eq!(table.annotations.len(), 1);
    assert_eq!(table.annotations[0].name, "IsHidden");

    // Column with dataType + scoped annotation/lineageTag.
    assert_eq!(table.columns.len(), 1);
    let amount = &table.columns[0];
    assert_eq!(amount.name, "Amount");
    assert_eq!(amount.data_type.as_deref(), Some("double"));
    assert_eq!(amount.lineage_tag.as_deref(), Some("col-amount"));
    assert_eq!(amount.annotations.len(), 1);
    assert_eq!(amount.annotations[0].name, "Format");

    // Multiline DAX measure body captured; trailing lineageTag scoped to it.
    assert_eq!(table.measures.len(), 1);
    let measure = &table.measures[0];
    assert_eq!(measure.name, "Total Sales");
    assert_eq!(
        measure.expression.as_deref(),
        Some("SUMX (\nSales,\nSales[Amount] * Sales[Qty]\n)")
    );
    assert_eq!(measure.lineage_tag.as_deref(), Some("measure-total"));

    // Partition with mode + fenced M body captured opaquely.
    assert_eq!(table.partitions.len(), 1);
    let partition = &table.partitions[0];
    assert_eq!(partition.name, "Sales");
    assert_eq!(partition.source_kind.as_deref(), Some("m"));
    assert_eq!(partition.mode.as_deref(), Some("import"));
    let body = partition
        .source_expression
        .as_deref()
        .expect("partition should capture the fenced M body");
    assert!(body.contains("Sql.Database"));
    assert!(!body.contains("```"));
}

// ── MISS: constructs the safe parser drops, truncates, or mis-scopes ─────────

/// S-PTM-23: a nested `hierarchy`/`level` member block is DROPPED — cleanly.
///
/// Failure mode: **dropped**. The hierarchy leaves no trace: its levels do not
/// become columns, its `annotation`/`lineageTag:` do not pollute the preceding
/// column or the table. Model-richness gap: `TmdlTable` has no `hierarchies`
/// field, so a grammar would not surface it without extending the model.
#[test]
fn s_ptm_23_hierarchy_member_block_dropped() {
    let model = parse(fixture_complex_table());
    let table = &model.tables[0];

    // Levels are not surfaced as columns.
    assert!(
        table
            .columns
            .iter()
            .all(|c| c.name != "Year" && c.name != "Month"),
        "hierarchy levels must not leak in as columns"
    );
    // Hierarchy metadata is dropped, not mis-attributed.
    assert!(
        !table.annotations.iter().any(|a| a.name == "HierAnno"),
        "hierarchy annotation must not attach to the table"
    );
    assert_eq!(
        table.lineage_tag.as_deref(),
        Some("table-guid"),
        "table lineageTag must not be overwritten by the hierarchy's"
    );
    // No representation of the hierarchy exists anywhere on the model.
}

/// S-PTM-24: a calculated column (`column X = <DAX>`) captures its name and DAX
/// expression correctly (fixed in 070-S / 070.001-T).
///
/// Both the single-line (`column Margin = <DAX>`) and multi-line
/// (`column FullName =` with the DAX on the following deeper-indented line) forms
/// split the name on the first `=` and capture the DAX into
/// `TmdlColumn.expression`, mirroring how measures capture their expression.
/// `dataType:` still attaches to the column.
#[test]
fn s_ptm_24_calculated_column_expression_captured() {
    let model = parse(fixture_calc_column());
    let table = &model.tables[0];

    assert_eq!(table.columns.len(), 2);

    // Single-line calc column: the name is split on `=`; the DAX is captured.
    assert_eq!(table.columns[0].name, "Margin");
    assert_eq!(
        table.columns[0].expression.as_deref(),
        Some("[SalesAmount] - [TotalCost]")
    );
    assert_eq!(table.columns[0].data_type.as_deref(), Some("decimal"));

    // Multi-line calc column: the trailing `=` opens a body capture that gathers
    // the deeper-indented DAX line into the expression.
    assert_eq!(table.columns[1].name, "FullName");
    assert_eq!(
        table.columns[1].expression.as_deref(),
        Some("[FirstName] & \" \" & [LastName]")
    );
    assert_eq!(table.columns[1].data_type.as_deref(), Some("string"));

    // The calculated logic is captured as a column expression, not as a measure.
    assert!(table.measures.is_empty());
}

/// S-PTM-25: a measure whose DAX body contains a colon is TRUNCATED — the whole
/// body is dropped because a `:` trips the "looks like a TMDL property" guard.
///
/// Failure mode: **truncated**. A grammar would not be fooled by a colon inside
/// a string literal, but this is also incrementally fixable by tightening the
/// property heuristic. A sibling single-line measure is unaffected.
#[test]
fn s_ptm_25_colon_in_dax_measure_truncated() {
    let model = parse(fixture_colon_in_dax());
    let table = &model.tables[0];

    assert_eq!(table.measures.len(), 2);

    // The `FORMAT ( NOW (), "HH:mm:ss" )` body is dropped: expression is None.
    assert_eq!(table.measures[0].name, "Clock");
    assert_eq!(
        table.measures[0].expression, None,
        "colon in the DAX body truncates the whole measure expression"
    );

    // A single-line measure with no colon is captured normally.
    assert_eq!(table.measures[1].name, "AfterClock");
    assert_eq!(table.measures[1].expression.as_deref(), Some("1 + 1"));
}

/// S-PTM-26: a `calculationGroup`/`calculationItem` block is DROPPED.
///
/// Failure mode: **dropped**. The calculation items never surface as measures
/// or any other entity; only the trailing plain column is captured.
/// Model-richness gap: there is no calculation-group type.
#[test]
fn s_ptm_26_calculation_group_dropped() {
    let model = parse(fixture_calc_group());
    let table = &model.tables[0];

    assert_eq!(table.name, "Time Intelligence");
    // Calculation items are not surfaced as measures.
    assert!(
        table.measures.is_empty(),
        "calculationItems must not leak in as measures"
    );
    // Only the trailing plain column survives.
    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.columns[0].name, "Name");
}

/// S-PTM-27: a row-level-security `role` block is DROPPED.
///
/// Failure mode: **dropped**. The role and its `tablePermission` never surface;
/// only the following table is captured. Model-richness gap: there is no role
/// type.
#[test]
fn s_ptm_27_rls_role_dropped() {
    let model = parse(fixture_role());

    // The role produced no entity; only the table remains.
    assert_eq!(model.tables.len(), 1);
    let table = &model.tables[0];
    assert_eq!(table.name, "Sales");
    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.columns[0].name, "Region");
    // No phantom column/measure was synthesized from the tablePermission DAX.
    assert!(table.measures.is_empty());
}

// ── Ingestion impact: one downstream assertion (per plan) ────────────────────

/// S-PTM-28: the calculated-column fix propagates through the ingestion adapter
/// (`extract_tmdl_semantic_model`) so the indexed `PowerBiColumn` name is the
/// clean column name (`Margin`), not the whole DAX expression. The adapter flows
/// the parser `name` through unchanged, so no adapter change was required.
#[test]
fn s_ptm_28_calc_column_name_reaches_indexed_entity() {
    let model = extract_tmdl_semantic_model(
        fixture_calc_column(),
        "models/Sales.SemanticModel/definition/tables/Sales.tmdl",
    )
    .expect("fixture should produce a semantic model");

    let table = &model.tables[0];
    // The indexed column name is the clean "Margin", not the DAX expression.
    assert_eq!(table.columns[0].name, "Margin");
    // A non-empty synthetic id is still generated over the clean name.
    assert!(!table.columns[0].id.is_empty());
}

// ── Finding anchor: aggregate delta summary ──────────────────────────────────

/// Re-derive the differential gate counts from live `parse_tmdl_document`
/// output. Returns `(passes, misses, model_richness_misses, heuristic_misses)`.
/// Because every verdict is computed from parser output, a fidelity change moves
/// these counts and fails the anchor test below.
fn differential_gate_counts() -> (usize, usize, usize, usize) {
    let model_tmdl = parse(fixture_model_tmdl());
    let rels = parse(fixture_relationships());
    let complex = parse(fixture_complex_table());
    let complex_table = &complex.tables[0];
    let calc_col = parse(fixture_calc_column());
    let colon = parse(fixture_colon_in_dax());
    let calc_group = parse(fixture_calc_group());
    let role = parse(fixture_role());

    // PASS verdicts (true == the construct is faithfully captured).
    let pass = [
        model_tmdl.model_name.as_deref() == Some("My Model") && model_tmdl.refs.len() == 5,
        rels.relationships.len() == 2 && rels.relationships[0].to_table == "Date",
        complex_table.columns.len() == 1
            && complex_table.measures.len() == 1
            && complex_table.measures[0].expression.is_some()
            && complex_table.partitions.len() == 1,
        // S-PTM-24 (fixed in 070.001-T): the calculated column splits its name on
        // `=` and captures the DAX into the column expression.
        calc_col.tables[0].columns[0].name == "Margin"
            && calc_col.tables[0].columns[0].expression.as_deref()
                == Some("[SalesAmount] - [TotalCost]"),
    ];

    // MISS verdicts (true == the loss is still present in parser output),
    // grouped by class. `model` == model-richness gap a grammar would not close
    // without extending the model types; `heur` == incrementally-fixable
    // heuristic parse bug.
    let model_misses = [
        !format!("{rels:#?}").contains("bothDirections"),
        complex_table
            .columns
            .iter()
            .all(|c| c.name != "Year" && c.name != "Month")
            && !complex_table
                .annotations
                .iter()
                .any(|a| a.name == "HierAnno"),
        calc_group.tables[0].measures.is_empty(),
        role.tables.len() == 1 && role.tables[0].name == "Sales",
    ];
    // Heuristic parse bugs (S-PTM-24 calc-column, S-PTM-25 colon-in-DAX). Each
    // entry is `true` only while its bug is still present, so `heuristic` counts
    // the remaining unfixed bugs. Retained as live regression guards: a
    // regression re-inflates the count and re-fails the anchor test below.
    let heur_misses = [
        calc_col.tables[0].columns[0].name.contains('='),
        colon.tables[0].measures[0].expression.is_none(),
    ];

    let passes = pass.iter().filter(|&&v| v).count();
    let model_richness = model_misses.iter().filter(|&&v| v).count();
    let heuristic = heur_misses.iter().filter(|&&v| v).count();
    (
        passes,
        model_richness + heuristic,
        model_richness,
        heuristic,
    )
}

/// S-PTM-29: assert the aggregate gate result. Every count is re-derived from
/// live `parse_tmdl_document` output (see [`differential_gate_counts`]), so this
/// is a genuine machine-checked anchor: if the parser's fidelity changes, the
/// counts change and this test forces the finding to be re-derived. The full
/// per-construct delta table lives in this module's docs and in
/// `docs/decisions/2026-07-04-tmdl-eval-gate-finding.md`.
#[test]
fn s_ptm_29_differential_summary_and_gate() {
    let (passes, misses, model_richness, heuristic_bugs) = differential_gate_counts();

    // Constructs the safe parser handles faithfully, including the S-PTM-24
    // calculated-column capture fixed in 070.001-T.
    assert_eq!(
        passes, 4,
        "expected four faithfully-captured constructs after the calc-column fix"
    );
    // Recorded misses still present; one heuristic bug remains (S-PTM-25).
    assert_eq!(misses, 5, "expected five recorded misses");
    // Gate rationale: the remaining misses are overwhelmingly model-richness gaps
    // a grammar does NOT close on its own, plus a single incrementally-fixable
    // heuristic bug (S-PTM-25, fixed next in 070.002-T).
    assert_eq!(
        model_richness, 4,
        "most misses are model-richness gaps independent of parse technology"
    );
    assert_eq!(
        heuristic_bugs, 1,
        "one heuristic parse bug remains (S-PTM-25 colon-in-DAX)"
    );

    eprintln!(
        "\nTMDL differential gate (066.008-T): {passes} PASS / {misses} MISS \
         ({model_richness} model-richness, {heuristic_bugs} incrementally-fixable \
         heuristic) → recommend DECLINE."
    );
}
