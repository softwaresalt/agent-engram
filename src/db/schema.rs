//! SurrealDB schema definitions (DEFINE TABLE / FIELD / INDEX statements).
//!
//! All FIELD and INDEX definitions use `OVERWRITE` so that `ensure_schema`
//! acts as an idempotent migration — any schema change (e.g. switching a
//! field from `VALUE` to `DEFAULT`) is applied even on pre-existing databases.
//!
//! Timestamp semantics:
//! - `DEFAULT time::now()` — value is set on INSERT when the field is omitted,
//!   but explicit SET / UPDATE values are honoured. This is the correct
//!   behaviour for `created_at` (immutable after first write) and `updated_at`
//!   (explicitly set by every write path in `queries.rs`).

/// Code file node — a source file tracked in the code graph.
pub const DEFINE_CODE_FILE: &str = r#"
DEFINE TABLE IF NOT EXISTS code_file SCHEMAFULL;
DEFINE FIELD OVERWRITE path ON TABLE code_file TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE language ON TABLE code_file TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE size_bytes ON TABLE code_file TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE content_hash ON TABLE code_file TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE last_indexed_at ON TABLE code_file TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS code_file_path ON TABLE code_file COLUMNS path UNIQUE;
DEFINE INDEX IF NOT EXISTS code_file_language ON TABLE code_file COLUMNS language;
"#;

/// Function node — a callable code unit extracted via AST parsing.
///
/// The table name `function` is a reserved keyword in SurrealDB v2 and MUST
/// be backtick-escaped in all SurrealQL statements.
pub const DEFINE_FUNCTION: &str = r#"
DEFINE TABLE IF NOT EXISTS `function` SCHEMAFULL;
DEFINE FIELD OVERWRITE name ON TABLE `function` TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE file_path ON TABLE `function` TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE line_start ON TABLE `function` TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE line_end ON TABLE `function` TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE signature ON TABLE `function` TYPE string;
DEFINE FIELD OVERWRITE docstring ON TABLE `function` TYPE option<string>;
DEFINE FIELD OVERWRITE body_hash ON TABLE `function` TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE token_count ON TABLE `function` TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE embed_type ON TABLE `function` TYPE string ASSERT $value INSIDE ['explicit_code', 'summary_pointer'];
DEFINE FIELD OVERWRITE embedding ON TABLE `function` TYPE array<float>;
DEFINE FIELD OVERWRITE summary ON TABLE `function` TYPE string;
DEFINE INDEX IF NOT EXISTS function_name ON TABLE `function` COLUMNS name;
DEFINE INDEX IF NOT EXISTS function_file ON TABLE `function` COLUMNS file_path;
DEFINE INDEX IF NOT EXISTS function_embedding ON TABLE `function` COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
"#;

/// Class node — a type definition (struct in Rust) extracted via AST parsing.
pub const DEFINE_CLASS: &str = r#"
DEFINE TABLE IF NOT EXISTS class SCHEMAFULL;
DEFINE FIELD OVERWRITE name ON TABLE class TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE file_path ON TABLE class TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE line_start ON TABLE class TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE line_end ON TABLE class TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE docstring ON TABLE class TYPE option<string>;
DEFINE FIELD OVERWRITE body_hash ON TABLE class TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE token_count ON TABLE class TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE embed_type ON TABLE class TYPE string ASSERT $value INSIDE ['explicit_code', 'summary_pointer'];
DEFINE FIELD OVERWRITE embedding ON TABLE class TYPE array<float>;
DEFINE FIELD OVERWRITE summary ON TABLE class TYPE string;
DEFINE INDEX IF NOT EXISTS class_name ON TABLE class COLUMNS name;
DEFINE INDEX IF NOT EXISTS class_file ON TABLE class COLUMNS file_path;
DEFINE INDEX IF NOT EXISTS class_embedding ON TABLE class COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
"#;

