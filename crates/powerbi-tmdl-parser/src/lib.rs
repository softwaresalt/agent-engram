//! Power BI-oriented TMDL parsing primitives.
//!
//! This crate provides a small, dependency-light parser boundary for TMDL
//! documents so Engram can consume semantic-model structure through a dedicated
//! crate interface rather than hard-coding all parsing inside the main daemon.
//! The current implementation is a fixture-driven line/indent parser. A future
//! tree-sitter-backed implementation can land behind the same public API.

#![forbid(unsafe_code)]

/// A parsed TMDL semantic-model document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TmdlModel {
    /// Explicit model name from `model ...`, if present.
    pub model_name: Option<String>,
    /// Tables declared in the document.
    pub tables: Vec<TmdlTable>,
    /// Relationships declared in the document.
    pub relationships: Vec<TmdlRelationship>,
    /// Top-level expressions declared in the document.
    pub expressions: Vec<TmdlExpression>,
    /// Data sources declared in the document.
    pub data_sources: Vec<TmdlDataSource>,
    /// `ref` statements declared at model scope (e.g. `ref table Sales`).
    pub refs: Vec<TmdlRef>,
    /// Annotations attached at model scope.
    pub annotations: Vec<TmdlAnnotation>,
    /// Model `culture:` metadata (e.g. `en-US`).
    pub culture: Option<String>,
    /// Model `defaultMode:` metadata (e.g. `import`, `directQuery`).
    pub default_mode: Option<String>,
    /// Model `lineageTag:` metadata.
    pub lineage_tag: Option<String>,
}

/// A parsed TMDL table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TmdlTable {
    /// Table name.
    pub name: String,
    /// Columns declared on the table.
    pub columns: Vec<TmdlColumn>,
    /// Measures declared on the table.
    pub measures: Vec<TmdlMeasure>,
    /// Partitions declared on the table.
    pub partitions: Vec<TmdlPartition>,
    /// Annotations attached at table scope.
    pub annotations: Vec<TmdlAnnotation>,
    /// Table `lineageTag:` metadata.
    pub lineage_tag: Option<String>,
}

/// A parsed TMDL partition.
///
/// Partitions bind a table to a physical load definition. The `= <kind>` token
/// after the partition name records the source kind (for example `m` for a
/// Power Query / M partition), `mode:` records the storage mode, and the
/// triple-backtick-fenced `source =` payload is captured as an opaque M body
/// that the parser never evaluates. The body is lightly normalized: each line
/// is trimmed and blank lines are dropped, then the remaining lines are joined
/// with `\n` (original indentation and blank-line spacing are not preserved).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TmdlPartition {
    /// Partition name.
    pub name: String,
    /// Source kind token following `= ` on the declaration line (e.g. `m`).
    pub source_kind: Option<String>,
    /// Optional `mode:` property (e.g. `import`, `directQuery`).
    pub mode: Option<String>,
    /// Opaque embedded source body (typically an M expression). Lightly
    /// normalized: each captured line is trimmed and blank lines are dropped
    /// before joining with `\n`, so exact original whitespace is not retained.
    pub source_expression: Option<String>,
}

/// A parsed TMDL column.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TmdlColumn {
    /// Column name.
    pub name: String,
    /// Optional `dataType:` property.
    pub data_type: Option<String>,
    /// Annotations attached at column scope.
    pub annotations: Vec<TmdlAnnotation>,
    /// Column `lineageTag:` metadata.
    pub lineage_tag: Option<String>,
}

/// A parsed TMDL measure.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TmdlMeasure {
    /// Measure name.
    pub name: String,
    /// Measure expression text.
    pub expression: Option<String>,
    /// Annotations attached at measure scope.
    pub annotations: Vec<TmdlAnnotation>,
    /// Measure `lineageTag:` metadata.
    pub lineage_tag: Option<String>,
}

/// A parsed TMDL relationship.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TmdlRelationship {
    /// Relationship name.
    pub name: String,
    /// Source table.
    pub from_table: String,
    /// Source column.
    pub from_column: String,
    /// Target table.
    pub to_table: String,
    /// Target column.
    pub to_column: String,
}

/// A parsed top-level TMDL expression.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TmdlExpression {
    /// Expression name.
    pub name: String,
    /// Expression body text.
    pub expression: Option<String>,
}

/// A parsed TMDL data source.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TmdlDataSource {
    /// Data source name.
    pub name: String,
    /// Data source `kind`/`type` token (e.g. `sql`, `structured`).
    pub kind: Option<String>,
    /// Underlying provider (e.g. `System.Data.SqlClient`).
    pub provider: Option<String>,
    /// Connection string, when declared inline.
    pub connection_string: Option<String>,
    /// Server / address the data source connects to.
    pub server: Option<String>,
    /// Database / catalog the data source targets.
    pub database: Option<String>,
}

/// A parsed TMDL `annotation <Name> = <Value>` statement.
///
/// Annotations are attached to the nearest enclosing object (model, table,
/// column, or measure) rather than surfaced as standalone graph nodes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TmdlAnnotation {
    /// Annotation name.
    pub name: String,
    /// Annotation value following `=`, when present.
    pub value: Option<String>,
}

