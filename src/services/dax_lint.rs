//! Tier-1 (syntactic) DAX linter driving the `engram verify <model.tmdl>` gate.
//!
//! Tier 1 is deterministic, pure, and local (no daemon, no database): it parses
//! a TMDL document, extracts each measure and calculated-column DAX expression,
//! and applies a small set of syntactic rules that produce
//! [`VerifyFinding`]s. The `engram verify` CLI maps the resulting
//! [`VerifyReport::conformant`] flag onto its pinned exit-code contract so the
//! linter is usable as a pre-commit / autoharness gate.
//!
//! Rules (all namespaced `dax.*`):
//! - `dax.empty_expression` ([`Severity::Warning`]): the expression has no
//!   content once comments and whitespace are removed.
//! - `dax.divide_operator` ([`Severity::Info`]): the expression divides with the
//!   `/` operator instead of the `DIVIDE()` function (unguarded division).
//! - `dax.deprecated_function` ([`Severity::Warning`]): the expression calls a
//!   function flagged as legacy / discouraged.
//! - `dax.malformed_ref` ([`Severity::Error`]): driven by the P1 extractor's
//!   diagnostics seam (unterminated string / quoted identifier / bracket /
//!   block comment) rather than re-lexing.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use powerbi_tmdl_parser::{DaxDiagnostic, extract_dax_references};

use crate::models::powerbi::PowerBiSemanticModel;
use crate::services::powerbi_indexer::{ModelScopeSchema, collect_powerbi_files_in_workspace};
use crate::services::powerbi_tmdl::{canonical_tmdl_model_path, extract_tmdl_semantic_model};
use crate::services::verify::{Severity, VerifyFinding, VerifyReport};

/// DAX functions flagged as legacy / discouraged.
///
/// Row-context iteration via `EARLIER` / `EARLIEST` is flagged by common DAX
/// best-practice analyzers in favour of `VAR` / `RETURN` variables, which are
/// clearer and avoid nested-row-context pitfalls.
const DEPRECATED_FUNCTIONS: &[&str] = &["EARLIER", "EARLIEST"];

/// Run the Tier-1 DAX lint over every measure and calculated column in a TMDL
/// document.
///
/// Returns a [`VerifyReport`] whose `conformant` flag mirrors the existing
/// verify exit-code contract (conformant iff no findings). A document with no
/// extractable semantic model (e.g. a non-model TMDL fragment) yields a
/// conformant, empty report.
#[must_use]
pub fn verify_tmdl_dax(rel_path: &str, content: &str) -> VerifyReport {
    let mut findings: Vec<VerifyFinding> = Vec::new();

    if let Some(model) = extract_tmdl_semantic_model(content, rel_path) {
        for table in &model.tables {
            for measure in &table.measures {
                if let Some(expression) = measure.expression.as_deref() {
                    let location = format!("{}[{}] (measure)", table.name, measure.name);
                    findings.extend(lint_dax_expression(rel_path, &location, expression));
                }
            }
            for column in &table.columns {
                if let Some(expression) = column.expression.as_deref() {
                    let location = format!("{}[{}] (calculated column)", table.name, column.name);
                    findings.extend(lint_dax_expression(rel_path, &location, expression));
                }
            }
        }
    }

    VerifyReport::from_findings(findings)
}

/// Apply the Tier-1 rule set to a single DAX `expression`.
///
/// `location` contextualises the diagnostic (e.g. `Sales[Total Sales]
/// (measure)`). Findings are dropped, never fabricated: `dax.malformed_ref` is
/// emitted only from the extractor's diagnostics seam, not by re-lexing.
#[must_use]
pub fn lint_dax_expression(rel_path: &str, location: &str, expression: &str) -> Vec<VerifyFinding> {
    let mut findings: Vec<VerifyFinding> = Vec::new();
    let references = extract_dax_references(expression);

    if is_effectively_empty(expression) {
        findings.push(finding(
            rel_path,
            location,
            "dax.empty_expression",
            "DAX expression is empty".to_string(),
            Severity::Warning,
        ));
    }

    if contains_bare_division(expression) {
        findings.push(finding(
            rel_path,
            location,
            "dax.divide_operator",
            "uses the '/' operator; prefer DIVIDE() for guarded division".to_string(),
            Severity::Info,
        ));
    }

    for function in &references.functions {
        if DEPRECATED_FUNCTIONS
            .iter()
            .any(|deprecated| deprecated.eq_ignore_ascii_case(function))
        {
            findings.push(finding(
                rel_path,
                location,
                "dax.deprecated_function",
                format!("calls deprecated function {function}()"),
                Severity::Warning,
            ));
        }
    }

    for diagnostic in &references.diagnostics {
        findings.push(finding(
            rel_path,
            location,
            "dax.malformed_ref",
            malformed_message(diagnostic).to_string(),
            Severity::Error,
        ));
    }

    findings
}

