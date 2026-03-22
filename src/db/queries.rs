//! Database query helpers for code graph operations.
//!
//! [`CodeGraphQueries`] provides typed, validated methods for all code graph
//! tables (code_file, function, class, interface, code edges, content records,
//! and commit nodes).

use std::collections::{HashSet, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

use crate::db::{Db, map_db_err};
use crate::errors::EngramError;

// ── Query performance observability (dxo.5.1) ──────────────────────────

/// Threshold in milliseconds above which a query is logged at WARN level.
pub const SLOW_QUERY_THRESHOLD_MS: u128 = 100;

/// Records query timing metadata and emits a warning for slow queries.
///
/// Intended to be called at the end of each public query method to:
/// 1. Record `result_count` in the current tracing span.
/// 2. Log a warning if elapsed time exceeds [`SLOW_QUERY_THRESHOLD_MS`].
///
/// # Arguments
///
/// * `query_type` — Category label (e.g., `"graph_traversal"`, `"knn_search"`, `"crud"`).
/// * `table` — Primary table being queried (e.g., `"function"`, `"class"`).
/// * `result_count` — Number of rows/items returned by the query.
/// * `elapsed` — Wall-clock duration of the query execution.
pub fn record_query_metrics(
    query_type: &str,
    table: &str,
    result_count: usize,
    elapsed: std::time::Duration,
) {
    let elapsed_ms = elapsed.as_millis();

    // Record fields on the current tracing span (silently ignored when no
    // matching pre-declared fields exist on the span).
    let span = tracing::Span::current();
    span.record("query_type", query_type);
    span.record("table", table);
    span.record("result_count", result_count);

    // Downcast to u64 for tracing event fields (`u128` does not implement
    // `tracing::Value`).
    let elapsed_ms_u64 = u64::try_from(elapsed_ms).unwrap_or(u64::MAX);

    tracing::info!(
        query_type,
        table,
        result_count,
        elapsed_ms = elapsed_ms_u64,
        "query completed"
    );

    if elapsed_ms > SLOW_QUERY_THRESHOLD_MS {
        tracing::warn!(
            query_type,
            table,
            result_count,
            elapsed_ms = elapsed_ms_u64,
            "slow query detected"
        );
    }
}

// ── Shared Row Types ───────────────────────────────────────────────────

/// Internal row type for COUNT() aggregate queries.
#[derive(Deserialize)]
struct CountRow {
    count: u64,
}

// ── Code Graph Row Types ───────────────────────────────────────────────

/// Internal row type for deserializing code_file records from SurrealDB.
#[derive(Deserialize)]
struct CodeFileRow {
    id: Thing,
    path: String,
    language: String,
    size_bytes: u64,
    content_hash: String,
    last_indexed_at: DateTime<Utc>,
}

impl CodeFileRow {
    fn into_code_file(self) -> crate::models::CodeFile {
        crate::models::CodeFile {
            id: format!("code_file:{}", self.id.id.to_raw()),
            path: self.path,
            language: self.language,
            size_bytes: self.size_bytes,
            content_hash: self.content_hash,
            last_indexed_at: self.last_indexed_at.to_rfc3339(),
        }
    }
}

/// Internal row type for deserializing function records from SurrealDB.
#[derive(Deserialize)]
struct FunctionRow {
    id: Thing,
    name: String,
    file_path: String,
    line_start: u32,
    line_end: u32,
    #[serde(default)]
    signature: String,
    #[serde(default)]
    docstring: Option<String>,
    body_hash: String,
    token_count: u32,
    embed_type: String,
    #[serde(default)]
    embedding: Vec<f32>,
    #[serde(default)]
    summary: String,
    /// `SurrealDB`-computed KNN similarity score (present when queried via
    /// `vector::similarity::cosine(embedding, $query) AS knn_score`).
    #[serde(default)]
    knn_score: Option<f32>,
}

impl FunctionRow {
    fn into_function(self) -> crate::models::Function {
        crate::models::Function {
            id: format!("function:{}", self.id.id.to_raw()),
            name: self.name,
            file_path: self.file_path,
            line_start: self.line_start,
            line_end: self.line_end,
            signature: self.signature,
            docstring: self.docstring,
            body: String::new(), // body populated at runtime from source
            body_hash: self.body_hash,
            token_count: self.token_count,
            embed_type: self.embed_type,
            embedding: self.embedding,
            summary: self.summary,
        }
    }
}

/// Internal row type for deserializing class records from SurrealDB.
#[derive(Deserialize)]
struct ClassRow {
    id: Thing,
    name: String,
    file_path: String,
    line_start: u32,
    line_end: u32,
    #[serde(default)]
    docstring: Option<String>,
    body_hash: String,
    token_count: u32,
    embed_type: String,
    #[serde(default)]
    embedding: Vec<f32>,
    #[serde(default)]
    summary: String,
    /// `SurrealDB`-computed KNN similarity score.
    #[serde(default)]
    knn_score: Option<f32>,
}

impl ClassRow {
    fn into_class(self) -> crate::models::Class {
        crate::models::Class {
            id: format!("class:{}", self.id.id.to_raw()),
            name: self.name,
            file_path: self.file_path,
            line_start: self.line_start,
            line_end: self.line_end,
            docstring: self.docstring,
            body: String::new(),
            body_hash: self.body_hash,
            token_count: self.token_count,
            embed_type: self.embed_type,
            embedding: self.embedding,
            summary: self.summary,
        }
    }
}

/// Internal row type for deserializing interface records from SurrealDB.
#[derive(Deserialize)]
struct InterfaceRow {
    id: Thing,
    name: String,
    file_path: String,
    line_start: u32,
    line_end: u32,
    #[serde(default)]
    docstring: Option<String>,
    body_hash: String,
    token_count: u32,
    embed_type: String,
    #[serde(default)]
    embedding: Vec<f32>,
    #[serde(default)]
    summary: String,
    /// `SurrealDB`-computed KNN similarity score.
    #[serde(default)]
    knn_score: Option<f32>,
}

impl InterfaceRow {
    fn into_interface(self) -> crate::models::Interface {
        crate::models::Interface {
            id: format!("interface:{}", self.id.id.to_raw()),
            name: self.name,
            file_path: self.file_path,
            line_start: self.line_start,
            line_end: self.line_end,
            docstring: self.docstring,
            body: String::new(),
            body_hash: self.body_hash,
            token_count: self.token_count,
            embed_type: self.embed_type,
            embedding: self.embedding,
            summary: self.summary,
        }
    }
}

/// Internal row type for deserializing code edge records from SurrealDB.
#[derive(Deserialize)]
#[allow(dead_code)]
struct CodeEdgeRow {
    r#in: Thing,
    out: Thing,
    #[serde(default)]
    import_path: Option<String>,
    #[serde(default)]
    linked_by: Option<String>,
    #[serde(default)]
    created_at: Option<DateTime<Utc>>,
}

// ── Code Graph Queries ─────────────────────────────────────────────────

/// Query helper for code graph CRUD operations.
///
/// Wraps a cloneable SurrealDB handle and provides typed, validated methods
/// for all code graph tables (code files, functions, classes, interfaces,
/// edges, content records, and commit nodes).
#[derive(Clone)]
pub struct CodeGraphQueries {
    db: Db,
}

impl CodeGraphQueries {
    /// Create a new `CodeGraphQueries` instance wrapping the given DB handle.
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    // ── code_file CRUD ──────────────────────────────────────────────

    /// Insert or update a code file record.
    pub async fn upsert_code_file(
        &self,
        file: &crate::models::CodeFile,
    ) -> Result<(), EngramError> {
        let id_raw = file.id.strip_prefix("code_file:").unwrap_or(&file.id);
        let record = Thing::from(("code_file", id_raw));
        #[allow(clippy::cast_possible_wrap)]
        let size_i64 = file.size_bytes as i64;
        self.db
            .query("UPSERT $id SET path = $path, language = $lang, size_bytes = $size, content_hash = $hash, last_indexed_at = time::now()")
            .bind(("id", record))
            .bind(("path", file.path.clone()))
            .bind(("lang", file.language.clone()))
            .bind(("size", size_i64))
            .bind(("hash", file.content_hash.clone()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Look up a code file by its workspace-relative path.
    pub async fn get_code_file_by_path(
        &self,
        path: &str,
    ) -> Result<Option<crate::models::CodeFile>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM code_file WHERE path = $path LIMIT 1")
            .bind(("path", path.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CodeFileRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map(CodeFileRow::into_code_file))
    }

    /// Delete a code file record and all its `defines` edges.
    pub async fn delete_code_file(&self, path: &str) -> Result<(), EngramError> {
        self.db
            .query("DELETE FROM code_file WHERE path = $path")
            .bind(("path", path.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// List all indexed code files.
    pub async fn list_code_files(&self) -> Result<Vec<crate::models::CodeFile>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM code_file ORDER BY path ASC")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CodeFileRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().map(CodeFileRow::into_code_file).collect())
    }

    /// Return all functions in the code graph.
    pub async fn all_functions(&self) -> Result<Vec<crate::models::Function>, EngramError> {
        let mut resp = self
            .db
            .query("SELECT * FROM `function` ORDER BY id ASC")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<FunctionRow> = resp.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().map(FunctionRow::into_function).collect())
    }

    /// Return all classes in the code graph.
    pub async fn all_classes(&self) -> Result<Vec<crate::models::Class>, EngramError> {
        let mut resp = self
            .db
            .query("SELECT * FROM class ORDER BY id ASC")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<ClassRow> = resp.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().map(ClassRow::into_class).collect())
    }

    /// Return all interfaces in the code graph.
    pub async fn all_interfaces(&self) -> Result<Vec<crate::models::Interface>, EngramError> {
        let mut resp = self
            .db
            .query("SELECT * FROM interface ORDER BY id ASC")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<InterfaceRow> = resp.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().map(InterfaceRow::into_interface).collect())
    }

    /// Return all code edges across every edge type.
    pub async fn all_code_edges(&self) -> Result<Vec<crate::models::CodeEdge>, EngramError> {
        use crate::models::code_edge::{CodeEdge, CodeEdgeType};

        let mut edges: Vec<CodeEdge> = Vec::new();

        // Calls edges
        let mut resp = self
            .db
            .query("SELECT * FROM calls")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CodeEdgeRow> = resp.take(0).map_err(map_db_err)?;
        for row in rows {
            edges.push(CodeEdge {
                edge_type: CodeEdgeType::Calls,
                from: format!("{}:{}", row.r#in.tb, row.r#in.id.to_raw()),
                to: format!("{}:{}", row.out.tb, row.out.id.to_raw()),
                import_path: None,
                linked_by: None,
                created_at: row
                    .created_at
                    .map_or_else(String::new, |dt| dt.to_rfc3339()),
            });
        }

        // Imports edges
        let mut resp = self
            .db
            .query("SELECT * FROM imports")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CodeEdgeRow> = resp.take(0).map_err(map_db_err)?;
        for row in rows {
            edges.push(CodeEdge {
                edge_type: CodeEdgeType::Imports,
                from: format!("{}:{}", row.r#in.tb, row.r#in.id.to_raw()),
                to: format!("{}:{}", row.out.tb, row.out.id.to_raw()),
                import_path: row.import_path,
                linked_by: None,
                created_at: row
                    .created_at
                    .map_or_else(String::new, |dt| dt.to_rfc3339()),
            });
        }

        // Defines edges
        let mut resp = self
            .db
            .query("SELECT * FROM defines")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CodeEdgeRow> = resp.take(0).map_err(map_db_err)?;
        for row in rows {
            edges.push(CodeEdge {
                edge_type: CodeEdgeType::Defines,
                from: format!("{}:{}", row.r#in.tb, row.r#in.id.to_raw()),
                to: format!("{}:{}", row.out.tb, row.out.id.to_raw()),
                import_path: None,
                linked_by: None,
                created_at: row
                    .created_at
                    .map_or_else(String::new, |dt| dt.to_rfc3339()),
            });
        }

        // Inherits_from edges
        let mut resp = self
            .db
            .query("SELECT * FROM inherits_from")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CodeEdgeRow> = resp.take(0).map_err(map_db_err)?;
        for row in rows {
            edges.push(CodeEdge {
                edge_type: CodeEdgeType::InheritsFrom,
                from: format!("{}:{}", row.r#in.tb, row.r#in.id.to_raw()),
                to: format!("{}:{}", row.out.tb, row.out.id.to_raw()),
                import_path: None,
                linked_by: None,
                created_at: row
                    .created_at
                    .map_or_else(String::new, |dt| dt.to_rfc3339()),
            });
        }

        // Concerns edges
        let mut resp = self
            .db
            .query("SELECT * FROM concerns")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CodeEdgeRow> = resp.take(0).map_err(map_db_err)?;
        for row in rows {
            edges.push(CodeEdge {
                edge_type: CodeEdgeType::Concerns,
                from: format!("{}:{}", row.r#in.tb, row.r#in.id.to_raw()),
                to: format!("{}:{}", row.out.tb, row.out.id.to_raw()),
                import_path: None,
                linked_by: row.linked_by,
                created_at: row
                    .created_at
                    .map_or_else(String::new, |dt| dt.to_rfc3339()),
            });
        }

        // Sort by (type, from, to)
        edges.sort_by(|a, b| {
            a.edge_type
                .as_str()
                .cmp(b.edge_type.as_str())
                .then(a.from.cmp(&b.from))
                .then(a.to.cmp(&b.to))
        });

        Ok(edges)
    }

    // ── function CRUD ───────────────────────────────────────────────

    /// Insert or update a function record.
    pub async fn upsert_function(&self, func: &crate::models::Function) -> Result<(), EngramError> {
        let id_raw = func.id.strip_prefix("function:").unwrap_or(&func.id);
        let record = Thing::from(("function", id_raw));
        self.db
            .query("UPSERT $id SET name = $name, file_path = $fp, line_start = $ls, line_end = $le, signature = $sig, docstring = $doc, body_hash = $bh, token_count = $tc, embed_type = $et, embedding = $emb, summary = $sum")
            .bind(("id", record))
            .bind(("name", func.name.clone()))
            .bind(("fp", func.file_path.clone()))
            .bind(("ls", i64::from(func.line_start)))
            .bind(("le", i64::from(func.line_end)))
            .bind(("sig", func.signature.clone()))
            .bind(("doc", func.docstring.clone()))
            .bind(("bh", func.body_hash.clone()))
            .bind(("tc", i64::from(func.token_count)))
            .bind(("et", func.embed_type.clone()))
            .bind(("emb", func.embedding.clone()))
            .bind(("sum", func.summary.clone()))
            .await
            .map_err(map_db_err)?
            .check()
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Look up a function by name.
    pub async fn get_function_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::models::Function>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM `function` WHERE name = $name LIMIT 1")
            .bind(("name", name.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<FunctionRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map(FunctionRow::into_function))
    }

    /// List all functions in a given file.
    pub async fn get_functions_by_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<crate::models::Function>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM `function` WHERE file_path = $fp ORDER BY line_start ASC")
            .bind(("fp", file_path.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<FunctionRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().map(FunctionRow::into_function).collect())
    }

    /// Delete all functions in a given file.
    pub async fn delete_functions_by_file(&self, file_path: &str) -> Result<(), EngramError> {
        self.db
            .query("DELETE FROM `function` WHERE file_path = $fp")
            .bind(("fp", file_path.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    // ── class CRUD ──────────────────────────────────────────────────

    /// Insert or update a class record.
    pub async fn upsert_class(&self, class: &crate::models::Class) -> Result<(), EngramError> {
        let id_raw = class.id.strip_prefix("class:").unwrap_or(&class.id);
        let record = Thing::from(("class", id_raw));
        self.db
            .query("UPSERT $id SET name = $name, file_path = $fp, line_start = $ls, line_end = $le, docstring = $doc, body_hash = $bh, token_count = $tc, embed_type = $et, embedding = $emb, summary = $sum")
            .bind(("id", record))
            .bind(("name", class.name.clone()))
            .bind(("fp", class.file_path.clone()))
            .bind(("ls", i64::from(class.line_start)))
            .bind(("le", i64::from(class.line_end)))
            .bind(("doc", class.docstring.clone()))
            .bind(("bh", class.body_hash.clone()))
            .bind(("tc", i64::from(class.token_count)))
            .bind(("et", class.embed_type.clone()))
            .bind(("emb", class.embedding.clone()))
            .bind(("sum", class.summary.clone()))
            .await
            .map_err(map_db_err)?
            .check()
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Look up a class by name.
    pub async fn get_class_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::models::Class>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM class WHERE name = $name LIMIT 1")
            .bind(("name", name.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<ClassRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map(ClassRow::into_class))
    }

    /// Delete all classes in a given file.
    pub async fn delete_classes_by_file(&self, file_path: &str) -> Result<(), EngramError> {
        self.db
            .query("DELETE FROM class WHERE file_path = $fp")
            .bind(("fp", file_path.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    // ── interface CRUD ──────────────────────────────────────────────

    /// Insert or update an interface record.
    pub async fn upsert_interface(
        &self,
        iface: &crate::models::Interface,
    ) -> Result<(), EngramError> {
        let id_raw = iface.id.strip_prefix("interface:").unwrap_or(&iface.id);
        let record = Thing::from(("interface", id_raw));
        self.db
            .query("UPSERT $id SET name = $name, file_path = $fp, line_start = $ls, line_end = $le, docstring = $doc, body_hash = $bh, token_count = $tc, embed_type = $et, embedding = $emb, summary = $sum")
            .bind(("id", record))
            .bind(("name", iface.name.clone()))
            .bind(("fp", iface.file_path.clone()))
            .bind(("ls", i64::from(iface.line_start)))
            .bind(("le", i64::from(iface.line_end)))
            .bind(("doc", iface.docstring.clone()))
            .bind(("bh", iface.body_hash.clone()))
            .bind(("tc", i64::from(iface.token_count)))
            .bind(("et", iface.embed_type.clone()))
            .bind(("emb", iface.embedding.clone()))
            .bind(("sum", iface.summary.clone()))
            .await
            .map_err(map_db_err)?
            .check()
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Look up an interface by name.
    pub async fn get_interface_by_name(
        &self,
        name: &str,
    ) -> Result<Option<crate::models::Interface>, EngramError> {
        let mut response = self
            .db
            .query("SELECT * FROM interface WHERE name = $name LIMIT 1")
            .bind(("name", name.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<InterfaceRow> = response.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map(InterfaceRow::into_interface))
    }

    /// Delete all interfaces in a given file.
    pub async fn delete_interfaces_by_file(&self, file_path: &str) -> Result<(), EngramError> {
        self.db
            .query("DELETE FROM interface WHERE file_path = $fp")
            .bind(("fp", file_path.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    // ── Edge CRUD ───────────────────────────────────────────────────

    /// Create a `calls` edge between two functions.
    #[allow(clippy::similar_names)]
    pub async fn create_calls_edge(
        &self,
        caller_id: &str,
        callee_id: &str,
    ) -> Result<(), EngramError> {
        let from = Thing::from((
            "function",
            caller_id.strip_prefix("function:").unwrap_or(caller_id),
        ));
        let to = Thing::from((
            "function",
            callee_id.strip_prefix("function:").unwrap_or(callee_id),
        ));
        self.db
            .query("RELATE $from->calls->$to")
            .bind(("from", from))
            .bind(("to", to))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Create an `imports` edge between two code files.
    #[allow(clippy::similar_names)]
    pub async fn create_imports_edge(
        &self,
        importer_id: &str,
        imported_id: &str,
        import_path: &str,
    ) -> Result<(), EngramError> {
        let from = Thing::from((
            "code_file",
            importer_id
                .strip_prefix("code_file:")
                .unwrap_or(importer_id),
        ));
        let to = Thing::from((
            "code_file",
            imported_id
                .strip_prefix("code_file:")
                .unwrap_or(imported_id),
        ));
        self.db
            .query("RELATE $from->imports->$to SET import_path = $path")
            .bind(("from", from))
            .bind(("to", to))
            .bind(("path", import_path.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Create a `defines` edge from a code file to a symbol.
    pub async fn create_defines_edge(
        &self,
        file_id: &str,
        symbol_table: &str,
        symbol_id: &str,
    ) -> Result<(), EngramError> {
        let from = Thing::from((
            "code_file",
            file_id.strip_prefix("code_file:").unwrap_or(file_id),
        ));
        let prefix = format!("{symbol_table}:");
        let to = Thing::from((
            symbol_table,
            symbol_id.strip_prefix(&prefix).unwrap_or(symbol_id),
        ));
        self.db
            .query("RELATE $from->defines->$to")
            .bind(("from", from))
            .bind(("to", to))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Create an `inherits_from` edge from class to class/interface.
    pub async fn create_inherits_edge(
        &self,
        child_table: &str,
        child_id: &str,
        parent_table: &str,
        parent_id: &str,
    ) -> Result<(), EngramError> {
        let child_prefix = format!("{child_table}:");
        let parent_prefix = format!("{parent_table}:");
        let from = Thing::from((
            child_table,
            child_id.strip_prefix(&child_prefix).unwrap_or(child_id),
        ));
        let to = Thing::from((
            parent_table,
            parent_id.strip_prefix(&parent_prefix).unwrap_or(parent_id),
        ));
        self.db
            .query("RELATE $from->inherits_from->$to")
            .bind(("from", from))
            .bind(("to", to))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Create a `concerns` edge from a task to a code symbol.
    pub async fn create_concerns_edge(
        &self,
        task_id: &str,
        symbol_table: &str,
        symbol_id: &str,
        linked_by: &str,
    ) -> Result<(), EngramError> {
        let sym_prefix = format!("{symbol_table}:");
        let from = Thing::from(("task", task_id.strip_prefix("task:").unwrap_or(task_id)));
        let to = Thing::from((
            symbol_table,
            symbol_id.strip_prefix(&sym_prefix).unwrap_or(symbol_id),
        ));
        self.db
            .query("RELATE $from->concerns->$to SET linked_by = $by")
            .bind(("from", from))
            .bind(("to", to))
            .bind(("by", linked_by.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Delete all edges of a given type originating from a code file.
    pub async fn delete_edges_from_file(
        &self,
        edge_table: &str,
        file_id: &str,
    ) -> Result<(), EngramError> {
        let from = Thing::from((
            "code_file",
            file_id.strip_prefix("code_file:").unwrap_or(file_id),
        ));
        let query = format!("DELETE FROM {edge_table} WHERE in = $from");
        self.db
            .query(&query)
            .bind(("from", from))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Delete all symbols and edges for a file (used during re-indexing).
    pub async fn clear_file_graph(&self, file_path: &str) -> Result<(), EngramError> {
        self.delete_functions_by_file(file_path).await?;
        self.delete_classes_by_file(file_path).await?;
        self.delete_interfaces_by_file(file_path).await?;
        Ok(())
    }

    // ── Concerns Edge Management (T044) ─────────────────────────────

    /// Retrieve all `concerns` edges whose target (`out`) is a symbol in the
    /// given file path. Returns `(task_id, symbol_table, symbol_id, linked_by)`
    /// tuples for every matching edge.
    pub async fn get_concerns_edges_for_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<ConcernsEdgeInfo>, EngramError> {
        let mut results = Vec::new();

        // Collect symbol IDs from this file across all symbol tables.
        for table in &["function", "class", "interface"] {
            let table_name = if *table == "function" {
                "`function`"
            } else {
                *table
            };
            let sym_query = format!("SELECT id FROM {table_name} WHERE file_path = $fp");
            let mut sym_resp = self
                .db
                .query(&sym_query)
                .bind(("fp", file_path.to_owned()))
                .await
                .map_err(map_db_err)?;
            let sym_ids: Vec<IdOnlyRow> = sym_resp.take(0).map_err(map_db_err)?;

            for sym_row in sym_ids {
                let sym_thing = sym_row.id.clone();
                let mut edge_resp = self
                    .db
                    .query("SELECT * FROM concerns WHERE out = $sym")
                    .bind(("sym", sym_thing.clone()))
                    .await
                    .map_err(map_db_err)?;
                let edge_rows: Vec<ConcernsRow> = edge_resp.take(0).map_err(map_db_err)?;

                for edge in edge_rows {
                    results.push(ConcernsEdgeInfo {
                        task_id: format!("{}:{}", edge.r#in.tb, edge.r#in.id.to_raw()),
                        symbol_table: (*table).to_string(),
                        symbol_id: format!("{}:{}", sym_thing.tb, sym_thing.id.to_raw()),
                        symbol_name: String::new(), // will be filled by caller
                        symbol_body_hash: String::new(), // will be filled by caller
                        linked_by: edge.linked_by.unwrap_or_default(),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Delete all `concerns` edges targeting a specific symbol node.
    pub async fn delete_concerns_edges_for_symbol(
        &self,
        symbol_table: &str,
        symbol_id: &str,
    ) -> Result<usize, EngramError> {
        let prefix = format!("{symbol_table}:");
        let thing = Thing::from((
            symbol_table,
            symbol_id.strip_prefix(&prefix).unwrap_or(symbol_id),
        ));
        let mut resp = self
            .db
            .query("SELECT * FROM concerns WHERE out = $sym")
            .bind(("sym", thing.clone()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<ConcernsRow> = resp.take(0).map_err(map_db_err)?;
        let count = rows.len();

        self.db
            .query("DELETE FROM concerns WHERE out = $sym")
            .bind(("sym", thing))
            .await
            .map_err(map_db_err)?;

        Ok(count)
    }

    /// Look up all symbols with a given `(name, body_hash)` across all symbol
    /// tables. Used for hash-resilient concerns edge relinking (FR-124).
    pub async fn find_symbols_by_name_and_hash(
        &self,
        name: &str,
        body_hash: &str,
    ) -> Result<Vec<SymbolIdentity>, EngramError> {
        let mut results = Vec::new();

        for table in &["function", "class", "interface"] {
            let table_name = if *table == "function" {
                "`function`"
            } else {
                *table
            };
            let query = format!(
                "SELECT id, name, file_path, body_hash FROM {table_name} WHERE name = $name AND body_hash = $bh"
            );
            let mut resp = self
                .db
                .query(&query)
                .bind(("name", name.to_owned()))
                .bind(("bh", body_hash.to_owned()))
                .await
                .map_err(map_db_err)?;
            let rows: Vec<SymbolIdentityRow> = resp.take(0).map_err(map_db_err)?;
            for row in rows {
                results.push(SymbolIdentity {
                    table: (*table).to_string(),
                    id: format!("{}:{}", row.id.tb, row.id.id.to_raw()),
                    name: row.name,
                    file_path: row.file_path,
                    body_hash: row.body_hash,
                });
            }
        }

        Ok(results)
    }

    /// Get all symbols (name + body_hash) in a given file, for pre-sync
    /// snapshot used by hash-resilient concerns relinking.
    pub async fn get_symbol_identities_for_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<SymbolIdentity>, EngramError> {
        let mut results = Vec::new();

        for table in &["function", "class", "interface"] {
            let table_name = if *table == "function" {
                "`function`"
            } else {
                *table
            };
            let query = format!(
                "SELECT id, name, file_path, body_hash FROM {table_name} WHERE file_path = $fp"
            );
            let mut resp = self
                .db
                .query(&query)
                .bind(("fp", file_path.to_owned()))
                .await
                .map_err(map_db_err)?;
            let rows: Vec<SymbolIdentityRow> = resp.take(0).map_err(map_db_err)?;
            for row in rows {
                results.push(SymbolIdentity {
                    table: (*table).to_string(),
                    id: format!("{}:{}", row.id.tb, row.id.id.to_raw()),
                    name: row.name,
                    file_path: row.file_path,
                    body_hash: row.body_hash,
                });
            }
        }

        Ok(results)
    }

    // ── Concerns Edge CRUD for link/unlink (T049) ───────────────────

    /// Check if a `concerns` edge already exists between task and symbol (FR-152 idempotency).
    pub async fn concerns_edge_exists(
        &self,
        task_id: &str,
        symbol_table: &str,
        symbol_id: &str,
    ) -> Result<bool, EngramError> {
        let from = Thing::from(("task", task_id.strip_prefix("task:").unwrap_or(task_id)));
        let sym_prefix = format!("{symbol_table}:");
        let to = Thing::from((
            symbol_table,
            symbol_id.strip_prefix(&sym_prefix).unwrap_or(symbol_id),
        ));
        let mut resp = self
            .db
            .query("SELECT count() AS count FROM concerns WHERE in = $from AND out = $to GROUP ALL")
            .bind(("from", from))
            .bind(("to", to))
            .await
            .map_err(map_db_err)?;
        let row: Option<CountRow> = resp.take(0).map_err(map_db_err)?;
        Ok(row.is_some_and(|r| r.count > 0))
    }

    /// Delete all `concerns` edges from a task to symbols with the given name.
    ///
    /// Returns the number of edges deleted.
    pub async fn delete_concerns_by_task_and_symbol_name(
        &self,
        task_id: &str,
        symbol_name: &str,
    ) -> Result<usize, EngramError> {
        let from = Thing::from(("task", task_id.strip_prefix("task:").unwrap_or(task_id)));

        // Find all symbol IDs matching the name across tables.
        let symbols = self.find_symbols_by_name(symbol_name).await?;
        let mut deleted = 0;
        for sym in &symbols {
            let (table, raw_id) = sym
                .id
                .split_once(':')
                .unwrap_or(("function", sym.id.as_str()));
            let to = Thing::from((table, raw_id));
            let mut resp = self
                .db
                .query("SELECT * FROM concerns WHERE in = $from AND out = $to")
                .bind(("from", from.clone()))
                .bind(("to", to.clone()))
                .await
                .map_err(map_db_err)?;
            let rows: Vec<ConcernsRow> = resp.take(0).map_err(map_db_err)?;
            if !rows.is_empty() {
                self.db
                    .query("DELETE FROM concerns WHERE in = $from AND out = $to")
                    .bind(("from", from.clone()))
                    .bind(("to", to))
                    .await
                    .map_err(map_db_err)?;
                deleted += rows.len();
            }
        }
        Ok(deleted)
    }

    /// List all `concerns` edges for a given task, returning symbol info.
    pub async fn list_concerns_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<ConcernsLink>, EngramError> {
        let from = Thing::from(("task", task_id.strip_prefix("task:").unwrap_or(task_id)));
        let mut resp = self
            .db
            .query("SELECT * FROM concerns WHERE in = $from")
            .bind(("from", from))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<ConcernsLinkRow> = resp.take(0).map_err(map_db_err)?;

        let mut results = Vec::new();
        for row in rows {
            let symbol_id = format!("{}:{}", row.out.tb, row.out.id.to_raw());
            let symbol_table = row.out.tb.clone();

            // Resolve the symbol to get its name and file_path.
            let (name, file_path) = if let Some(sym) = self.resolve_symbol(&symbol_id).await? {
                (sym.name, sym.file_path)
            } else {
                (String::new(), String::new())
            };

            results.push(ConcernsLink {
                symbol_table,
                symbol_id,
                symbol_name: name,
                file_path,
                linked_by: row.linked_by.unwrap_or_default(),
            });
        }
        Ok(results)
    }

    /// Reverse-lookup: given a set of symbol IDs, find all task IDs linked via
    /// `concerns` edges (task → symbol direction, queried in reverse).
    ///
    /// Returns `(task_id, symbol_id)` pairs so callers can build dependency paths.
    pub async fn find_tasks_for_symbols(
        &self,
        symbol_ids: &[String],
    ) -> Result<Vec<(String, String)>, EngramError> {
        let mut results: Vec<(String, String)> = Vec::new();
        for sym_id in symbol_ids {
            let parts: Vec<&str> = sym_id.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }
            let thing = Thing::from((parts[0], parts[1]));
            let mut resp = self
                .db
                .query("SELECT * FROM concerns WHERE out = $sym")
                .bind(("sym", thing))
                .await
                .map_err(map_db_err)?;
            let rows: Vec<ConcernsRow> = resp.take(0).map_err(map_db_err)?;
            for row in rows {
                let task_id = format!("{}:{}", row.r#in.tb, row.r#in.id.to_raw());
                results.push((task_id, sym_id.clone()));
            }
        }
        Ok(results)
    }

    // ── BFS Traversal Queries (T038) ────────────────────────────────

    /// Look up all symbols (functions, classes, interfaces) whose name matches exactly.
    ///
    /// Returns a vec of `(table, id, name, file_path)` tuples across all symbol tables.
    pub async fn find_symbols_by_name(&self, name: &str) -> Result<Vec<SymbolMatch>, EngramError> {
        let mut results = Vec::new();

        // Query functions
        let mut resp = self
            .db
            .query("SELECT * FROM `function` WHERE name = $name")
            .bind(("name", name.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<FunctionRow> = resp.take(0).map_err(map_db_err)?;
        for row in rows {
            let func = row.into_function();
            results.push(SymbolMatch {
                table: "function".to_owned(),
                id: func.id,
                name: func.name,
                file_path: func.file_path,
                line_start: Some(func.line_start),
                line_end: Some(func.line_end),
                signature: Some(func.signature),
                body: func.body,
                embed_type: Some(func.embed_type),
                summary: Some(func.summary),
                embedding: func.embedding,
            });
        }

        // Query classes
        let mut resp = self
            .db
            .query("SELECT * FROM class WHERE name = $name")
            .bind(("name", name.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<ClassRow> = resp.take(0).map_err(map_db_err)?;
        for row in rows {
            let cls = row.into_class();
            results.push(SymbolMatch {
                table: "class".to_owned(),
                id: cls.id,
                name: cls.name,
                file_path: cls.file_path,
                line_start: Some(cls.line_start),
                line_end: Some(cls.line_end),
                signature: None,
                body: cls.body,
                embed_type: Some(cls.embed_type),
                summary: Some(cls.summary),
                embedding: cls.embedding,
            });
        }

        // Query interfaces
        let mut resp = self
            .db
            .query("SELECT * FROM interface WHERE name = $name")
            .bind(("name", name.to_owned()))
            .await
            .map_err(map_db_err)?;
        let rows: Vec<InterfaceRow> = resp.take(0).map_err(map_db_err)?;
        for row in rows {
            let iface = row.into_interface();
            results.push(SymbolMatch {
                table: "interface".to_owned(),
                id: iface.id,
                name: iface.name,
                file_path: iface.file_path,
                line_start: Some(iface.line_start),
                line_end: Some(iface.line_end),
                signature: None,
                body: iface.body,
                embed_type: Some(iface.embed_type),
                summary: Some(iface.summary),
                embedding: iface.embedding,
            });
        }

        Ok(results)
    }

    /// BFS traversal from a symbol node, collecting neighbors up to `max_depth`
    /// hops and capping at `max_nodes` total results.
    ///
    /// Returns the list of neighbor nodes (excluding the root) and edges.
    pub async fn bfs_neighborhood(
        &self,
        root_id: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<BfsResult, EngramError> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        let mut neighbors: Vec<SymbolMatch> = Vec::new();
        let mut edges: Vec<BfsEdge> = Vec::new();

        visited.insert(root_id.to_owned());
        queue.push_back((root_id.to_owned(), 0));

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            // Get outbound edges (current -> other)
            let outbound = self.get_outbound_edges(&current_id).await?;
            for (edge_type, target_id) in &outbound {
                if visited.contains(target_id) {
                    continue;
                }
                if neighbors.len() >= max_nodes {
                    return Ok(BfsResult {
                        neighbors,
                        edges,
                        truncated: true,
                    });
                }
                visited.insert(target_id.clone());
                if let Some(sym) = self.resolve_symbol(target_id).await? {
                    neighbors.push(sym);
                    edges.push(BfsEdge {
                        edge_type: edge_type.clone(),
                        from: current_id.clone(),
                        to: target_id.clone(),
                    });
                    queue.push_back((target_id.clone(), depth + 1));
                }
            }

            // Get inbound edges (other -> current)
            let inbound = self.get_inbound_edges(&current_id).await?;
            for (edge_type, source_id) in &inbound {
                if visited.contains(source_id) {
                    continue;
                }
                if neighbors.len() >= max_nodes {
                    return Ok(BfsResult {
                        neighbors,
                        edges,
                        truncated: true,
                    });
                }
                visited.insert(source_id.clone());
                if let Some(sym) = self.resolve_symbol(source_id).await? {
                    neighbors.push(sym);
                    edges.push(BfsEdge {
                        edge_type: edge_type.clone(),
                        from: source_id.clone(),
                        to: current_id.clone(),
                    });
                    queue.push_back((source_id.clone(), depth + 1));
                }
            }
        }

        Ok(BfsResult {
            neighbors,
            edges,
            truncated: false,
        })
    }

    /// Get outbound code edges from a node (calls, imports, defines, inherits_from).
    async fn get_outbound_edges(
        &self,
        node_id: &str,
    ) -> Result<Vec<(String, String)>, EngramError> {
        let (table, raw_id) = parse_node_id(node_id);
        let record = Thing::from((table.as_str(), raw_id.as_str()));
        let mut results = Vec::new();

        for edge_table in &["calls", "imports", "defines", "inherits_from"] {
            let query = format!("SELECT out FROM {edge_table} WHERE in = $node");
            let mut resp = self
                .db
                .query(&query)
                .bind(("node", record.clone()))
                .await
                .map_err(map_db_err)?;
            let rows: Vec<OutEdgeRow> = resp.take(0).map_err(map_db_err)?;
            for row in rows {
                let target_id = format!("{}:{}", row.out.tb, row.out.id.to_raw());
                results.push(((*edge_table).to_owned(), target_id));
            }
        }

        Ok(results)
    }

    /// Get inbound code edges to a node.
    async fn get_inbound_edges(&self, node_id: &str) -> Result<Vec<(String, String)>, EngramError> {
        let (table, raw_id) = parse_node_id(node_id);
        let record = Thing::from((table.as_str(), raw_id.as_str()));
        let mut results = Vec::new();

        for edge_table in &["calls", "imports", "defines", "inherits_from"] {
            let query = format!("SELECT in FROM {edge_table} WHERE out = $node");
            let mut resp = self
                .db
                .query(&query)
                .bind(("node", record.clone()))
                .await
                .map_err(map_db_err)?;
            let rows: Vec<InEdgeRow> = resp.take(0).map_err(map_db_err)?;
            for row in rows {
                let source_id = format!("{}:{}", row.r#in.tb, row.r#in.id.to_raw());
                results.push(((*edge_table).to_owned(), source_id));
            }
        }

        Ok(results)
    }

    /// Resolve any symbol ID to its full metadata.
    pub async fn resolve_symbol(&self, node_id: &str) -> Result<Option<SymbolMatch>, EngramError> {
        let (table, _raw_id) = parse_node_id(node_id);

        match table.as_str() {
            "function" => {
                let mut resp = self
                    .db
                    .query("SELECT * FROM $id")
                    .bind(("id", parse_thing(node_id)))
                    .await
                    .map_err(map_db_err)?;
                let rows: Vec<FunctionRow> = resp.take(0).map_err(map_db_err)?;
                Ok(rows.into_iter().next().map(|r| {
                    let f = r.into_function();
                    SymbolMatch {
                        table: "function".to_owned(),
                        id: f.id,
                        name: f.name,
                        file_path: f.file_path,
                        line_start: Some(f.line_start),
                        line_end: Some(f.line_end),
                        signature: Some(f.signature),
                        body: f.body,
                        embed_type: Some(f.embed_type),
                        summary: Some(f.summary),
                        embedding: f.embedding,
                    }
                }))
            }
            "class" => {
                let mut resp = self
                    .db
                    .query("SELECT * FROM $id")
                    .bind(("id", parse_thing(node_id)))
                    .await
                    .map_err(map_db_err)?;
                let rows: Vec<ClassRow> = resp.take(0).map_err(map_db_err)?;
                Ok(rows.into_iter().next().map(|r| {
                    let c = r.into_class();
                    SymbolMatch {
                        table: "class".to_owned(),
                        id: c.id,
                        name: c.name,
                        file_path: c.file_path,
                        line_start: Some(c.line_start),
                        line_end: Some(c.line_end),
                        signature: None,
                        body: c.body,
                        embed_type: Some(c.embed_type),
                        summary: Some(c.summary),
                        embedding: c.embedding,
                    }
                }))
            }
            "interface" => {
                let mut resp = self
                    .db
                    .query("SELECT * FROM $id")
                    .bind(("id", parse_thing(node_id)))
                    .await
                    .map_err(map_db_err)?;
                let rows: Vec<InterfaceRow> = resp.take(0).map_err(map_db_err)?;
                Ok(rows.into_iter().next().map(|r| {
                    let i = r.into_interface();
                    SymbolMatch {
                        table: "interface".to_owned(),
                        id: i.id,
                        name: i.name,
                        file_path: i.file_path,
                        line_start: Some(i.line_start),
                        line_end: Some(i.line_end),
                        signature: None,
                        body: i.body,
                        embed_type: Some(i.embed_type),
                        summary: Some(i.summary),
                        embedding: i.embedding,
                    }
                }))
            }
            "code_file" => {
                let mut resp = self
                    .db
                    .query("SELECT * FROM $id")
                    .bind(("id", parse_thing(node_id)))
                    .await
                    .map_err(map_db_err)?;
                let rows: Vec<CodeFileRow> = resp.take(0).map_err(map_db_err)?;
                Ok(rows.into_iter().next().map(|r| {
                    let cf = r.into_code_file();
                    SymbolMatch {
                        table: "code_file".to_owned(),
                        id: cf.id,
                        name: cf.path.clone(),
                        file_path: cf.path,
                        line_start: None,
                        line_end: None,
                        signature: None,
                        body: String::new(),
                        embed_type: None,
                        summary: None,
                        embedding: Vec::new(),
                    }
                }))
            }
            _ => Ok(None),
        }
    }

    // ── Symbol Listing Queries (T038) ───────────────────────────────

    /// List symbols with optional filtering and pagination.
    ///
    /// Filters by `file_path`, `node_type` (function/class/interface),
    /// and `name_prefix`. Returns paginated results with total count.
    pub async fn list_symbols(
        &self,
        filter: &SymbolFilter,
    ) -> Result<SymbolListResult, EngramError> {
        let mut all_symbols: Vec<SymbolListEntry> = Vec::new();

        let tables = match filter.node_type.as_deref() {
            Some("function") => vec!["function"],
            Some("class") => vec!["class"],
            Some("interface") => vec!["interface"],
            _ => vec!["function", "class", "interface"],
        };

        for table in &tables {
            let mut conditions = Vec::new();
            let mut has_file_path = false;
            let mut has_prefix = false;

            if filter.file_path.is_some() {
                conditions.push("file_path = $fp");
                has_file_path = true;
            }
            if filter.name_prefix.is_some() {
                conditions.push("name CONTAINS $prefix OR string::starts_with(name, $prefix)");
                has_prefix = true;
            }

            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!(" WHERE {}", conditions.join(" AND "))
            };

            // Backtick the function table name since it is a reserved keyword
            let table_name = if *table == "function" {
                "`function`"
            } else {
                table
            };

            // Count query
            // Count query is done globally in count_all_symbols; skip per-table count here.
            let table_count: u64 = 0;

            // Data query with pagination
            let data_query = format!(
                "SELECT * FROM {table_name}{where_clause} ORDER BY name ASC LIMIT $lim START $off"
            );
            let mut data_qb = self.db.query(&data_query);
            if has_file_path {
                data_qb = data_qb.bind(("fp", filter.file_path.clone().unwrap_or_default()));
            }
            if has_prefix {
                data_qb = data_qb.bind(("prefix", filter.name_prefix.clone().unwrap_or_default()));
            }
            #[allow(clippy::cast_possible_wrap)]
            let limit_i64 = filter.limit as i64;
            #[allow(clippy::cast_possible_wrap)]
            let offset_i64 = filter.offset as i64;
            data_qb = data_qb.bind(("lim", limit_i64)).bind(("off", offset_i64));

            let mut data_resp = data_qb.await.map_err(map_db_err)?;

            match *table {
                "function" => {
                    let rows: Vec<FunctionRow> = data_resp.take(0).map_err(map_db_err)?;
                    for row in rows {
                        let f = row.into_function();
                        all_symbols.push(SymbolListEntry {
                            name: f.name,
                            node_type: "function".to_owned(),
                            file_path: f.file_path,
                            line_start: Some(f.line_start),
                            line_end: Some(f.line_end),
                        });
                    }
                }
                "class" => {
                    let rows: Vec<ClassRow> = data_resp.take(0).map_err(map_db_err)?;
                    for row in rows {
                        let c = row.into_class();
                        all_symbols.push(SymbolListEntry {
                            name: c.name,
                            node_type: "class".to_owned(),
                            file_path: c.file_path,
                            line_start: Some(c.line_start),
                            line_end: Some(c.line_end),
                        });
                    }
                }
                "interface" => {
                    let rows: Vec<InterfaceRow> = data_resp.take(0).map_err(map_db_err)?;
                    for row in rows {
                        let i = row.into_interface();
                        all_symbols.push(SymbolListEntry {
                            name: i.name,
                            node_type: "interface".to_owned(),
                            file_path: i.file_path,
                            line_start: Some(i.line_start),
                            line_end: Some(i.line_end),
                        });
                    }
                }
                _ => {}
            }

            // Accumulate total count across tables (for unfiltered or multi-table queries)
            // Note: offset/limit apply per table in this implementation; for cross-table
            // pagination the total_count reflects the sum.
            all_symbols.sort_by(|a, b| a.name.cmp(&b.name));
            let _ = table_count; // used below
        }

        // Calculate total count across all queried tables
        let total = self.count_all_symbols(filter).await?;

        let has_more = (filter.offset + filter.limit) < total;

        Ok(SymbolListResult {
            symbols: all_symbols,
            total_count: total,
            has_more,
        })
    }

    /// Count total symbols matching the filter across all relevant tables.
    async fn count_all_symbols(&self, filter: &SymbolFilter) -> Result<usize, EngramError> {
        let tables = match filter.node_type.as_deref() {
            Some("function") => vec!["function"],
            Some("class") => vec!["class"],
            Some("interface") => vec!["interface"],
            _ => vec!["function", "class", "interface"],
        };

        let mut total: usize = 0;
        for table in &tables {
            let mut conditions = Vec::new();
            let mut has_file_path = false;
            let mut has_prefix = false;

            if filter.file_path.is_some() {
                conditions.push("file_path = $fp");
                has_file_path = true;
            }
            if filter.name_prefix.is_some() {
                conditions.push("name CONTAINS $prefix OR string::starts_with(name, $prefix)");
                has_prefix = true;
            }

            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!(" WHERE {}", conditions.join(" AND "))
            };

            let table_name = if *table == "function" {
                "`function`"
            } else {
                table
            };

            let query = format!("SELECT count() FROM {table_name}{where_clause} GROUP ALL");
            let mut qb = self.db.query(&query);
            if has_file_path {
                qb = qb.bind(("fp", filter.file_path.clone().unwrap_or_default()));
            }
            if has_prefix {
                qb = qb.bind(("prefix", filter.name_prefix.clone().unwrap_or_default()));
            }
            let mut resp = qb.await.map_err(map_db_err)?;
            let rows: Vec<CountRow> = resp.take(0).map_err(map_db_err)?;
            #[allow(clippy::cast_possible_truncation)]
            {
                total += rows.first().map_or(0, |r| r.count) as usize;
            }
        }

        Ok(total)
    }

    /// Vector search across all symbol embeddings. Returns up to `limit` nearest
    /// matches by cosine similarity.
    ///
    /// Delegates to [`vector_search_symbols_native`] and strips scores.
    pub async fn vector_search_symbols(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SymbolMatch>, EngramError> {
        let with_scores = self
            .vector_search_symbols_native(query_embedding, limit)
            .await?;
        Ok(with_scores.into_iter().map(|(_, m)| m).collect())
    }

    // ── Native KNN vector search (dxo.2.1 / dxo.2.3) ─────────────

    /// Vector search using `SurrealDB`'s native KNN operator with MTREE indexes.
    ///
    /// Replaces the O(n) full-table-scan approach in [`vector_search_symbols`]
    /// with O(log n) index-backed queries using `<|K,COSINE|>`.
    ///
    /// Scores are `SurrealDB`-authoritative: computed via
    /// `vector::similarity::cosine(embedding, $query)` in the SELECT clause.
    ///
    /// Returns up to `limit` nearest matches with cosine similarity scores.
    ///
    /// # Errors
    ///
    /// Returns `EngramError` if any database query fails.
    pub async fn vector_search_symbols_native(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(f32, SymbolMatch)>, EngramError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let mut results: Vec<(f32, SymbolMatch)> = Vec::new();
        let query_vec = query_embedding.to_vec();

        // ── Functions — KNN + DB-computed score ──
        let func_sql = format!(
            "SELECT *, vector::similarity::cosine(embedding, $query) AS knn_score \
             FROM `function` WHERE embedding <|{limit},COSINE|> $query"
        );
        let mut resp = self
            .db
            .query(&func_sql)
            .bind(("query", query_vec.clone()))
            .await
            .map_err(map_db_err)?;
        let func_rows: Vec<FunctionRow> = resp.take(0).map_err(map_db_err)?;
        for row in func_rows {
            let score = row.knn_score.unwrap_or(0.0).clamp(0.0, 1.0);
            let f = row.into_function();
            results.push((
                score,
                SymbolMatch {
                    table: "function".to_owned(),
                    id: f.id,
                    name: f.name,
                    file_path: f.file_path,
                    line_start: Some(f.line_start),
                    line_end: Some(f.line_end),
                    signature: Some(f.signature),
                    body: f.body,
                    embed_type: Some(f.embed_type),
                    summary: Some(f.summary),
                    embedding: f.embedding,
                },
            ));
        }

        // ── Classes — KNN + DB-computed score ──
        let class_sql = format!(
            "SELECT *, vector::similarity::cosine(embedding, $query) AS knn_score \
             FROM class WHERE embedding <|{limit},COSINE|> $query"
        );
        let mut resp = self
            .db
            .query(&class_sql)
            .bind(("query", query_vec.clone()))
            .await
            .map_err(map_db_err)?;
        let class_rows: Vec<ClassRow> = resp.take(0).map_err(map_db_err)?;
        for row in class_rows {
            let score = row.knn_score.unwrap_or(0.0).clamp(0.0, 1.0);
            let c = row.into_class();
            results.push((
                score,
                SymbolMatch {
                    table: "class".to_owned(),
                    id: c.id,
                    name: c.name,
                    file_path: c.file_path,
                    line_start: Some(c.line_start),
                    line_end: Some(c.line_end),
                    signature: None,
                    body: c.body,
                    embed_type: Some(c.embed_type),
                    summary: Some(c.summary),
                    embedding: c.embedding,
                },
            ));
        }

        // ── Interfaces — KNN + DB-computed score ──
        let iface_sql = format!(
            "SELECT *, vector::similarity::cosine(embedding, $query) AS knn_score \
             FROM interface WHERE embedding <|{limit},COSINE|> $query"
        );
        let mut resp = self
            .db
            .query(&iface_sql)
            .bind(("query", query_vec))
            .await
            .map_err(map_db_err)?;
        let iface_rows: Vec<InterfaceRow> = resp.take(0).map_err(map_db_err)?;
        for row in iface_rows {
            let score = row.knn_score.unwrap_or(0.0).clamp(0.0, 1.0);
            let i = row.into_interface();
            results.push((
                score,
                SymbolMatch {
                    table: "interface".to_owned(),
                    id: i.id,
                    name: i.name,
                    file_path: i.file_path,
                    line_start: Some(i.line_start),
                    line_end: Some(i.line_end),
                    signature: None,
                    body: i.body,
                    embed_type: Some(i.embed_type),
                    summary: Some(i.summary),
                    embedding: i.embedding,
                },
            ));
        }

        // Merge results across tables, sort by score descending, take top limit
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    // ── Native graph traversal (dxo.1.1) ─────────────────────────

    /// Traverse the code graph using native SurrealQL `->edge->` / `<-edge<-`
    /// syntax instead of the manual BFS in [`bfs_neighborhood`].
    ///
    /// Issues a single batched SurrealQL query per hop covering all edge types
    /// (calls, imports, defines, `inherits_from`, concerns) in both directions,
    /// replacing the N×5 round-trip pattern of the original BFS.
    ///
    /// Returns the same [`BfsResult`] shape for backward compatibility.
    ///
    /// # Errors
    ///
    /// Returns `EngramError` if the database query fails.
    pub async fn graph_neighborhood(
        &self,
        root_id: &str,
        max_depth: usize,
        max_nodes: usize,
    ) -> Result<BfsResult, EngramError> {
        // Edge types traversed in both directions per hop.
        const EDGE_TABLES: [&str; 5] = ["calls", "imports", "defines", "inherits_from", "concerns"];

        let mut visited: HashSet<String> = HashSet::new();
        let mut frontier: Vec<String> = Vec::new();
        let mut neighbors: Vec<SymbolMatch> = Vec::new();
        let mut edges: Vec<BfsEdge> = Vec::new();
        let mut truncated = false;

        visited.insert(root_id.to_owned());
        frontier.push(root_id.to_owned());

        for _depth in 0..max_depth {
            if frontier.is_empty() {
                break;
            }

            let mut next_frontier: Vec<String> = Vec::new();

            for current_id in &frontier {
                if truncated {
                    break;
                }

                let record = parse_thing(current_id);

                // Single batched query: 10 statements (5 outbound + 5 inbound)
                // covering all edge types in one database round-trip.
                let mut resp = self
                    .db
                    .query(
                        "SELECT out FROM calls WHERE in = $node;\
                         SELECT out FROM imports WHERE in = $node;\
                         SELECT out FROM defines WHERE in = $node;\
                         SELECT out FROM inherits_from WHERE in = $node;\
                         SELECT out FROM concerns WHERE in = $node;\
                         SELECT in FROM calls WHERE out = $node;\
                         SELECT in FROM imports WHERE out = $node;\
                         SELECT in FROM defines WHERE out = $node;\
                         SELECT in FROM inherits_from WHERE out = $node;\
                         SELECT in FROM concerns WHERE out = $node;",
                    )
                    .bind(("node", record))
                    .await
                    .map_err(map_db_err)?;

                // Process outbound edges (statements 0..5).
                for (idx, edge_type) in EDGE_TABLES.iter().enumerate() {
                    let rows: Vec<OutEdgeRow> = resp.take(idx).map_err(map_db_err)?;
                    for row in rows {
                        let target_id = format!("{}:{}", row.out.tb, row.out.id.to_raw());
                        truncated = !self
                            .try_add_neighbor(
                                current_id,
                                &target_id,
                                edge_type,
                                true,
                                max_nodes,
                                &mut visited,
                                &mut neighbors,
                                &mut edges,
                                &mut next_frontier,
                            )
                            .await?;
                        if truncated {
                            break;
                        }
                    }
                    if truncated {
                        break;
                    }
                }

                // Process inbound edges (statements 5..10).
                if !truncated {
                    for (idx, edge_type) in EDGE_TABLES.iter().enumerate() {
                        let rows: Vec<InEdgeRow> = resp.take(5 + idx).map_err(map_db_err)?;
                        for row in rows {
                            let source_id = format!("{}:{}", row.r#in.tb, row.r#in.id.to_raw());
                            truncated = !self
                                .try_add_neighbor(
                                    current_id,
                                    &source_id,
                                    edge_type,
                                    false,
                                    max_nodes,
                                    &mut visited,
                                    &mut neighbors,
                                    &mut edges,
                                    &mut next_frontier,
                                )
                                .await?;
                            if truncated {
                                break;
                            }
                        }
                        if truncated {
                            break;
                        }
                    }
                }
            }

            frontier = next_frontier;
        }

        Ok(BfsResult {
            neighbors,
            edges,
            truncated,
        })
    }

    /// Try to add a neighbor discovered during [`graph_neighborhood`] traversal.
    ///
    /// Returns `Ok(true)` if the neighbor was added (or already visited),
    /// `Ok(false)` if `max_nodes` was reached and traversal should stop.
    #[allow(clippy::too_many_arguments)]
    async fn try_add_neighbor(
        &self,
        current_id: &str,
        neighbor_id: &str,
        edge_type: &str,
        is_outbound: bool,
        max_nodes: usize,
        visited: &mut HashSet<String>,
        neighbors: &mut Vec<SymbolMatch>,
        edges: &mut Vec<BfsEdge>,
        next_frontier: &mut Vec<String>,
    ) -> Result<bool, EngramError> {
        if visited.contains(neighbor_id) {
            return Ok(true);
        }
        if neighbors.len() >= max_nodes {
            return Ok(false);
        }

        visited.insert(neighbor_id.to_owned());

        if let Some(sym) = self.resolve_symbol(neighbor_id).await? {
            neighbors.push(sym);

            let (from, to) = if is_outbound {
                (current_id.to_owned(), neighbor_id.to_owned())
            } else {
                (neighbor_id.to_owned(), current_id.to_owned())
            };

            edges.push(BfsEdge {
                edge_type: edge_type.to_owned(),
                from,
                to,
            });

            next_frontier.push(neighbor_id.to_owned());
        }

        Ok(true)
    }

    // ── Embedding write-back (T076/T077) ────────────────────────────

    /// Update the embedding vector for any symbol node by its full ID (e.g., `"function:abc"`).
    pub async fn update_symbol_embedding(
        &self,
        sym_id: &str,
        embedding: Vec<f32>,
    ) -> Result<(), EngramError> {
        let (table, raw_id) = sym_id.split_once(':').unwrap_or(("", sym_id));
        let record = Thing::from((table, raw_id));
        self.db
            .query("UPDATE $id SET embedding = $emb")
            .bind(("id", record))
            .bind(("emb", embedding))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    // ── COUNT queries (T094) ─────────────────────────────────────────

    /// Return the total count of indexed code files.
    pub async fn count_code_files(&self) -> Result<u64, EngramError> {
        let mut resp = self
            .db
            .query("SELECT count() AS count FROM code_file GROUP ALL")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CountRow> = resp.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map_or(0, |r| r.count))
    }

    /// Return the total count of indexed functions.
    pub async fn count_functions(&self) -> Result<u64, EngramError> {
        let mut resp = self
            .db
            .query("SELECT count() AS count FROM `function` GROUP ALL")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CountRow> = resp.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map_or(0, |r| r.count))
    }

    /// Return the total count of indexed classes.
    pub async fn count_classes(&self) -> Result<u64, EngramError> {
        let mut resp = self
            .db
            .query("SELECT count() AS count FROM class GROUP ALL")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CountRow> = resp.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map_or(0, |r| r.count))
    }

    /// Return the total count of indexed interfaces.
    pub async fn count_interfaces(&self) -> Result<u64, EngramError> {
        let mut resp = self
            .db
            .query("SELECT count() AS count FROM interface GROUP ALL")
            .await
            .map_err(map_db_err)?;
        let rows: Vec<CountRow> = resp.take(0).map_err(map_db_err)?;
        Ok(rows.into_iter().next().map_or(0, |r| r.count))
    }

    /// Return the total count of code edges across all edge types.
    pub async fn count_code_edges(&self) -> Result<u64, EngramError> {
        let mut total = 0u64;
        for table in &["calls", "imports", "defines", "inherits_from", "concerns"] {
            let query = format!("SELECT count() AS count FROM {table} GROUP ALL");
            let mut resp = self.db.query(&query).await.map_err(map_db_err)?;
            let rows: Vec<CountRow> = resp.take(0).map_err(map_db_err)?;
            total += rows.into_iter().next().map_or(0, |r| r.count);
        }
        Ok(total)
    }

    // ── Batch concerns query (T096) ──────────────────────────────────

    /// List all `concerns` edges for multiple tasks in a single query.
    ///
    /// Returns a map of `task_id` → `Vec<ConcernsLink>`.
    pub async fn list_concerns_for_tasks(
        &self,
        task_ids: &[String],
    ) -> Result<std::collections::HashMap<String, Vec<ConcernsLink>>, EngramError> {
        let mut result: std::collections::HashMap<String, Vec<ConcernsLink>> =
            std::collections::HashMap::new();
        if task_ids.is_empty() {
            return Ok(result);
        }
        // Query each task individually; SurrealDB doesn't support IN on Thing lists easily.
        for task_id in task_ids {
            let links = self.list_concerns_for_task(task_id).await?;
            result.insert(task_id.clone(), links);
        }
        Ok(result)
    }

    // ── Content Record queries ──────────────────────────────────────

    /// Upsert a content record by file path, creating or replacing
    /// the existing record for that file.
    pub async fn upsert_content_record(
        &self,
        record: &crate::models::ContentRecord,
    ) -> Result<(), EngramError> {
        let thing = Thing::from(("content_record", record.id.as_str()));
        let ingested = record.ingested_at.to_rfc3339();
        self.db
            .query(
                "UPSERT $record SET \
                    content_type = $content_type, \
                    file_path = $file_path, \
                    content_hash = $content_hash, \
                    content = $content, \
                    embedding = $embedding, \
                    source_path = $source_path, \
                    file_size_bytes = $file_size_bytes, \
                    ingested_at = <datetime>$ingested",
            )
            .bind(("record", thing))
            .bind(("content_type", record.content_type.clone()))
            .bind(("file_path", record.file_path.clone()))
            .bind(("content_hash", record.content_hash.clone()))
            .bind(("content", record.content.clone()))
            .bind(("embedding", record.embedding.clone()))
            .bind(("source_path", record.source_path.clone()))
            .bind((
                "file_size_bytes",
                i64::try_from(record.file_size_bytes).unwrap_or(i64::MAX),
            ))
            .bind(("ingested", ingested))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Select all content records, optionally filtered by content type.
    pub async fn select_content_records(
        &self,
        content_type: Option<&str>,
    ) -> Result<Vec<crate::models::ContentRecord>, EngramError> {
        let rows: Vec<ContentRecordRow> = if let Some(ct) = content_type {
            self.db
                .query("SELECT * FROM content_record WHERE content_type = $ct ORDER BY file_path")
                .bind(("ct", ct.to_owned()))
                .await
                .map_err(map_db_err)?
                .take(0)
                .map_err(map_db_err)?
        } else {
            self.db
                .query("SELECT * FROM content_record ORDER BY file_path")
                .await
                .map_err(map_db_err)?
                .take(0)
                .map_err(map_db_err)?
        };
        Ok(rows.into_iter().map(ContentRecordRow::into_model).collect())
    }

    /// Delete a content record by file path.
    pub async fn delete_content_record_by_path(&self, file_path: &str) -> Result<(), EngramError> {
        self.db
            .query("DELETE FROM content_record WHERE file_path = $fp")
            .bind(("fp", file_path.to_owned()))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    // ── Commit Node queries ─────────────────────────────────────────

    /// Upsert a commit node by hash.
    pub async fn upsert_commit_node(
        &self,
        node: &crate::models::CommitNode,
    ) -> Result<(), EngramError> {
        let thing = Thing::from(("commit_node", node.id.as_str()));
        let ts = node.timestamp.to_rfc3339();
        let changes_json = serde_json::to_value(&node.changes).unwrap_or_default();
        self.db
            .query(
                "UPSERT $record SET \
                    hash = $hash, \
                    short_hash = $short_hash, \
                    author_name = $author_name, \
                    author_email = $author_email, \
                    timestamp = <datetime>$ts, \
                    message = $message, \
                    parent_hashes = $parent_hashes, \
                    changes = $changes",
            )
            .bind(("record", thing))
            .bind(("hash", node.hash.clone()))
            .bind(("short_hash", node.short_hash.clone()))
            .bind(("author_name", node.author_name.clone()))
            .bind(("author_email", node.author_email.clone()))
            .bind(("ts", ts))
            .bind(("message", node.message.clone()))
            .bind(("parent_hashes", node.parent_hashes.clone()))
            .bind(("changes", changes_json))
            .await
            .map_err(map_db_err)?;
        Ok(())
    }

    /// Select commit nodes within a date range, ordered by timestamp descending.
    pub async fn select_commits_by_date_range(
        &self,
        since: Option<&DateTime<Utc>>,
        until: Option<&DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<crate::models::CommitNode>, EngramError> {
        let effective_limit = if limit == 0 { 20 } else { limit };
        let rows: Vec<CommitNodeRow> = match (since, until) {
            (Some(s), Some(u)) => self
                .db
                .query(
                    "SELECT * FROM commit_node \
                         WHERE timestamp >= <datetime>$since AND timestamp <= <datetime>$until \
                         ORDER BY timestamp DESC LIMIT $lim",
                )
                .bind(("since", s.to_rfc3339()))
                .bind(("until", u.to_rfc3339()))
                .bind(("lim", effective_limit))
                .await
                .map_err(map_db_err)?
                .take(0)
                .map_err(map_db_err)?,
            (Some(s), None) => self
                .db
                .query(
                    "SELECT * FROM commit_node \
                         WHERE timestamp >= <datetime>$since \
                         ORDER BY timestamp DESC LIMIT $lim",
                )
                .bind(("since", s.to_rfc3339()))
                .bind(("lim", effective_limit))
                .await
                .map_err(map_db_err)?
                .take(0)
                .map_err(map_db_err)?,
            (None, Some(u)) => self
                .db
                .query(
                    "SELECT * FROM commit_node \
                         WHERE timestamp <= <datetime>$until \
                         ORDER BY timestamp DESC LIMIT $lim",
                )
                .bind(("until", u.to_rfc3339()))
                .bind(("lim", effective_limit))
                .await
                .map_err(map_db_err)?
                .take(0)
                .map_err(map_db_err)?,
            (None, None) => self
                .db
                .query("SELECT * FROM commit_node ORDER BY timestamp DESC LIMIT $lim")
                .bind(("lim", effective_limit))
                .await
                .map_err(map_db_err)?
                .take(0)
                .map_err(map_db_err)?,
        };
        Ok(rows.into_iter().map(CommitNodeRow::into_model).collect())
    }

    /// Select commit nodes that have a change record for a given file path.
    pub async fn select_commits_by_file_path(
        &self,
        file_path: &str,
        limit: u32,
    ) -> Result<Vec<crate::models::CommitNode>, EngramError> {
        let effective_limit = if limit == 0 { 20 } else { limit };
        let rows: Vec<CommitNodeRow> = self
            .db
            .query(
                "SELECT * FROM commit_node \
                 WHERE changes[WHERE file_path = $fp] != [] \
                 ORDER BY timestamp DESC LIMIT $lim",
            )
            .bind(("fp", file_path.to_owned()))
            .bind(("lim", effective_limit))
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;
        Ok(rows.into_iter().map(CommitNodeRow::into_model).collect())
    }

    /// Return the hash of the most recently indexed commit, if any.
    ///
    /// Used by the git graph service to resume incremental indexing without
    /// re-walking commits that are already stored.
    pub async fn latest_indexed_commit_hash(&self) -> Result<Option<String>, EngramError> {
        #[derive(serde::Deserialize)]
        struct HashRow {
            hash: String,
        }
        let rows: Vec<HashRow> = self
            .db
            .query("SELECT hash FROM commit_node ORDER BY timestamp DESC LIMIT 1")
            .await
            .map_err(map_db_err)?
            .take(0)
            .map_err(map_db_err)?;
        Ok(rows.into_iter().next().map(|r| r.hash))
    }
}

/// Information about a `concerns` edge targeting a symbol.
#[derive(Debug, Clone)]
pub struct ConcernsEdgeInfo {
    /// The task ID that has the concerns edge (e.g., `task:abc123`).
    pub task_id: String,
    /// Table name of the target symbol (function, class, interface).
    pub symbol_table: String,
    /// Full qualified ID of the target symbol.
    pub symbol_id: String,
    /// Name of the target symbol (populated by caller).
    pub symbol_name: String,
    /// Body hash of the target symbol (populated by caller).
    pub symbol_body_hash: String,
    /// The client/agent that created the link.
    pub linked_by: String,
}

/// Symbol identity tuple for hash-resilient concerns relinking (FR-124).
#[derive(Debug, Clone)]
pub struct SymbolIdentity {
    /// Table name (function, class, interface).
    pub table: String,
    /// Full qualified ID (e.g., `function:abc123`).
    pub id: String,
    /// Symbol name.
    pub name: String,
    /// Workspace-relative file path.
    pub file_path: String,
    /// Body hash for identity matching.
    pub body_hash: String,
}

/// Internal row for ID-only queries.
#[derive(Deserialize)]
struct IdOnlyRow {
    id: Thing,
}

/// Internal row for concerns edge queries.
#[derive(Deserialize)]
struct ConcernsRow {
    r#in: Thing,
    #[allow(dead_code)]
    out: Thing,
    #[serde(default)]
    linked_by: Option<String>,
}

/// Internal row for concerns edge listing (includes `out` for resolution).
#[derive(Deserialize)]
struct ConcernsLinkRow {
    #[allow(dead_code)]
    r#in: Thing,
    out: Thing,
    #[serde(default)]
    linked_by: Option<String>,
}

/// A resolved `concerns` edge link with symbol metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcernsLink {
    /// Table name of the target symbol (function, class, interface).
    pub symbol_table: String,
    /// Full qualified ID of the target symbol.
    pub symbol_id: String,
    /// Symbol name.
    pub symbol_name: String,
    /// File path of the symbol.
    pub file_path: String,
    /// The client/agent that created the link.
    pub linked_by: String,
}

/// Internal row for symbol identity queries.
#[derive(Deserialize)]
struct SymbolIdentityRow {
    id: Thing,
    name: String,
    file_path: String,
    body_hash: String,
}

// ── Supporting Types for BFS / Symbol Listing ──────────────────────────

/// A matched code symbol with full metadata.
#[derive(Debug, Clone)]
pub struct SymbolMatch {
    /// Table name (function, class, interface, code_file).
    pub table: String,
    /// Full qualified ID (e.g., `function:abc123`).
    pub id: String,
    /// Symbol name.
    pub name: String,
    /// Workspace-relative file path.
    pub file_path: String,
    /// 1-based start line, if applicable.
    pub line_start: Option<u32>,
    /// 1-based end line, if applicable.
    pub line_end: Option<u32>,
    /// Function signature, if applicable.
    pub signature: Option<String>,
    /// Full source body (may be empty until loaded from disk).
    pub body: String,
    /// Embedding type (`explicit_code` or `summary_pointer`).
    pub embed_type: Option<String>,
    /// Summary text.
    pub summary: Option<String>,
    /// Embedding vector.
    pub embedding: Vec<f32>,
}

/// An edge discovered during BFS traversal.
#[derive(Debug, Clone)]
pub struct BfsEdge {
    /// Edge type (calls, imports, defines, inherits_from).
    pub edge_type: String,
    /// Source node ID.
    pub from: String,
    /// Target node ID.
    pub to: String,
}

/// Result of a BFS neighborhood query.
#[derive(Debug)]
pub struct BfsResult {
    /// Neighbor nodes discovered.
    pub neighbors: Vec<SymbolMatch>,
    /// Edges connecting root to neighbors.
    pub edges: Vec<BfsEdge>,
    /// Whether the traversal was truncated at `max_nodes`.
    pub truncated: bool,
}

/// Filter criteria for `list_symbols`.
#[derive(Debug, Default)]
pub struct SymbolFilter {
    /// Filter by workspace-relative file path.
    pub file_path: Option<String>,
    /// Filter by node type (function, class, interface).
    pub node_type: Option<String>,
    /// Filter by name prefix.
    pub name_prefix: Option<String>,
    /// Maximum results per page.
    pub limit: usize,
    /// Offset for pagination.
    pub offset: usize,
}

/// A single entry in a `list_symbols` result.
#[derive(Debug, Clone, Serialize)]
pub struct SymbolListEntry {
    /// Symbol name.
    pub name: String,
    /// Node type (function, class, interface).
    #[serde(rename = "type")]
    pub node_type: String,
    /// Workspace-relative file path.
    pub file_path: String,
    /// 1-based start line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<u32>,
    /// 1-based end line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<u32>,
}

/// Result of a `list_symbols` query.
#[derive(Debug)]
pub struct SymbolListResult {
    /// The matched symbols for this page.
    pub symbols: Vec<SymbolListEntry>,
    /// Total count of matching symbols (for pagination).
    pub total_count: usize,
    /// Whether more results exist beyond limit+offset.
    pub has_more: bool,
}

/// Internal row for outbound edge queries.
#[derive(Deserialize)]
struct OutEdgeRow {
    out: Thing,
}

/// Internal row for inbound edge queries.
#[derive(Deserialize)]
struct InEdgeRow {
    r#in: Thing,
}

/// Internal row for deserializing content records from SurrealDB.
#[derive(Deserialize)]
struct ContentRecordRow {
    id: Thing,
    content_type: String,
    file_path: String,
    content_hash: String,
    content: String,
    #[serde(default)]
    embedding: Option<Vec<f32>>,
    #[serde(default)]
    source_path: String,
    #[serde(default)]
    file_size_bytes: i64,
    #[serde(default)]
    ingested_at: Option<String>,
}

impl ContentRecordRow {
    fn into_model(self) -> crate::models::ContentRecord {
        crate::models::ContentRecord {
            id: self.id.id.to_raw(),
            content_type: self.content_type,
            file_path: self.file_path,
            content_hash: self.content_hash,
            content: self.content,
            embedding: self.embedding,
            source_path: self.source_path,
            file_size_bytes: u64::try_from(self.file_size_bytes).unwrap_or(0),
            ingested_at: self
                .ingested_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
        }
    }
}

/// Internal row for deserializing commit nodes from SurrealDB.
#[derive(Deserialize)]
struct CommitNodeRow {
    id: Thing,
    hash: String,
    #[serde(default)]
    short_hash: String,
    #[serde(default)]
    author_name: String,
    #[serde(default)]
    author_email: String,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    message: String,
    #[serde(default)]
    parent_hashes: Vec<String>,
    #[serde(default)]
    changes: serde_json::Value,
}

impl CommitNodeRow {
    fn into_model(self) -> crate::models::CommitNode {
        let changes: Vec<crate::models::ChangeRecord> =
            serde_json::from_value(self.changes).unwrap_or_default();
        crate::models::CommitNode {
            id: self.id.id.to_raw(),
            hash: self.hash,
            short_hash: self.short_hash,
            author_name: self.author_name,
            author_email: self.author_email,
            timestamp: self
                .timestamp
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now),
            message: self.message,
            parent_hashes: self.parent_hashes,
            changes,
        }
    }
}

/// Parse a node ID like `function:abc123` into `("function", "abc123")`.
fn parse_node_id(node_id: &str) -> (String, String) {
    if let Some((table, raw_id)) = node_id.split_once(':') {
        (table.to_owned(), raw_id.to_owned())
    } else {
        ("unknown".to_owned(), node_id.to_owned())
    }
}

/// Parse a node ID string into a SurrealDB `Thing`.
fn parse_thing(node_id: &str) -> Thing {
    let (table, raw_id) = parse_node_id(node_id);
    Thing::from((table.as_str(), raw_id.as_str()))
}

#[cfg(test)]
mod tests {
    use crate::services::embedding::has_meaningful_embedding;

    // ── GAP-002: has_meaningful_embedding unit tests ─────────────────
    // Tests are against the canonical `services::embedding::has_meaningful_embedding`.

    #[test]
    fn meaningful_embedding_excludes_empty_vec() {
        assert!(!has_meaningful_embedding(&[]));
    }

    #[test]
    fn meaningful_embedding_excludes_zero_vectors() {
        assert!(!has_meaningful_embedding(&vec![0.0_f32; 384]));
    }

    #[test]
    fn meaningful_embedding_accepts_nonzero_vector() {
        let mut e = vec![0.0_f32; 384];
        e[100] = 0.01;
        assert!(has_meaningful_embedding(&e));
    }

    #[test]
    fn meaningful_embedding_accepts_small_nonzero() {
        // f32::MIN_POSITIVE < f32::EPSILON, so use a value above EPSILON threshold
        assert!(has_meaningful_embedding(&[0.0, 0.0, 2.0 * f32::EPSILON]));
    }
}