/// Interface node — a trait definition extracted via AST parsing.
pub const DEFINE_INTERFACE: &str = r#"
DEFINE TABLE IF NOT EXISTS interface SCHEMAFULL;
DEFINE FIELD OVERWRITE name ON TABLE interface TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE file_path ON TABLE interface TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE line_start ON TABLE interface TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE line_end ON TABLE interface TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE docstring ON TABLE interface TYPE option<string>;
DEFINE FIELD OVERWRITE body_hash ON TABLE interface TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE token_count ON TABLE interface TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE embed_type ON TABLE interface TYPE string ASSERT $value INSIDE ['explicit_code', 'summary_pointer'];
DEFINE FIELD OVERWRITE embedding ON TABLE interface TYPE array<float>;
DEFINE FIELD OVERWRITE summary ON TABLE interface TYPE string;
DEFINE INDEX IF NOT EXISTS interface_name ON TABLE interface COLUMNS name;
DEFINE INDEX IF NOT EXISTS interface_file ON TABLE interface COLUMNS file_path;
DEFINE INDEX IF NOT EXISTS interface_embedding ON TABLE interface COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
"#;

/// Code graph edge tables — relationships between code nodes and across regions.
pub const DEFINE_CODE_EDGES: &str = r#"
DEFINE TABLE IF NOT EXISTS calls SCHEMALESS TYPE RELATION;
DEFINE FIELD OVERWRITE created_at ON TABLE calls TYPE datetime DEFAULT time::now();

DEFINE TABLE IF NOT EXISTS imports SCHEMALESS TYPE RELATION;
DEFINE FIELD OVERWRITE import_path ON TABLE imports TYPE string;
DEFINE FIELD OVERWRITE created_at ON TABLE imports TYPE datetime DEFAULT time::now();

DEFINE TABLE IF NOT EXISTS inherits_from SCHEMALESS TYPE RELATION;
DEFINE FIELD OVERWRITE created_at ON TABLE inherits_from TYPE datetime DEFAULT time::now();

DEFINE TABLE IF NOT EXISTS defines SCHEMALESS TYPE RELATION;
DEFINE FIELD OVERWRITE created_at ON TABLE defines TYPE datetime DEFAULT time::now();

DEFINE TABLE IF NOT EXISTS concerns SCHEMALESS TYPE RELATION;
DEFINE FIELD OVERWRITE linked_by ON TABLE concerns TYPE string;
DEFINE FIELD OVERWRITE created_at ON TABLE concerns TYPE datetime DEFAULT time::now();
"#;

/// Schema version constant for `.engram/.version` file.
pub const SCHEMA_VERSION: &str = "2.0.0";

/// Content record table — ingested workspace content partitioned by type.
///
/// Each record represents a single file's content from a registered source.
/// Records are unique by `file_path` within a workspace database.
pub const DEFINE_CONTENT_RECORD: &str = r#"
DEFINE TABLE IF NOT EXISTS content_record SCHEMAFULL;
DEFINE FIELD OVERWRITE content_type ON TABLE content_record TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE file_path ON TABLE content_record TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE content_hash ON TABLE content_record TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE content ON TABLE content_record TYPE string;
DEFINE FIELD OVERWRITE embedding ON TABLE content_record TYPE option<array<float>>;
DEFINE FIELD OVERWRITE source_path ON TABLE content_record TYPE string;
DEFINE FIELD OVERWRITE file_size_bytes ON TABLE content_record TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE ingested_at ON TABLE content_record TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS content_type_idx ON TABLE content_record COLUMNS content_type;
DEFINE INDEX IF NOT EXISTS content_file_idx ON TABLE content_record COLUMNS file_path UNIQUE;
"#;

/// Commit node table — git commits in the change graph.
///
/// Each record represents a git commit with its metadata, parent
/// references, and per-file change records embedded as an array.
pub const DEFINE_COMMIT_NODE: &str = r#"
DEFINE TABLE IF NOT EXISTS commit_node SCHEMAFULL;
DEFINE FIELD OVERWRITE hash ON TABLE commit_node TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE short_hash ON TABLE commit_node TYPE string;
DEFINE FIELD OVERWRITE author_name ON TABLE commit_node TYPE string;
DEFINE FIELD OVERWRITE author_email ON TABLE commit_node TYPE string;
DEFINE FIELD OVERWRITE timestamp ON TABLE commit_node TYPE datetime;
DEFINE FIELD OVERWRITE message ON TABLE commit_node TYPE string;
DEFINE FIELD OVERWRITE parent_hashes ON TABLE commit_node TYPE array DEFAULT [];
DEFINE FIELD OVERWRITE changes ON TABLE commit_node TYPE array DEFAULT [];
DEFINE INDEX IF NOT EXISTS commit_hash_idx ON TABLE commit_node COLUMNS hash UNIQUE;
DEFINE INDEX IF NOT EXISTS commit_time_idx ON TABLE commit_node COLUMNS timestamp;
"#;