/// A parsed TMDL `ref <kind> <name>` statement, recorded at model scope.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TmdlRef {
    /// Referenced object kind (e.g. `table`, `cultureInfo`, `relationship`).
    pub kind: String,
    /// Referenced object name.
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TmdlMemberKind {
    Column(usize),
    Measure(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TmdlTableDraft {
    name: String,
    columns: Vec<TmdlColumn>,
    measures: Vec<TmdlMeasure>,
    partitions: Vec<TmdlPartition>,
    last_member: Option<TmdlMemberKind>,
    /// Indent width of the current open column/measure member, used to scope
    /// `annotation`/`lineageTag:` lines to the member versus the table.
    member_indent: Option<usize>,
    /// Indent width at which an unmodeled member-level block (e.g. `hierarchy`,
    /// `level`, `role`) opened. While set, deeper-indented `annotation`/
    /// `lineageTag:` lines are dropped rather than mis-attached to the preceding
    /// column/measure or the table.
    skip_below_indent: Option<usize>,
    /// Annotations attached at table scope.
    annotations: Vec<TmdlAnnotation>,
    /// Table `lineageTag:` metadata.
    lineage_tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingMeasureBody {
    indent: usize,
    measure_index: usize,
    lines: Vec<String>,
}

/// Capture state for a partition block that is still being read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PendingPartition {
    /// Indent width of the `partition` declaration line.
    indent: usize,
    /// Index of the partition inside the current table draft.
    partition_index: usize,
    /// Whether a triple-backtick-fenced source body is currently open.
    fence_open: bool,
    /// Captured (trimmed) source body lines.
    source_lines: Vec<String>,
}

/// Capture state for a `dataSource` block that is still being read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PendingDataSource {
    /// Indent width of the `dataSource` declaration line.
    indent: usize,
    /// Index of the data source inside the parse state.
    index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct PendingRelationship {
    indent: usize,
    name: String,
    from: Option<(String, String)>,
    to: Option<(String, String)>,
}

#[derive(Debug, Default)]
struct ParseState {
    model_name: Option<String>,
    tables: Vec<TmdlTable>,
    current_table: Option<TmdlTableDraft>,
    relationships: Vec<TmdlRelationship>,
    expressions: Vec<TmdlExpression>,
    pending_relationship: Option<PendingRelationship>,
    data_sources: Vec<TmdlDataSource>,
    pending_measure_body: Option<PendingMeasureBody>,
    pending_partition: Option<PendingPartition>,
    pending_data_source: Option<PendingDataSource>,
    model_refs: Vec<TmdlRef>,
    model_annotations: Vec<TmdlAnnotation>,
    model_culture: Option<String>,
    model_default_mode: Option<String>,
    model_lineage_tag: Option<String>,
}

/// Parse a TMDL document into semantic-model objects.
///
/// Returns `None` when the input does not contain any supported TMDL objects.
#[must_use]
pub fn parse_tmdl_document(source: &str) -> Option<TmdlModel> {
    let mut state = ParseState::default();

    for raw_line in source.lines() {
        // While a fenced partition source body is open, capture lines verbatim
        // (trimmed of surrounding whitespace) until the closing fence so that M
        // content is never misinterpreted as TMDL declarations or properties.
        if state
            .pending_partition
            .as_ref()
            .is_some_and(|partition| partition.fence_open)
        {
            capture_partition_source_line(&mut state.pending_partition, raw_line);
            continue;
        }

        let line = raw_line.trim_end();
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        let indent = leading_indent_width(line);

        prepare_pending_state(&mut state, indent, trimmed);

        if !capture_pending_state(&mut state, indent, trimmed) {
            let _ = handle_declaration(&mut state, indent, trimmed);
        }
    }

    finalize_parse_state(state)
}

fn leading_indent_width(line: &str) -> usize {
    line.len() - line.trim_start_matches([' ', '\t']).len()
}

fn prepare_pending_state(state: &mut ParseState, indent: usize, trimmed: &str) {
    if should_finish_measure_capture(state.pending_measure_body.as_ref(), indent, trimmed) {
        finish_pending_measure_body(&mut state.current_table, &mut state.pending_measure_body);
    }

    if should_finish_relationship(state.pending_relationship.as_ref(), indent) {
        finish_pending_relationship(&mut state.relationships, &mut state.pending_relationship);
    }

    if should_finish_partition(state.pending_partition.as_ref(), indent) {
        finish_pending_partition(&mut state.current_table, &mut state.pending_partition);
    }

    if should_finish_data_source(state.pending_data_source.as_ref(), indent) {
        state.pending_data_source = None;
    }

    // Close an unmodeled member-block skip window once the parser dedents back to
    // or above the indent where the block opened.
    if let Some(table) = state.current_table.as_mut() {
        if table
            .skip_below_indent
            .is_some_and(|opened| indent <= opened)
        {
            table.skip_below_indent = None;
        }
    }
}

fn capture_pending_state(state: &mut ParseState, indent: usize, trimmed: &str) -> bool {
    capture_measure_body_line(
        &mut state.current_table,
        &mut state.pending_measure_body,
        indent,
        trimmed,
    ) || capture_relationship_property(&mut state.pending_relationship, indent, trimmed)
        || capture_partition_property(
            &mut state.current_table,
            &mut state.pending_partition,
            indent,
            trimmed,
        )
        || capture_data_source_property(
            &mut state.data_sources,
            state.pending_data_source.as_ref(),
            indent,
            trimmed,
        )
}

/// Open a new `table` declaration, finishing any pending measure/relationship/
/// partition blocks and flushing the previous table draft first.
fn start_table(state: &mut ParseState, rest: &str) {
    finish_pending_measure_body(&mut state.current_table, &mut state.pending_measure_body);
    finish_pending_relationship(&mut state.relationships, &mut state.pending_relationship);
    finish_pending_partition(&mut state.current_table, &mut state.pending_partition);
    flush_table(&mut state.tables, &mut state.current_table);
    state.current_table = Some(TmdlTableDraft {
        name: parse_identifier(rest),
        columns: Vec::new(),
        measures: Vec::new(),
        partitions: Vec::new(),
        last_member: None,
        member_indent: None,
        skip_below_indent: None,
        annotations: Vec::new(),
        lineage_tag: None,
    });
}

fn handle_declaration(state: &mut ParseState, indent: usize, trimmed: &str) -> bool {
    if let Some(rest) = trimmed.strip_prefix("model ") {
        state.model_name = Some(parse_identifier(rest));
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("table ") {
        start_table(state, rest);
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("column ") {
        return start_column(state.current_table.as_mut(), indent, rest);
    }

    if let Some(rest) = trimmed.strip_prefix("measure ") {
        return start_measure(
            state.current_table.as_mut(),
            &mut state.pending_measure_body,
            indent,
            rest,
        );
    }

    if let Some(rest) = trimmed.strip_prefix("relationship ") {
        finish_pending_measure_body(&mut state.current_table, &mut state.pending_measure_body);
        finish_pending_relationship(&mut state.relationships, &mut state.pending_relationship);
        if let Some(relationship) = parse_inline_relationship(rest) {
            state.relationships.push(relationship);
        } else {
            state.pending_relationship = Some(PendingRelationship {
                indent,
                name: parse_identifier(rest),
                from: None,
                to: None,
            });
        }
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("expression ") {
        state.expressions.push(parse_expression_declaration(rest));
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("dataType:") {
        return set_column_data_type(state.current_table.as_mut(), rest);
    }

    if let Some(rest) = trimmed.strip_prefix("expression:") {
        return set_measure_expression(state.current_table.as_mut(), rest);
    }

    if let Some(rest) = trimmed.strip_prefix("partition ") {
        return start_partition(
            &mut state.current_table,
            &mut state.pending_partition,
            indent,
            rest,
        );
    }

    if let Some(rest) = trimmed.strip_prefix("dataSource ") {
        state.data_sources.push(TmdlDataSource {
            name: parse_identifier(rest),
            ..TmdlDataSource::default()
        });
        state.pending_data_source = Some(PendingDataSource {
            indent,
            index: state.data_sources.len() - 1,
        });
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("ref ") {
        if let Some(reference) = parse_ref_declaration(rest) {
            state.model_refs.push(reference);
        }
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("annotation ") {
        attach_annotation(state, indent, parse_annotation_declaration(rest));
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("lineageTag:") {
        attach_lineage_tag(state, indent, parse_identifier(rest));
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("culture:") {
        state.model_culture = Some(parse_identifier(rest));
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("defaultMode:") {
        state.model_default_mode = Some(parse_identifier(rest));
        return true;
    }

    // Unmodeled declaration-boundary keywords (`hierarchy`, `level`, `role`,
    // `function`, `culture` block form, ...) open blocks this parser does not
    // model. Inside a table, opening one clears column/measure member scope and
    // marks a skip window so a stray nested `lineageTag:`/`annotation` is dropped
    // rather than mis-attached to (and overwriting) the preceding column/measure.
    if is_declaration_line(trimmed) {
        enter_unmodeled_member_block(state, indent);
        return true;
    }

    false
}

/// Handle an `is_declaration_line` keyword this parser does not model
/// (`hierarchy`, `level`, `role`, ...) encountered inside a table.
///
/// Clears the current column/measure member scope and opens (or widens) a skip
/// window so nested `lineageTag:`/`annotation` lines are dropped rather than
/// mis-attributed to the preceding member.
fn enter_unmodeled_member_block(state: &mut ParseState, indent: usize) {
    if let Some(table) = state.current_table.as_mut() {
        table.last_member = None;
        table.member_indent = None;
        let widen = table
            .skip_below_indent
            .is_none_or(|existing| indent <= existing);
        if widen {
            table.skip_below_indent = Some(indent);
        }
    }
}

fn finalize_parse_state(mut state: ParseState) -> Option<TmdlModel> {
    finish_pending_measure_body(&mut state.current_table, &mut state.pending_measure_body);
    finish_pending_relationship(&mut state.relationships, &mut state.pending_relationship);
    finish_pending_partition(&mut state.current_table, &mut state.pending_partition);
    flush_table(&mut state.tables, &mut state.current_table);

    if state.model_name.is_none()
        && state.tables.is_empty()
        && state.relationships.is_empty()
        && state.expressions.is_empty()
        && state.data_sources.is_empty()
    {
        return None;
    }

    Some(TmdlModel {
        model_name: state.model_name,
        tables: state.tables,
        relationships: state.relationships,
        expressions: state.expressions,
        data_sources: state.data_sources,
        refs: state.model_refs,
        annotations: state.model_annotations,
        culture: state.model_culture,
        default_mode: state.model_default_mode,
        lineage_tag: state.model_lineage_tag,
    })
}

fn start_column(current_table: Option<&mut TmdlTableDraft>, indent: usize, rest: &str) -> bool {
    let Some(table) = current_table else {
        return false;
    };

    table.columns.push(TmdlColumn {
        name: parse_identifier(rest),
        data_type: None,
        ..TmdlColumn::default()
    });
    table.last_member = Some(TmdlMemberKind::Column(table.columns.len() - 1));
    table.member_indent = Some(indent);
    true
}

fn start_measure(
    current_table: Option<&mut TmdlTableDraft>,
    pending_measure_body: &mut Option<PendingMeasureBody>,
    indent: usize,
    rest: &str,
) -> bool {
    let Some(table) = current_table else {
        return false;
    };

    let (name, expression) = parse_measure_declaration(rest);
    table.measures.push(TmdlMeasure {
        name,
        expression,
        ..TmdlMeasure::default()
    });
    let measure_index = table.measures.len() - 1;
    table.last_member = Some(TmdlMemberKind::Measure(measure_index));
    table.member_indent = Some(indent);
    if table.measures[measure_index].expression.is_none() {
        *pending_measure_body = Some(PendingMeasureBody {
            indent,
            measure_index,
            lines: Vec::new(),
        });
    }

    true
}

fn start_partition(
    current_table: &mut Option<TmdlTableDraft>,
    pending_partition: &mut Option<PendingPartition>,
    indent: usize,
    rest: &str,
) -> bool {
    // Close any partition block that was still open before starting a new one.
    finish_pending_partition(current_table, pending_partition);

    let Some(table) = current_table.as_mut() else {
        return false;
    };

    let (name, source_kind) = parse_partition_declaration(rest);
    table.partitions.push(TmdlPartition {
        name,
        source_kind,
        mode: None,
        source_expression: None,
    });
    let partition_index = table.partitions.len() - 1;
    // A partition is not a column/measure member; clear the last member so a
    // stray `dataType:`/`expression:` cannot attach to a prior column/measure.
    table.last_member = None;
    table.member_indent = None;
    *pending_partition = Some(PendingPartition {
        indent,
        partition_index,
        fence_open: false,
        source_lines: Vec::new(),
    });

    true
}

fn set_column_data_type(current_table: Option<&mut TmdlTableDraft>, rest: &str) -> bool {
    let Some(table) = current_table else {
        return false;
    };

    if let Some(TmdlMemberKind::Column(index)) = table.last_member {
        table.columns[index].data_type = Some(parse_identifier(rest));
    }

    true
}

fn set_measure_expression(current_table: Option<&mut TmdlTableDraft>, rest: &str) -> bool {
    let Some(table) = current_table else {
        return false;
    };

    if let Some(TmdlMemberKind::Measure(index)) = table.last_member {
        table.measures[index].expression = Some(rest.trim().to_string());
    }

    true
}

fn should_finish_measure_capture(
    pending: Option<&PendingMeasureBody>,
    indent: usize,
    trimmed: &str,
) -> bool {
    pending.is_some_and(|body| {
        indent <= body.indent || looks_like_tmdl_property(trimmed) || is_declaration_line(trimmed)
    })
}

fn should_finish_relationship(pending: Option<&PendingRelationship>, indent: usize) -> bool {
    pending.is_some_and(|relationship| indent <= relationship.indent)
}

fn should_finish_partition(pending: Option<&PendingPartition>, indent: usize) -> bool {
    pending.is_some_and(|partition| indent <= partition.indent)
}

fn capture_measure_body_line(
    current_table: &mut Option<TmdlTableDraft>,
    pending: &mut Option<PendingMeasureBody>,
    indent: usize,
    trimmed: &str,
) -> bool {
    let Some(body) = pending.as_mut() else {
        return false;
    };

    if indent <= body.indent || looks_like_tmdl_property(trimmed) || is_declaration_line(trimmed) {
        return false;
    }

    let Some(table) = current_table.as_mut() else {
        return false;
    };

    table.last_member = Some(TmdlMemberKind::Measure(body.measure_index));
    body.lines.push(trimmed.to_string());
    true
}

fn capture_relationship_property(
    pending: &mut Option<PendingRelationship>,
    indent: usize,
    trimmed: &str,
) -> bool {
    let Some(relationship) = pending.as_mut() else {
        return false;
    };

    if indent <= relationship.indent {
        return false;
    }

    if let Some(rest) = trimmed.strip_prefix("fromColumn:") {
        relationship.from = parse_table_column_value(rest);
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("toColumn:") {
        relationship.to = parse_table_column_value(rest);
        return true;
    }

    false
}

/// Capture a property line inside an open partition block.
///
/// Recognizes `mode:` and `source =`, and treats any other deeper-indented line
/// as opaque block content that belongs to the partition (so it does not fall
/// through to declaration handling and prematurely end the block).
fn capture_partition_property(
    current_table: &mut Option<TmdlTableDraft>,
    pending: &mut Option<PendingPartition>,
    indent: usize,
    trimmed: &str,
) -> bool {
    let Some(partition_pending) = pending.as_mut() else {
        return false;
    };

    if indent <= partition_pending.indent {
        return false;
    }

    let Some(table) = current_table.as_mut() else {
        return false;
    };
    let partition = &mut table.partitions[partition_pending.partition_index];

    if let Some(rest) = trimmed.strip_prefix("mode:") {
        partition.mode = Some(parse_identifier(rest));
        return true;
    }

    if let Some(rest) = trimmed
        .strip_prefix("source =")
        .or_else(|| trimmed.strip_prefix("source="))
    {
        begin_partition_source(partition_pending, partition, rest);
        return true;
    }

    // Deeper-indented content inside the partition block is consumed opaquely.
    true
}

/// Begin reading a partition `source` payload.
///
/// A triple-backtick-fenced value opens a fenced multi-line body; any other
/// non-empty value is treated as an inline single-line source expression.
fn begin_partition_source(
    pending: &mut PendingPartition,
    partition: &mut TmdlPartition,
    rest: &str,
) {
    let value = rest.trim();

    if let Some(after_fence) = value.strip_prefix("```") {
        pending.fence_open = true;
        let after_fence = after_fence.trim();
        if !after_fence.is_empty() && after_fence != "```" {
            pending.source_lines.push(after_fence.to_string());
        }
        return;
    }

    if !value.is_empty() {
        partition.source_expression = Some(value.to_string());
    }
}

/// Capture one raw line of a fenced partition source body.
///
/// The closing triple-backtick fence ends capture; blank lines are dropped and
/// all other lines are stored trimmed of surrounding whitespace.
fn capture_partition_source_line(pending: &mut Option<PendingPartition>, raw_line: &str) {
    let Some(partition_pending) = pending.as_mut() else {
        return;
    };

    let trimmed = raw_line.trim();
    if trimmed == "```" {
        partition_pending.fence_open = false;
    } else if !trimmed.is_empty() {
        partition_pending.source_lines.push(trimmed.to_string());
    }
}

fn finish_pending_partition(
    current_table: &mut Option<TmdlTableDraft>,
    pending: &mut Option<PendingPartition>,
) {
    let Some(partition_pending) = pending.take() else {
        return;
    };

    if partition_pending.source_lines.is_empty() {
        return;
    }

    let Some(table) = current_table.as_mut() else {
        return;
    };

    let partition = &mut table.partitions[partition_pending.partition_index];
    if partition.source_expression.is_none() {
        partition.source_expression = Some(partition_pending.source_lines.join("\n"));
    }
}

fn should_finish_data_source(pending: Option<&PendingDataSource>, indent: usize) -> bool {
    pending.is_some_and(|data_source| indent <= data_source.indent)
}

/// Capture a property line inside an open `dataSource` block.
///
/// Recognizes the connection-oriented keys and consumes any other deeper line
/// opaquely so it does not fall through to declaration handling and end the
/// block prematurely.
fn capture_data_source_property(
    data_sources: &mut [TmdlDataSource],
    pending: Option<&PendingDataSource>,
    indent: usize,
    trimmed: &str,
) -> bool {
    let Some(ds_pending) = pending else {
        return false;
    };

    if indent <= ds_pending.indent {
        return false;
    }

    let Some(data_source) = data_sources.get_mut(ds_pending.index) else {
        return false;
    };

    if let Some(rest) = trimmed.strip_prefix("kind:") {
        data_source.kind = Some(parse_identifier(rest));
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("type:") {
        if data_source.kind.is_none() {
            data_source.kind = Some(parse_identifier(rest));
        }
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("provider:") {
        data_source.provider = Some(parse_identifier(rest));
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("connectionString:") {
        data_source.connection_string = Some(parse_identifier(rest));
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("server:") {
        data_source.server = Some(parse_identifier(rest));
        return true;
    }

    if let Some(rest) = trimmed.strip_prefix("database:") {
        data_source.database = Some(parse_identifier(rest));
        return true;
    }

    // Deeper-indented content inside the data source block is consumed opaquely.
    true
}

fn finish_pending_measure_body(
    current_table: &mut Option<TmdlTableDraft>,
    pending: &mut Option<PendingMeasureBody>,
) {
    let Some(body) = pending.take() else {
        return;
    };

    let Some(table) = current_table.as_mut() else {
        return;
    };

    if body.lines.is_empty() {
        return;
    }

    let expression = body.lines.join("\n");
    table.measures[body.measure_index].expression = Some(expression);
}

fn finish_pending_relationship(
    relationships: &mut Vec<TmdlRelationship>,
    pending: &mut Option<PendingRelationship>,
) {
    let Some(relationship) = pending.take() else {
        return;
    };

    let (Some((from_table, from_column)), Some((to_table, to_column))) =
        (relationship.from, relationship.to)
    else {
        return;
    };

    relationships.push(TmdlRelationship {
        name: relationship.name,
        from_table,
        from_column,
        to_table,
        to_column,
    });
}

fn flush_table(tables: &mut Vec<TmdlTable>, current_table: &mut Option<TmdlTableDraft>) {
    let Some(table) = current_table.take() else {
        return;
    };

    tables.push(TmdlTable {
        name: table.name,
        columns: table.columns,
        measures: table.measures,
        partitions: table.partitions,
        annotations: table.annotations,
        lineage_tag: table.lineage_tag,
    });
}

fn parse_measure_declaration(rest: &str) -> (String, Option<String>) {
    let mut parts = rest.splitn(2, '=');
    let name = parse_identifier(parts.next().unwrap_or_default());
    let expression = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    (name, expression)
}

fn parse_partition_declaration(rest: &str) -> (String, Option<String>) {
    let mut parts = rest.splitn(2, '=');
    let name = parse_identifier(parts.next().unwrap_or_default());
    let source_kind = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    (name, source_kind)
}

/// The object an `annotation`/`lineageTag:` line attaches to, resolved by the
/// current table draft state and the line indent.
enum MetadataTarget {
    Model,
    Table,
    Column(usize),
    Measure(usize),
    /// The line falls inside an unmodeled member block (e.g. a `hierarchy`) and
    /// must be dropped rather than attached to any modeled object.
    Skip,
}

/// Resolve which object a scope-sensitive metadata line (`annotation` or
/// `lineageTag:`) attaches to.
///
/// When no table is open the target is the model. A line indented deeper than an
/// open unmodeled member block is skipped. Otherwise a line indented deeper than
/// the current column/measure member attaches to that member; failing that it
/// attaches to the table itself.
fn resolve_metadata_target(
    current_table: Option<&TmdlTableDraft>,
    indent: usize,
) -> MetadataTarget {
    let Some(table) = current_table else {
        return MetadataTarget::Model;
    };

    if table
        .skip_below_indent
        .is_some_and(|opened| indent > opened)
    {
        return MetadataTarget::Skip;
    }

    if let (Some(member), Some(member_indent)) = (table.last_member.as_ref(), table.member_indent) {
        if indent > member_indent {
            return match member {
                TmdlMemberKind::Column(index) => MetadataTarget::Column(*index),
                TmdlMemberKind::Measure(index) => MetadataTarget::Measure(*index),
            };
        }
    }

    MetadataTarget::Table
}

/// Attach an annotation to the object resolved for `indent`.
fn attach_annotation(state: &mut ParseState, indent: usize, annotation: TmdlAnnotation) {
    match resolve_metadata_target(state.current_table.as_ref(), indent) {
        MetadataTarget::Skip => {}
        MetadataTarget::Model => state.model_annotations.push(annotation),
        MetadataTarget::Table => {
            if let Some(table) = state.current_table.as_mut() {
                table.annotations.push(annotation);
            }
        }
        MetadataTarget::Column(index) => {
            if let Some(column) = state
                .current_table
                .as_mut()
                .and_then(|table| table.columns.get_mut(index))
            {
                column.annotations.push(annotation);
            }
        }
        MetadataTarget::Measure(index) => {
            if let Some(measure) = state
                .current_table
                .as_mut()
                .and_then(|table| table.measures.get_mut(index))
            {
                measure.annotations.push(annotation);
            }
        }
    }
}

/// Attach a `lineageTag:` value to the object resolved for `indent`.
fn attach_lineage_tag(state: &mut ParseState, indent: usize, value: String) {
    match resolve_metadata_target(state.current_table.as_ref(), indent) {
        MetadataTarget::Skip => {}
        MetadataTarget::Model => state.model_lineage_tag = Some(value),
        MetadataTarget::Table => {
            if let Some(table) = state.current_table.as_mut() {
                table.lineage_tag = Some(value);
            }
        }
        MetadataTarget::Column(index) => {
            if let Some(column) = state
                .current_table
                .as_mut()
                .and_then(|table| table.columns.get_mut(index))
            {
                column.lineage_tag = Some(value);
            }
        }
        MetadataTarget::Measure(index) => {
            if let Some(measure) = state
                .current_table
                .as_mut()
                .and_then(|table| table.measures.get_mut(index))
            {
                measure.lineage_tag = Some(value);
            }
        }
    }
}

/// Parse a `ref <kind> <name>` statement into a [`TmdlRef`].
///
/// Returns `None` when the statement lacks a kind or name token.
fn parse_ref_declaration(rest: &str) -> Option<TmdlRef> {
    let (kind, name) = rest.trim().split_once(char::is_whitespace)?;
    let name = parse_identifier(name);
    if name.is_empty() {
        return None;
    }

    Some(TmdlRef {
        kind: kind.trim().to_string(),
        name,
    })
}

/// Parse an `annotation <Name> = <Value>` statement into a [`TmdlAnnotation`].
fn parse_annotation_declaration(rest: &str) -> TmdlAnnotation {
    let mut parts = rest.splitn(2, '=');
    let name = parse_identifier(parts.next().unwrap_or_default());
    let value = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    TmdlAnnotation { name, value }
}

fn parse_expression_declaration(rest: &str) -> TmdlExpression {
    let mut parts = rest.splitn(2, '=');
    let name = parse_identifier(parts.next().unwrap_or_default());
    let expression = parts
        .next()
        .map(str::trim)
        .map(strip_trailing_meta_clause)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    TmdlExpression { name, expression }
}

fn parse_inline_relationship(rest: &str) -> Option<TmdlRelationship> {
    let mut parts = rest.splitn(2, "->");
    let lhs = parts.next()?.trim();
    let rhs = parts.next()?.trim();
    let (from_table, from_column) = parse_table_column_value(lhs)?;
    let (to_table, to_column) = parse_table_column_value(rhs)?;

    Some(TmdlRelationship {
        name: format!("{from_table}.{from_column}->{to_table}.{to_column}"),
        from_table,
        from_column,
        to_table,
        to_column,
    })
}

fn parse_table_column_value(value: &str) -> Option<(String, String)> {
    let (table, column) = value.trim().split_once('.')?;
    Some((parse_identifier(table), parse_identifier(column)))
}

fn parse_identifier(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn strip_trailing_meta_clause(value: &str) -> &str {
    value
        .split_once(" meta [")
        .map_or(value, |(expression, _)| expression)
}

fn looks_like_tmdl_property(trimmed: &str) -> bool {
    trimmed.contains(':')
}

fn is_declaration_line(trimmed: &str) -> bool {
    [
        "annotation ",
        "column ",
        "culture ",
        "dataSource ",
        "expression ",
        "function ",
        "hierarchy ",
        "level ",
        "measure ",
        "model ",
        "partition ",
        "ref ",
        "relationship ",
        "role ",
        "table ",
    ]
    .into_iter()
    .any(|prefix| trimmed.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::parse_tmdl_document;

    #[test]
    fn parse_relationship_block() {
        let Some(model) = parse_tmdl_document(
            "
relationship FactToTitle
  fromColumn: FactVehicleRegistrations.VehicleTitleKey
  toColumn: DimVehicleTitle.VehicleTitleKey
",
        ) else {
            panic!("fixture should parse");
        };

        assert_eq!(model.relationships.len(), 1);
        let relationship = &model.relationships[0];
        assert_eq!(relationship.from_table, "FactVehicleRegistrations");
        assert_eq!(relationship.from_column, "VehicleTitleKey");
        assert_eq!(relationship.to_table, "DimVehicleTitle");
        assert_eq!(relationship.to_column, "VehicleTitleKey");
    }

    #[test]
    fn parse_multiline_measure_expression() {
        let Some(model) = parse_tmdl_document(
            "
table Sales
  measure 'Registrations With Lien Holder' =
    CALCULATE (
      [Total Registrations],
      FILTER ( Sales, Sales[HasLien] = TRUE () )
    )
    formatString: #,0
",
        ) else {
            panic!("fixture should parse");
        };

        assert_eq!(model.tables.len(), 1);
        assert_eq!(model.tables[0].measures.len(), 1);
        assert_eq!(
            model.tables[0].measures[0].expression.as_deref(),
            Some(
                "CALCULATE (\n[Total Registrations],\nFILTER ( Sales, Sales[HasLien] = TRUE () )\n)"
            )
        );
    }

    #[test]
    fn parse_top_level_expressions() {
        let Some(model) = parse_tmdl_document(
            r#"
expression SynapseSqlServer = "dp-da-synw-t-cus-01-ondemand.sql.azuresynapse.net" meta [IsParameterQuery=true, Type="Text"]

expression SynapseDatabase = "ILSOS_EDW" meta [IsParameterQuery=true, Type="Text"]
"#,
        ) else {
            panic!("fixture should parse");
        };

        assert_eq!(model.expressions.len(), 2);
        assert_eq!(model.expressions[0].name, "SynapseSqlServer");
        assert_eq!(
            model.expressions[0].expression.as_deref(),
            Some("\"dp-da-synw-t-cus-01-ondemand.sql.azuresynapse.net\"")
        );
        assert_eq!(model.expressions[1].name, "SynapseDatabase");
        assert_eq!(
            model.expressions[1].expression.as_deref(),
            Some("\"ILSOS_EDW\"")
        );
    }

    #[test]
    fn parse_partition_with_fenced_m_source() {
        let Some(model) = parse_tmdl_document(
            "
table FactVehicleRegistrations
  column Amount
    dataType: double
  partition FactVehicleRegistrations = m
    mode: import
    source = ```
        let
            Source = Sql.Database(\"server\", \"db\")
        in
            Source
        ```
",
        ) else {
            panic!("fixture should parse");
        };

        assert_eq!(model.tables.len(), 1);
        let table = &model.tables[0];
        assert_eq!(table.partitions.len(), 1);
        let partition = &table.partitions[0];
        assert_eq!(partition.name, "FactVehicleRegistrations");
        assert_eq!(partition.source_kind.as_deref(), Some("m"));
        assert_eq!(partition.mode.as_deref(), Some("import"));
        let body = partition
            .source_expression
            .as_deref()
            .expect("partition should capture the fenced M body");
        assert!(body.contains("let"), "M body should retain the let keyword");
        assert!(
            body.contains("Sql.Database"),
            "M body should retain the source function call"
        );
        assert!(
            !body.contains("```"),
            "captured body must not include the fence delimiters"
        );
        // The column that precedes the partition must still parse.
        assert_eq!(table.columns.len(), 1);
        assert_eq!(table.columns[0].name, "Amount");
    }

    #[test]
    fn parse_data_source_properties() {
        let Some(model) = parse_tmdl_document(
            "
dataSource SqlWarehouse
  kind: sql
  provider: System.Data.SqlClient
  connectionString: Data Source=myserver;Initial Catalog=EDW
  server: myserver
  database: EDW
",
        ) else {
            panic!("fixture should parse");
        };

        assert_eq!(model.data_sources.len(), 1);
        let ds = &model.data_sources[0];
        assert_eq!(ds.name, "SqlWarehouse");
        assert_eq!(ds.kind.as_deref(), Some("sql"));
        assert_eq!(ds.provider.as_deref(), Some("System.Data.SqlClient"));
        assert_eq!(
            ds.connection_string.as_deref(),
            Some("Data Source=myserver;Initial Catalog=EDW")
        );
        assert_eq!(ds.server.as_deref(), Some("myserver"));
        assert_eq!(ds.database.as_deref(), Some("EDW"));
    }

    #[test]
    fn parse_refs_annotations_and_lineage() {
        let Some(model) = parse_tmdl_document(
            "
model Sales Model
  culture: en-US
  defaultMode: import
  lineageTag: model-guid-1
  annotation PBI_QueryOrder = [\"Sales\"]

  ref table Sales
  ref cultureInfo en-US

table Sales
  lineageTag: table-guid-1
  annotation IsHidden = false

  column Amount
    dataType: double
    lineageTag: column-guid-1
    annotation Format = \"#,0\"

  measure 'Total' = SUM(Sales[Amount])
    lineageTag: measure-guid-1
    annotation DisplayFolder = KPIs
",
        ) else {
            panic!("fixture should parse");
        };

        // Model-level metadata.
        assert_eq!(model.culture.as_deref(), Some("en-US"));
        assert_eq!(model.default_mode.as_deref(), Some("import"));
        assert_eq!(model.lineage_tag.as_deref(), Some("model-guid-1"));
        assert_eq!(model.annotations.len(), 1);
        assert_eq!(model.annotations[0].name, "PBI_QueryOrder");
        assert_eq!(model.annotations[0].value.as_deref(), Some("[\"Sales\"]"));

        // Model-level refs.
        assert_eq!(model.refs.len(), 2);
        assert_eq!(model.refs[0].kind, "table");
        assert_eq!(model.refs[0].name, "Sales");
        assert_eq!(model.refs[1].kind, "cultureInfo");
        assert_eq!(model.refs[1].name, "en-US");

        // Table-scoped metadata.
        assert_eq!(model.tables.len(), 1);
        let table = &model.tables[0];
        assert_eq!(table.lineage_tag.as_deref(), Some("table-guid-1"));
        assert_eq!(table.annotations.len(), 1);
        assert_eq!(table.annotations[0].name, "IsHidden");
        assert_eq!(table.annotations[0].value.as_deref(), Some("false"));

        // Column-scoped metadata.
        assert_eq!(table.columns.len(), 1);
        let column = &table.columns[0];
        assert_eq!(column.data_type.as_deref(), Some("double"));
        assert_eq!(column.lineage_tag.as_deref(), Some("column-guid-1"));
        assert_eq!(column.annotations.len(), 1);
        assert_eq!(column.annotations[0].name, "Format");

        // Measure-scoped metadata.
        assert_eq!(table.measures.len(), 1);
        let measure = &table.measures[0];
        assert_eq!(measure.lineage_tag.as_deref(), Some("measure-guid-1"));
        assert_eq!(measure.annotations.len(), 1);
        assert_eq!(measure.annotations[0].name, "DisplayFolder");
    }

    #[test]
    fn parse_hierarchy_does_not_corrupt_member_metadata() {
        let Some(model) = parse_tmdl_document(
            "
table Date
  column Year
    dataType: int64
    lineageTag: column-guid-1
    annotation ColAnno = colval

  hierarchy 'Calendar'
    lineageTag: hierarchy-guid
    annotation HierAnno = hierval
    level Year
      column: Year
",
        ) else {
            panic!("fixture should parse");
        };

        assert_eq!(model.tables.len(), 1);
        let table = &model.tables[0];
        assert_eq!(table.columns.len(), 1);
        let column = &table.columns[0];
        // The hierarchy's nested lineageTag/annotation must NOT overwrite or
        // pollute the preceding column's own metadata.
        assert_eq!(column.lineage_tag.as_deref(), Some("column-guid-1"));
        assert_eq!(column.annotations.len(), 1);
        assert_eq!(column.annotations[0].name, "ColAnno");
        // The hierarchy metadata is dropped, not mis-attached to the table.
        assert_eq!(table.lineage_tag, None);
        assert!(table.annotations.is_empty());
    }
}