/// Build a `dax.*` [`VerifyFinding`]. Line is unknown (`None`) because the model
/// adapter does not preserve per-expression source spans; the `location`
/// carried in `message` identifies the offending member instead.
fn finding(
    rel_path: &str,
    location: &str,
    rule: &str,
    message: String,
    severity: Severity,
) -> VerifyFinding {
    VerifyFinding {
        rule: rule.to_string(),
        message: format!("{rel_path}: {location}: {message}"),
        line: None,
        severity,
    }
}

/// Map an extractor diagnostic to a stable `dax.malformed_ref` message.
fn malformed_message(diagnostic: &DaxDiagnostic) -> &'static str {
    match diagnostic {
        DaxDiagnostic::UnterminatedString => "unterminated string literal in DAX expression",
        DaxDiagnostic::UnterminatedQuotedIdentifier => {
            "unterminated quoted table identifier in DAX expression"
        }
        DaxDiagnostic::UnterminatedBlockComment => "unterminated block comment in DAX expression",
        DaxDiagnostic::UnterminatedBracket => "unterminated bracketed reference in DAX expression",
    }
}

/// Whether an expression is empty once DAX comments and whitespace are removed.
fn is_effectively_empty(expression: &str) -> bool {
    strip_comments(expression).trim().is_empty()
}

