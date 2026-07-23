//! Tree-sitter SQL grammar parser.
//!
//! Extracts schema-definition symbols and reference edges from SQL source files.
//!
//! # Node kinds used (tree-sitter-sequel 0.3)
//!
//! Top-level structure: `program` > `statement` > actual statement node.
//!
//! - `create_table` / `create_view` → [`super::ExtractedSymbol::Class`]
//! - `create_function` → [`super::ExtractedSymbol::Function`]
//! - `CREATE PROCEDURE` is currently unsupported by tree-sitter-sequel 0.3 and
//!   parses as `ERROR` rather than `create_procedure`; the matcher for
//!   `create_procedure` is retained for forward compatibility with future
//!   grammar support
//! - `from` (SELECT from-clause, sibling inside `statement`),
//!   including JOIN clauses (`join`, `cross_join`, `lateral_join`,
//!   `lateral_cross_join`)
//!   and `insert` > `object_reference` → [`super::ExtractedEdge::References`]
//!
//! Names are extracted from the first `object_reference` > `identifier` child.

use tree_sitter::{Node, Parser};

use super::{ExtractedClass, ExtractedEdge, ExtractedFunction, ExtractedSymbol, ParseResult};

/// Parse a SQL source file and extract symbols and edges.
///
/// # Errors
///
/// Returns [`crate::errors::EngramError`] if the grammar cannot be loaded or
/// tree-sitter fails to produce a valid parse tree.
pub(super) fn parse_sql_source(source: &str) -> Result<ParseResult, crate::errors::EngramError> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_sequel::LANGUAGE.into())
        .map_err(|e| {
            crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
                reason: format!("Failed to set SQL grammar: {e}"),
            })
        })?;

    let tree = parser.parse(source, None).ok_or_else(|| {
        crate::errors::EngramError::CodeGraph(crate::errors::CodeGraphError::ParseFailed {
            reason: "tree-sitter returned no parse tree for SQL source".to_owned(),
        })
    })?;

    let root = tree.root_node();
    let mut symbols = Vec::new();
    let mut edges = Vec::new();

    extract_sql_top_level(root, source, &mut symbols, &mut edges);

    Ok(ParseResult { symbols, edges })
}

fn extract_sql_top_level(
    root: Node<'_>,
    source: &str,
    symbols: &mut Vec<ExtractedSymbol>,
    edges: &mut Vec<ExtractedEdge>,
) {
    // The grammar wraps every SQL statement in a `statement` container.
    let mut root_cursor = root.walk();
    for statement in root.children(&mut root_cursor) {
        if statement.kind() != "statement" {
            continue;
        }
        let mut stmt_cursor = statement.walk();
        for child in statement.children(&mut stmt_cursor) {
            match child.kind() {
                "create_table" | "create_view" => {
                    if let Some(cls) = extract_sql_class(child, source) {
                        edges.push(ExtractedEdge::Defines {
                            symbol_name: cls.name.clone(),
                        });
                        symbols.push(ExtractedSymbol::Class(cls));
                    }
                }
                "create_function" | "create_procedure" => {
                    if let Some(func) = extract_sql_function(child, source) {
                        edges.push(ExtractedEdge::Defines {
                            symbol_name: func.name.clone(),
                        });
                        symbols.push(ExtractedSymbol::Function(func));
                    }
                }
                // `from` is a sibling of `select` inside the same `statement`.
                "from" => {
                    extract_from_references(child, source, edges);
                }
                // INSERT has the target `object_reference` as a direct child.
                "insert" => {
                    extract_insert_references(child, source, edges);
                }
                _ => {}
            }
        }
    }
}

/// Extract the name from a CREATE TABLE/VIEW/FUNCTION/PROCEDURE node.
///
/// Names live in `object_reference` > `identifier` children joined with `.`
/// to support schema-qualified names like `public.users`.
fn extract_sql_name(node: Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "object_reference" {
            let mut inner = child.walk();
            let parts: Vec<String> = child
                .children(&mut inner)
                .filter(|n| n.kind() == "identifier")
                .map(|n| super::node_text(n, source))
                .collect();
            if !parts.is_empty() {
                return Some(parts.join("."));
            }
        }
    }
    None
}

fn extract_sql_class(node: Node<'_>, source: &str) -> Option<ExtractedClass> {
    let name = extract_sql_name(node, source)?;
    let body = super::node_text(node, source);
    let body_hash = super::sha256_hex(&body);
    #[allow(clippy::cast_possible_truncation)]
    let line_start = (node.start_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let line_end = (node.end_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let token_count = (body.len() / 4) as u32;
    Some(ExtractedClass {
        name,
        line_start,
        line_end,
        docstring: None,
        body,
        body_hash,
        token_count,
    })
}

fn extract_sql_function(node: Node<'_>, source: &str) -> Option<ExtractedFunction> {
    let name = extract_sql_name(node, source)?;
    let body = super::node_text(node, source);
    let body_hash = super::sha256_hex(&body);
    #[allow(clippy::cast_possible_truncation)]
    let line_start = (node.start_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let line_end = (node.end_position().row + 1) as u32;
    #[allow(clippy::cast_possible_truncation)]
    let token_count = (body.len() / 4) as u32;
    Some(ExtractedFunction {
        name,
        line_start,
        line_end,
        signature: String::new(),
        docstring: None,
        body,
        body_hash,
        token_count,
    })
}

/// Extract table references from a `from` clause node.
///
/// Handles both direct `relation` children and JOIN children (`join`,
/// `cross_join`, `lateral_join`, `lateral_cross_join`), each of which
/// also contains a `relation` > `object_reference` subtree (033.002-T).
fn extract_from_references(node: Node<'_>, source: &str, edges: &mut Vec<ExtractedEdge>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "relation" => {
                extract_relation_reference(child, source, edges);
            }
            // JOIN variants each embed a `relation` for the joined table.
            "join" | "cross_join" | "lateral_join" | "lateral_cross_join" => {
                let mut join_cursor = child.walk();
                for join_child in child.children(&mut join_cursor) {
                    if join_child.kind() == "relation" {
                        extract_relation_reference(join_child, source, edges);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Push a `References` edge for the `object_reference` inside a `relation` node.
fn extract_relation_reference(relation: Node<'_>, source: &str, edges: &mut Vec<ExtractedEdge>) {
    let mut rel_cursor = relation.walk();
    for obj_ref in relation.children(&mut rel_cursor) {
        if obj_ref.kind() == "object_reference" {
            let mut id_cursor = obj_ref.walk();
            let parts: Vec<String> = obj_ref
                .children(&mut id_cursor)
                .filter(|n| n.kind() == "identifier")
                .map(|n| super::node_text(n, source))
                .collect();
            if !parts.is_empty() {
                let target = parts.join(".");
                edges.push(ExtractedEdge::References {
                    source: "select".to_owned(),
                    target,
                });
            }
        }
    }
}

/// Extract the target table reference from an `insert` node.
///
/// Structure: `insert` > `object_reference` > `identifier` children joined with `.`
/// to support schema-qualified targets such as `dbo.orders` (033.002-T).
fn extract_insert_references(node: Node<'_>, source: &str, edges: &mut Vec<ExtractedEdge>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "object_reference" {
            let mut id_cursor = child.walk();
            let parts: Vec<String> = child
                .children(&mut id_cursor)
                .filter(|n| n.kind() == "identifier")
                .map(|n| super::node_text(n, source))
                .collect();
            if !parts.is_empty() {
                let target = parts.join(".");
                edges.push(ExtractedEdge::References {
                    source: "insert".to_owned(),
                    target,
                });
            }
            return;
        }
    }
}

/// Spark-SQL table-lineage extraction (095-F, plan Unit U3).
///
/// The pure extraction half of the Spark-SQL lineage path. It recovers directional
/// `derives_from` links from CTAS (`CREATE TABLE … AS SELECT … FROM …`) and Spark
/// `INSERT` (`INSERT INTO/OVERWRITE [TABLE] … SELECT … FROM …`) statements, grouped
/// per statement so each write target keeps its own read sources (F3/F4). Only a
/// 3-part `catalog.schema.table` bound to a trusted metastore authority resolves;
/// 1-/2-part names, `ERROR`/`CREATE PROCEDURE` statements, `CREATE VIEW`/temp-view
/// DDL, and 3-part names with no trusted authority all drop (013-D, 0 false edges).
///
/// U0 (095.001-T, Outcome A) confirmed enhancing `sql.rs` suffices: CTAS from-descent,
/// INSERT target/source extraction, and a bounded `TABLE`-keyword normalization shim
/// (tree-sitter-sequel 0.3 mis-parses the Spark `TABLE` keyword).
/// U3 Spark-SQL lineage extractor, re-exported for the notebook router (U4a).
pub(crate) use sql_lineage::extract_sql_lineage;

#[allow(dead_code)]
pub(super) mod sql_lineage {
    use std::collections::HashSet;

    use tree_sitter::{Node, Parser};

    use crate::errors::{CodeGraphError, EngramError};
    use crate::models::lineage::{LineageAuthorityContext, LineageEdgeCandidate};

    /// Extract statement-grouped table-lineage candidates from Spark-SQL `source`.
    ///
    /// # Errors
    ///
    /// Returns [`EngramError`] if the SQL grammar cannot be loaded or tree-sitter
    /// fails to produce a parse tree.
    pub(crate) fn extract_sql_lineage(
        source: &str,
        authority_ctx: &LineageAuthorityContext,
    ) -> Result<Vec<LineageEdgeCandidate>, EngramError> {
        let normalized = normalize_spark_insert(source);
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_sequel::LANGUAGE.into())
            .map_err(|e| {
                EngramError::CodeGraph(CodeGraphError::ParseFailed {
                    reason: format!("Failed to set SQL grammar: {e}"),
                })
            })?;
        let tree = parser.parse(&normalized, None).ok_or_else(|| {
            EngramError::CodeGraph(CodeGraphError::ParseFailed {
                reason: "tree-sitter returned no parse tree for SQL source".to_owned(),
            })
        })?;

        let root = tree.root_node();
        let mut candidates = Vec::new();
        let mut cursor = root.walk();
        for statement in root.children(&mut cursor) {
            if statement.kind() != "statement" {
                continue; // ERROR / CREATE PROCEDURE are not `statement` nodes.
            }
            let mut stmt_cursor = statement.walk();
            for child in statement.children(&mut stmt_cursor) {
                match child.kind() {
                    // CTAS only — a plain `CREATE TABLE (cols…)` has no
                    // `create_query` and therefore no read sources.
                    "create_table" if has_create_query(child) => {
                        if let Some(candidate) = build_candidate(child, &normalized, authority_ctx)
                        {
                            candidates.push(candidate);
                        }
                    }
                    "insert" => {
                        if let Some(candidate) = build_candidate(child, &normalized, authority_ctx)
                        {
                            candidates.push(candidate);
                        }
                    }
                    // create_view / create_function / plain create_table: no edge.
                    _ => {}
                }
            }
        }
        Ok(candidates)
    }

    /// Build one statement-grouped candidate: the direct-child `object_reference`
    /// is the write target; every `relation` in the subtree is a read source.
    /// Fails closed when the target does not resolve or no source resolves.
    fn build_candidate(
        node: Node<'_>,
        source: &str,
        ctx: &LineageAuthorityContext,
    ) -> Option<LineageEdgeCandidate> {
        let target_name = direct_object_reference_name(node, source)?;
        let target = ctx.resolve_table(&target_name)?;

        let mut source_names = Vec::new();
        collect_relation_names(node, source, &mut source_names);

        let mut sources = Vec::new();
        let mut seen = HashSet::new();
        for name in source_names {
            if let Some(endpoint) = ctx.resolve_table(&name) {
                if seen.insert(endpoint.id.clone()) {
                    sources.push(endpoint);
                }
            }
        }
        if sources.is_empty() {
            return None;
        }
        Some(LineageEdgeCandidate { target, sources })
    }

    /// Whether a `create_table` node has an `AS SELECT` body (`create_query`).
    fn has_create_query(node: Node<'_>) -> bool {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .any(|c| c.kind() == "create_query")
    }

    /// The dotted name of the first **direct** `object_reference` child (the
    /// write target of a `create_table`/`insert`).
    fn direct_object_reference_name(node: Node<'_>, source: &str) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "object_reference" {
                if let Some(name) = object_reference_dotted_name(child, source) {
                    return Some(name);
                }
            }
        }
        None
    }

    /// Recursively collect the dotted name of every `relation`'s
    /// `object_reference` in source order (the read sources). `relation` nodes
    /// only occur in `from`/`join` contexts, so column references are excluded.
    fn collect_relation_names(node: Node<'_>, source: &str, out: &mut Vec<String>) {
        if node.kind() == "relation" {
            if let Some(name) = direct_object_reference_name(node, source) {
                out.push(name);
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            collect_relation_names(child, source, out);
        }
    }

    /// Join an `object_reference`'s `identifier` children with `.`.
    fn object_reference_dotted_name(object_reference: Node<'_>, source: &str) -> Option<String> {
        let mut cursor = object_reference.walk();
        let parts: Vec<&str> = object_reference
            .children(&mut cursor)
            .filter(|n| n.kind() == "identifier")
            .map(|n| node_text(n, source))
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts.join("."))
        }
    }

    /// Slice `source` by a node's byte range (self-contained; no `super::`).
    fn node_text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
        &source[node.byte_range()]
    }

    // ── `TABLE`-keyword normalization shim (U0 finding) ───────────────────────

    /// Rewrite Spark `INSERT OVERWRITE TABLE <t>` / `INSERT INTO TABLE <t>` to the
    /// grammar-clean `INSERT INTO <t>` form.
    ///
    /// tree-sitter-sequel 0.3 mis-parses the `TABLE` keyword (consuming it as the
    /// target's `database` identifier and raising `ERROR`). This bounded pre-pass
    /// strips the stray `OVERWRITE`/`TABLE` keywords so the statement parses via
    /// the same grammar; it only ever rewrites these two documented prefixes and
    /// preserves all other text verbatim (Outcome A — not a grammar swap).
    fn normalize_spark_insert(source: &str) -> String {
        let bytes = source.as_bytes();
        let mut out = String::with_capacity(source.len());
        let mut copy_from = 0usize;
        let mut i = 0usize;
        while i < bytes.len() {
            if is_word_boundary_before(bytes, i) && match_ci_word(bytes, i, b"INSERT") {
                if let Some(end) = insert_table_prefix_end(bytes, i + b"INSERT".len()) {
                    out.push_str(&source[copy_from..i]);
                    out.push_str("INSERT INTO");
                    i = end;
                    copy_from = end;
                    continue;
                }
            }
            i += 1;
        }
        out.push_str(&source[copy_from..]);
        out
    }

    /// After an `INSERT`, match `OVERWRITE TABLE` / `INTO TABLE` and return the
    /// byte index just past the `TABLE` keyword, or `None`.
    fn insert_table_prefix_end(bytes: &[u8], after_insert: usize) -> Option<usize> {
        let lead_start = skip_ws(bytes, after_insert);
        if lead_start == after_insert {
            return None; // A keyword must follow whitespace after `INSERT`.
        }
        for lead in [b"OVERWRITE".as_slice(), b"INTO".as_slice()] {
            if let Some(after_lead) = matched_word_end(bytes, lead_start, lead) {
                let table_start = skip_ws(bytes, after_lead);
                if table_start == after_lead {
                    continue;
                }
                if let Some(after_table) = matched_word_end(bytes, table_start, b"TABLE") {
                    return Some(after_table);
                }
            }
        }
        None
    }

    /// Return `pos + kw.len()` when `kw` matches at `pos` as a whole word.
    fn matched_word_end(bytes: &[u8], pos: usize, kw: &[u8]) -> Option<usize> {
        match_ci_word(bytes, pos, kw).then(|| pos + kw.len())
    }

    /// Case-insensitive whole-word match of `kw` at `pos` (trailing boundary
    /// required; the caller checks the leading boundary).
    fn match_ci_word(bytes: &[u8], pos: usize, kw: &[u8]) -> bool {
        let end = pos + kw.len();
        if end > bytes.len() {
            return false;
        }
        if !bytes[pos..end]
            .iter()
            .zip(kw)
            .all(|(b, k)| b.eq_ignore_ascii_case(k))
        {
            return false;
        }
        bytes.get(end).is_none_or(|b| !is_word_byte(*b))
    }

    /// Advance past ASCII whitespace.
    fn skip_ws(bytes: &[u8], mut pos: usize) -> usize {
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        pos
    }

    /// Whether the byte before `i` is a word boundary (or start of input).
    fn is_word_boundary_before(bytes: &[u8], i: usize) -> bool {
        i == 0 || !is_word_byte(bytes[i - 1])
    }

    /// Whether `b` is an identifier byte (`[A-Za-z0-9_]`).
    fn is_word_byte(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }

    #[cfg(test)]
    mod tests {
        use std::collections::BTreeMap;

        use super::*;

        fn trusted_ctx() -> LineageAuthorityContext {
            let mut catalogs = BTreeMap::new();
            catalogs.insert("cat".to_owned(), "prod-metastore".to_owned());
            LineageAuthorityContext::new(catalogs, Vec::new())
        }

        fn names(candidate: &LineageEdgeCandidate) -> (String, Vec<String>) {
            (
                candidate.target.name.clone(),
                candidate.sources.iter().map(|s| s.name.clone()).collect(),
            )
        }

        #[test]
        fn ctas_and_insert_overwrite_table_emit_derives_edges() {
            let ctx = trusted_ctx();

            let ctas =
                extract_sql_lineage("CREATE TABLE cat.sch.t AS SELECT x FROM cat.sch.src", &ctx)
                    .expect("ctas");
            assert_eq!(ctas.len(), 1);
            let (target, sources) = names(&ctas[0]);
            assert_eq!(target, "cat.sch.t");
            assert_eq!(sources, vec!["cat.sch.src".to_owned()]);

            // INSERT OVERWRITE TABLE requires the `TABLE`-keyword normalization.
            let insert = extract_sql_lineage(
                "INSERT OVERWRITE TABLE cat.sch.t SELECT x FROM cat.sch.src",
                &ctx,
            )
            .expect("insert");
            assert_eq!(insert.len(), 1);
            let (target, sources) = names(&insert[0]);
            assert_eq!(target, "cat.sch.t");
            assert_eq!(sources, vec!["cat.sch.src".to_owned()]);

            // INSERT INTO TABLE (also mis-parsed without normalization).
            let insert_into = extract_sql_lineage(
                "INSERT INTO TABLE cat.sch.t SELECT x FROM cat.sch.src",
                &ctx,
            )
            .expect("insert into table");
            assert_eq!(insert_into.len(), 1);
            assert_eq!(names(&insert_into[0]).0, "cat.sch.t");
        }

        #[test]
        fn multi_source_ctas_groups_sources_and_statements_keep_distinct_targets() {
            let ctx = trusted_ctx();

            let joined = extract_sql_lineage(
                "CREATE TABLE cat.sch.t AS SELECT a.x FROM cat.sch.a JOIN cat.sch.b ON a.id = b.id",
                &ctx,
            )
            .expect("join ctas");
            assert_eq!(joined.len(), 1, "one candidate for one statement (F3)");
            let (target, sources) = names(&joined[0]);
            assert_eq!(target, "cat.sch.t");
            assert_eq!(
                sources,
                vec!["cat.sch.a".to_owned(), "cat.sch.b".to_owned()],
                "both join sources grouped under one target"
            );

            let two = extract_sql_lineage(
                concat!(
                    "CREATE TABLE cat.sch.t1 AS SELECT x FROM cat.sch.a;\n",
                    "CREATE TABLE cat.sch.t2 AS SELECT x FROM cat.sch.b",
                ),
                &ctx,
            )
            .expect("two statements");
            assert_eq!(two.len(), 2, "two statements → two candidates (F4)");
            let targets: Vec<String> = two.iter().map(|c| c.target.name.clone()).collect();
            assert_eq!(
                targets,
                vec!["cat.sch.t1".to_owned(), "cat.sch.t2".to_owned()]
            );
        }

        #[test]
        fn no_authority_resolves_zero_edges() {
            let ctx = LineageAuthorityContext::empty();
            let candidates = extract_sql_lineage(
                concat!(
                    "CREATE TABLE cat.sch.t AS SELECT a.x FROM cat.sch.a JOIN cat.sch.b ON a.id = b.id;\n",
                    "INSERT OVERWRITE TABLE cat.sch.t SELECT x FROM cat.sch.src",
                ),
                &ctx,
            )
            .expect("no authority");
            assert_eq!(candidates.len(), 0, "no trusted authority ⇒ fail-closed");
        }

        #[test]
        fn two_part_names_and_error_statements_resolve_zero_edges() {
            let ctx = trusted_ctx();

            // 2-part names never resolve (need 3-part catalog.schema.table).
            let two_part = extract_sql_lineage("CREATE TABLE sch.t AS SELECT x FROM sch.src", &ctx)
                .expect("two part");
            assert_eq!(two_part.len(), 0, "2-part names fail closed");

            // CREATE PROCEDURE parses to ERROR (not a `statement`) → no edge.
            let procedure =
                extract_sql_lineage("CREATE PROCEDURE foo() BEGIN END", &ctx).expect("procedure");
            assert_eq!(procedure.len(), 0, "ERROR statements fail closed");

            // CREATE VIEW is not table lineage → no edge even when resolvable.
            let view =
                extract_sql_lineage("CREATE VIEW cat.sch.v AS SELECT x FROM cat.sch.src", &ctx)
                    .expect("view");
            assert_eq!(
                view.len(),
                0,
                "CREATE VIEW is excluded (table lineage only)"
            );
        }
    }
}