/// Remove DAX line (`//`, `--`) and block (`/* */`) comments from an expression.
///
/// String literals, quoted identifiers, and bracketed names are preserved so a
/// `--` sequence inside data is not mistaken for a comment.
fn strip_comments(expression: &str) -> String {
    let chars: Vec<char> = expression.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' => i = copy_string_literal(&chars, i, &mut out),
            '\'' => i = copy_quoted_identifier(&chars, i, &mut out),
            '[' => i = copy_bracketed_name(&chars, i, &mut out),
            '/' if i + 1 < chars.len() && chars[i + 1] == '/' => {
                i = skip_line_comment(&chars, i + 2);
            }
            '-' if i + 1 < chars.len() && chars[i + 1] == '-' => {
                i = skip_line_comment(&chars, i + 2);
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                i = skip_block_comment(&chars, i + 2);
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

fn skip_line_comment(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && chars[i] != '\n' {
        i += 1;
    }
    i
}

fn skip_block_comment(chars: &[char], mut i: usize) -> usize {
    while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
        i += 1;
    }
    (i + 2).min(chars.len())
}

fn copy_string_literal(chars: &[char], start: usize, out: &mut String) -> usize {
    copy_delimited(chars, start, out, '"', '"')
}

fn copy_quoted_identifier(chars: &[char], start: usize, out: &mut String) -> usize {
    copy_delimited(chars, start, out, '\'', '\'')
}

fn copy_bracketed_name(chars: &[char], start: usize, out: &mut String) -> usize {
    copy_delimited(chars, start, out, '[', ']')
}

fn copy_delimited(
    chars: &[char],
    start: usize,
    out: &mut String,
    open: char,
    close: char,
) -> usize {
    debug_assert_eq!(chars[start], open);
    out.push(chars[start]);
    let mut i = start + 1;
    while i < chars.len() {
        out.push(chars[i]);
        if chars[i] == close {
            if i + 1 < chars.len() && chars[i + 1] == close {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    i
}

/// Whether the expression uses a bare `/` division operator.
///
/// Strings, quoted identifiers, bracketed references, and comments are skipped
/// so a `/` inside a literal or a line/block comment is not misreported.
fn contains_bare_division(expression: &str) -> bool {
    let chars: Vec<char> = expression.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '"' => {
                i += 1;
                // `""` is an escaped quote inside a DAX string literal, so a
                // doubled quote does not end the string.
                while i < chars.len() {
                    if chars[i] == '"' {
                        if i + 1 < chars.len() && chars[i + 1] == '"' {
                            i += 2;
                            continue;
                        }
                        break;
                    }
                    i += 1;
                }
                i += 1;
            }
            '\'' => {
                i += 1;
                // `''` is an escaped quote inside a quoted identifier.
                while i < chars.len() {
                    if chars[i] == '\'' {
                        if i + 1 < chars.len() && chars[i + 1] == '\'' {
                            i += 2;
                            continue;
                        }
                        break;
                    }
                    i += 1;
                }
                i += 1;
            }
            '[' => {
                i += 1;
                // `]]` is an escaped closing bracket inside a bracketed name
                // (mirrors the lexer), so a doubled bracket does not end the
                // reference — otherwise a `/` in the remainder of the name
                // (e.g. `[Path ]] / USD]`) would falsely trip the scanner.
                while i < chars.len() {
                    if chars[i] == ']' {
                        if i + 1 < chars.len() && chars[i + 1] == ']' {
                            i += 2;
                            continue;
                        }
                        break;
                    }
                    i += 1;
                }
                i += 1;
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '/' => {
                i += 2;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '-' if i + 1 < chars.len() && chars[i + 1] == '-' => {
                i += 2;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '/' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
            }
            '/' => return true,
            _ => i += 1,
        }
    }
    false
}

// ── Tier 2 — schema-aware semantic rules ────────────────────────────────────

/// A measure within a model scope, carrying the context needed to lint it and
/// to participate in measure→measure cycle detection.
#[derive(Debug, Clone)]
pub(crate) struct ScopeMeasure {
    /// Logical path of the `.tmdl` file declaring the measure.
    pub rel_path: String,
    /// Owning table name.
    pub table: String,
    /// Measure name (unique within a model scope).
    pub name: String,
    /// The measure's DAX expression.
    pub expression: String,
}

/// Lint every measure and calculated-column expression in a parsed `model`,
/// running both Tier-1 (syntactic) and Tier-2 (schema-aware) rules.
///
/// `schema` must be the aggregated [`ModelScopeSchema`] for the model's scope
/// (unioned across every sibling `.tmdl` file keyed by
/// `canonical_tmdl_model_path`) so a valid cross-file reference is not
/// mis-reported as broken.
#[must_use]
pub(crate) fn lint_model_dax(
    rel_path: &str,
    model: &PowerBiSemanticModel,
    schema: &ModelScopeSchema,
) -> Vec<VerifyFinding> {
    let mut findings: Vec<VerifyFinding> = Vec::new();
    for table in &model.tables {
        for measure in &table.measures {
            if let Some(expression) = measure.expression.as_deref() {
                let location = format!("{}[{}] (measure)", table.name, measure.name);
                findings.extend(lint_dax_expression(rel_path, &location, expression));
                findings.extend(lint_dax_expression_semantic(
                    rel_path,
                    &location,
                    &table.name,
                    expression,
                    schema,
                ));
            }
        }
        for column in &table.columns {
            if let Some(expression) = column.expression.as_deref() {
                let location = format!("{}[{}] (calculated column)", table.name, column.name);
                findings.extend(lint_dax_expression(rel_path, &location, expression));
                findings.extend(lint_dax_expression_semantic(
                    rel_path,
                    &location,
                    &table.name,
                    expression,
                    schema,
                ));
            }
        }
    }
    findings
}

/// Apply the Tier-2 (schema-aware) rule set to a single DAX `expression`.
///
/// `current_table` is the table owning the member being linted, used to resolve
/// unqualified references. Resolution mirrors the P3 indexer's edge resolver so
/// a reference the indexer *would* resolve to a real node never fires a
/// broken-ref finding; the linter additionally surfaces style findings
/// (`dax.unqualified_column`, `dax.qualified_measure`) that the indexer does not
/// model.
///
/// Rules:
/// - qualified `Table[X]`: valid if `X` is a column of `Table`; a
///   [`Severity::Warning`] `dax.qualified_measure` if `X` is a measure (measures
///   are model-scoped and should be referenced unqualified); otherwise a
///   [`Severity::Error`] `dax.broken_column_ref`.
/// - unqualified `[X]`: valid if `X` is a measure; a [`Severity::Warning`]
///   `dax.unqualified_column` if `X` is a column of `current_table`; a
///   [`Severity::Error`] `dax.broken_column_ref` if `X` is a column of some
///   *other* table (must be qualified); otherwise a [`Severity::Error`]
///   `dax.broken_measure_ref`.
#[must_use]
pub(crate) fn lint_dax_expression_semantic(
    rel_path: &str,
    location: &str,
    current_table: &str,
    expression: &str,
    schema: &ModelScopeSchema,
) -> Vec<VerifyFinding> {
    let mut findings: Vec<VerifyFinding> = Vec::new();
    let references = extract_dax_references(expression);

    for reference in &references.columns {
        match reference.table.as_deref() {
            Some(table) => {
                if schema.has_column(table, &reference.column) {
                    // Resolves to a real column — valid.
                } else if schema.has_table_measure(table, &reference.column)
                    || schema.measure_owner(&reference.column).is_some()
                {
                    findings.push(finding(
                        rel_path,
                        location,
                        "dax.qualified_measure",
                        format!(
                            "'{table}[{col}]' qualifies a measure with a table; measures are \
                             model-scoped — reference it unqualified as [{col}]",
                            col = reference.column
                        ),
                        Severity::Warning,
                    ));
                } else {
                    findings.push(finding(
                        rel_path,
                        location,
                        "dax.broken_column_ref",
                        format!(
                            "'{table}[{col}]' does not resolve to a column in the model scope",
                            col = reference.column
                        ),
                        Severity::Error,
                    ));
                }
            }
            None => {
                if schema.measure_owner(&reference.column).is_some() {
                    // Resolves to a measure — valid (measures are unqualified).
                } else if schema.has_column(current_table, &reference.column) {
                    findings.push(finding(
                        rel_path,
                        location,
                        "dax.unqualified_column",
                        format!(
                            "'[{col}]' references a column without a table qualifier; prefer \
                             {current_table}[{col}]",
                            col = reference.column
                        ),
                        Severity::Warning,
                    ));
                } else if schema.column_exists_anywhere(&reference.column) {
                    findings.push(finding(
                        rel_path,
                        location,
                        "dax.broken_column_ref",
                        format!(
                            "'[{col}]' does not resolve on {current_table}; the column exists on \
                             another table and must be qualified",
                            col = reference.column
                        ),
                        Severity::Error,
                    ));
                } else {
                    findings.push(finding(
                        rel_path,
                        location,
                        "dax.broken_measure_ref",
                        format!(
                            "'[{col}]' does not resolve to a measure or column in the model scope",
                            col = reference.column
                        ),
                        Severity::Error,
                    ));
                }
            }
        }
    }

    findings
}

/// Detect measure→measure dependency cycles across a model scope.
///
/// Builds a directed dependency graph over the scope's measures — an edge
/// `A → B` exists when measure `A`'s DAX expression references measure `B` — and
/// emits a [`Severity::Error`] `dax.measure_cycle` for every measure that
/// participates in a cycle (including a self-reference). The traversal is
/// cycle-safe: it marks visited nodes so self- and mutually-referential
/// fixtures terminate instead of looping forever.
#[must_use]
pub(crate) fn detect_measure_cycles(
    measures: &[ScopeMeasure],
    schema: &ModelScopeSchema,
) -> Vec<VerifyFinding> {
    // Adjacency: case-folded measure name → the case-folded measure names it
    // references. DAX identifiers are case-insensitive, so both keys and targets
    // are lowercased to keep `[TotalSales]` and a declared `Total Sales`/`totalsales`
    // on the same cycle-detection node.
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();
    for measure in measures {
        let references = extract_dax_references(&measure.expression);
        let mut targets: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for reference in &references.columns {
            // A reference resolves to a measure when it is unqualified and names
            // a known measure, or qualified and names a table-declared measure.
            let is_measure = match reference.table.as_deref() {
                Some(table) => schema.has_table_measure(table, &reference.column),
                None => schema.measure_owner(&reference.column).is_some(),
            };
            if is_measure {
                let target = reference.column.to_lowercase();
                if seen.insert(target.clone()) {
                    targets.push(target);
                }
            }
        }
        deps.entry(measure.name.to_lowercase())
            .or_default()
            .extend(targets);
    }

    let mut findings: Vec<VerifyFinding> = Vec::new();
    for measure in measures {
        if measure_on_cycle(&measure.name.to_lowercase(), &deps) {
            let location = format!("{}[{}] (measure)", measure.table, measure.name);
            findings.push(finding(
                &measure.rel_path,
                &location,
                "dax.measure_cycle",
                format!(
                    "measure '{}' participates in a measure→measure dependency cycle",
                    measure.name
                ),
                Severity::Error,
            ));
        }
    }
    findings
}

/// Whether `start` can reach itself by following measure dependencies.
///
/// Uses a visited set so the walk terminates on self- and mutually-referential
/// measures instead of recursing forever.
fn measure_on_cycle(start: &str, deps: &HashMap<String, Vec<String>>) -> bool {
    let mut stack: Vec<&str> = deps
        .get(start)
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect();
    let mut visited: HashSet<&str> = HashSet::new();
    while let Some(node) = stack.pop() {
        if node == start {
            return true;
        }
        if visited.insert(node) {
            if let Some(next) = deps.get(node) {
                stack.extend(next.iter().map(String::as_str));
            }
        }
    }
    false
}

/// Error raised while linting the indexed Power BI models in a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintError {
    /// A caller-supplied `model_path` did not resolve to any indexed Power BI
    /// model scope in the bound workspace.
    ModelPathNotIndexed(String),
    /// An indexed `.tmdl` file in an active model could not be read or decoded
    /// as UTF-8. Surfaced (with the file path) rather than silently skipped, so
    /// a partially-examined model scope never reports a false `conformant: true`.
    FileUnreadable {
        /// Workspace-relative (best-effort) path of the offending file.
        path: String,
        /// Human-readable failure reason.
        reason: String,
    },
}

/// Aggregated per-scope state collected while walking the indexed `.tmdl` files.
#[derive(Default)]
struct ScopeState {
    schema: ModelScopeSchema,
    files: Vec<(String, PowerBiSemanticModel)>,
}

/// Lint the DAX in every indexed Power BI model under `source_paths`, resolving
/// references against a model-scope-aggregated schema rebuilt from the CURRENT
/// on-disk state of every sibling `.tmdl` file.
///
/// `workspace_root` is the bound workspace directory; `source_paths` are the
/// registry `powerbi` content-source paths (relative to the workspace root);
/// `max_file_size` is the registry's per-file byte limit. A discovered `.tmdl`
/// that resolves (via `canonicalize`) outside the canonical workspace root — for
/// example through a symlink inside an active source — is skipped before it is
/// read, and a file larger than `max_file_size` is skipped with the same
/// eligibility check the indexer applies.
/// When `model_path_filter` is `Some`, it is canonicalised via
/// `canonical_tmdl_model_path` to a single model scope and only that scope is
/// linted; a filter that matches no indexed scope yields
/// [`LintError::ModelPathNotIndexed`]. An indexed `.tmdl` file that cannot be
/// read or decoded yields [`LintError::FileUnreadable`] rather than a silent
/// skip.
///
/// Because the schema is rebuilt by reparsing the files at lint time, a sibling
/// column add/rename/delete is reflected immediately — a stale reference in an
/// unchanged sibling fires `dax.broken_column_ref`, and a newly-added column
/// clears a previously-broken reference — independent of which files an
/// incremental index pass reprocessed.
pub fn lint_indexed_models(
    workspace_root: &Path,
    source_paths: &[String],
    max_file_size: u64,
    model_path_filter: Option<&str>,
) -> Result<VerifyReport, LintError> {
    // Canonical workspace root for the symlink-containment guard below. When the
    // root cannot be canonicalised the guard is skipped (there is nothing safe to
    // compare against), which matches the pre-existing lenient behaviour.
    let canonical_root = workspace_root.canonicalize().ok();

    // Group every indexed `.tmdl` file by its canonical model scope, unioning
    // each parsed model into that scope's aggregated schema (BTreeMap keeps the
    // findings order deterministic across scopes).
    let mut scopes: BTreeMap<String, ScopeState> = BTreeMap::new();
    for source_path in source_paths {
        let source_dir = workspace_root.join(source_path);
        if !source_dir.is_dir() {
            continue;
        }
        for file_path in collect_powerbi_files_in_workspace(&source_dir, workspace_root) {
            let is_tmdl = file_path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("tmdl"));
            if !is_tmdl {
                continue;
            }
            // Workspace-containment guard: a symlink inside an active source
            // could point a discovered `.tmdl` outside the workspace. Skip
            // anything that does not canonicalise within the workspace root
            // before reading it, honouring the workspace-containment contract.
            if let Some(root) = canonical_root.as_ref() {
                match file_path.canonicalize() {
                    Ok(canon) if canon.starts_with(root) => {}
                    _ => continue,
                }
            }
            // Oversized files are skipped with the same eligibility check as
            // indexing, so an unindexed oversized sibling never influences the
            // lint and this blocking worker never reads a huge file.
            if let Ok(meta) = file_path.metadata() {
                if meta.len() > max_file_size {
                    continue;
                }
            }
            let rel_path = file_path
                .strip_prefix(workspace_root)
                .unwrap_or(&file_path)
                .to_string_lossy()
                .replace('\\', "/");
            // A read or UTF-8 failure on an active model file is surfaced (with
            // the path) rather than skipped: silently dropping it could leave a
            // whole-workspace lint reporting `conformant: true` while an active
            // model went unexamined.
            let content_bytes =
                std::fs::read(&file_path).map_err(|e| LintError::FileUnreadable {
                    path: rel_path.clone(),
                    reason: e.to_string(),
                })?;
            let content =
                std::str::from_utf8(&content_bytes).map_err(|e| LintError::FileUnreadable {
                    path: rel_path.clone(),
                    reason: e.to_string(),
                })?;
            let Some(model) = extract_tmdl_semantic_model(content, &rel_path) else {
                continue;
            };
            let scope = canonical_tmdl_model_path(&rel_path);
            let entry = scopes.entry(scope).or_default();
            entry.schema.add_model(&model);
            entry.files.push((rel_path, model));
        }
    }

    let selected_scope = model_path_filter.map(canonical_tmdl_model_path);
    if let Some(want) = selected_scope.as_deref() {
        if !scopes.contains_key(want) {
            return Err(LintError::ModelPathNotIndexed(
                model_path_filter.unwrap_or_default().to_owned(),
            ));
        }
    }

    let mut findings: Vec<VerifyFinding> = Vec::new();
    for (scope_key, state) in &scopes {
        if let Some(want) = selected_scope.as_deref() {
            if scope_key != want {
                continue;
            }
        }
        let mut scope_measures: Vec<ScopeMeasure> = Vec::new();
        for (rel_path, model) in &state.files {
            findings.extend(lint_model_dax(rel_path, model, &state.schema));
            for table in &model.tables {
                for measure in &table.measures {
                    if let Some(expression) = measure.expression.as_deref() {
                        scope_measures.push(ScopeMeasure {
                            rel_path: rel_path.clone(),
                            table: table.name.clone(),
                            name: measure.name.clone(),
                            expression: expression.to_owned(),
                        });
                    }
                }
            }
        }
        findings.extend(detect_measure_cycles(&scope_measures, &state.schema));
    }

    Ok(VerifyReport::from_findings(findings))
}

#[cfg(test)]
mod tests {
    use super::contains_bare_division;

    #[test]
    fn bare_division_detects_real_operator() {
        assert!(contains_bare_division("[Amount] / [Count]"));
        assert!(contains_bare_division("DIVIDE([A], [B]) + [x] / 2"));
    }

    #[test]
    fn bare_division_ignores_slash_in_string_and_comment() {
        assert!(!contains_bare_division(r#""a / b""#));
        assert!(!contains_bare_division("[Amount] // ratio a / b"));
        assert!(!contains_bare_division("/* a / b */ [Amount]"));
        assert!(!contains_bare_division("'Table / Name'[Col]"));
    }

    #[test]
    fn bare_division_ignores_escaped_bracket_with_slash() {
        // A column name containing an escaped `]]` and a `/` must not expose the
        // slash to the scanner (regression for the cycle-3 review finding).
        assert!(!contains_bare_division("Sales[Path ]] / USD]"));
        assert!(!contains_bare_division("[Rate ]] / bps]"));
        // A real division that follows an escaped-bracket reference is still seen.
        assert!(contains_bare_division("Sales[Path ]] USD] / [Count]"));
    }

    #[test]
    fn bare_division_ignores_escaped_quote_with_slash() {
        // `""` is an escaped quote inside a string literal; the `/` between the
        // escaped quote and the string end must stay hidden.
        assert!(!contains_bare_division(r#""a "" / b""#));
    }
}
