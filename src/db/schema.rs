/// SurrealDB schema definitions (DEFINE TABLE / FIELD / INDEX statements).
///
/// All FIELD and INDEX definitions use `OVERWRITE` so that `ensure_schema`
/// acts as an idempotent migration — any schema change (e.g. switching a
/// field from `VALUE` to `DEFAULT`) is applied even on pre-existing databases.
///
/// Timestamp semantics:
/// - `DEFAULT time::now()` — value is set on INSERT when the field is omitted,
///   but explicit SET / UPDATE values are honoured.  This is the correct
///   behaviour for `created_at` (immutable after first write) and `updated_at`
///   (explicitly set by every write path in `queries.rs`).
pub const DEFINE_SPEC: &str = r#"
DEFINE TABLE IF NOT EXISTS spec SCHEMAFULL;
DEFINE FIELD OVERWRITE title ON TABLE spec TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE content ON TABLE spec TYPE string;
DEFINE FIELD OVERWRITE embedding ON TABLE spec TYPE option<array<float>>;
DEFINE FIELD OVERWRITE file_path ON TABLE spec TYPE string;
DEFINE FIELD OVERWRITE created_at ON TABLE spec TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE spec TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS spec_file_path ON TABLE spec COLUMNS file_path UNIQUE;
DEFINE INDEX IF NOT EXISTS spec_embedding ON TABLE spec COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
"#;

pub const DEFINE_TASK: &str = r#"
DEFINE TABLE IF NOT EXISTS task SCHEMAFULL;
DEFINE FIELD OVERWRITE title ON TABLE task TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE status ON TABLE task TYPE string ASSERT $value INSIDE ['todo','in_progress','done','blocked'];
DEFINE FIELD OVERWRITE work_item_id ON TABLE task TYPE option<string>;
DEFINE FIELD OVERWRITE description ON TABLE task TYPE string;
DEFINE FIELD OVERWRITE context_summary ON TABLE task TYPE option<string>;
DEFINE FIELD OVERWRITE priority ON TABLE task TYPE string DEFAULT 'p2';
DEFINE FIELD OVERWRITE priority_order ON TABLE task TYPE int DEFAULT 2;
DEFINE FIELD OVERWRITE issue_type ON TABLE task TYPE string DEFAULT 'task';
DEFINE FIELD OVERWRITE assignee ON TABLE task TYPE option<string>;
DEFINE FIELD OVERWRITE defer_until ON TABLE task TYPE option<datetime>;
DEFINE FIELD OVERWRITE pinned ON TABLE task TYPE bool DEFAULT false;
DEFINE FIELD OVERWRITE compaction_level ON TABLE task TYPE int DEFAULT 0;
DEFINE FIELD OVERWRITE compacted_at ON TABLE task TYPE option<datetime>;
DEFINE FIELD OVERWRITE workflow_state ON TABLE task TYPE option<string>;
DEFINE FIELD OVERWRITE workflow_id ON TABLE task TYPE option<string>;
DEFINE FIELD OVERWRITE created_at ON TABLE task TYPE datetime DEFAULT time::now();
DEFINE FIELD OVERWRITE updated_at ON TABLE task TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS task_status ON TABLE task COLUMNS status;
DEFINE INDEX IF NOT EXISTS task_work_item ON TABLE task COLUMNS work_item_id;
DEFINE INDEX IF NOT EXISTS task_updated ON TABLE task COLUMNS updated_at;
DEFINE INDEX IF NOT EXISTS task_priority ON TABLE task COLUMNS priority_order;
DEFINE INDEX IF NOT EXISTS task_assignee ON TABLE task COLUMNS assignee;
DEFINE INDEX IF NOT EXISTS task_defer_until ON TABLE task COLUMNS defer_until;
DEFINE INDEX IF NOT EXISTS task_issue_type ON TABLE task COLUMNS issue_type;
DEFINE INDEX IF NOT EXISTS task_pinned ON TABLE task COLUMNS pinned;
DEFINE INDEX IF NOT EXISTS task_compaction ON TABLE task COLUMNS compaction_level;
"#;

pub const DEFINE_CONTEXT: &str = r#"
DEFINE TABLE IF NOT EXISTS context SCHEMAFULL;
DEFINE FIELD OVERWRITE content ON TABLE context TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE embedding ON TABLE context TYPE option<array<float>>;
DEFINE FIELD OVERWRITE source_client ON TABLE context TYPE string;
DEFINE FIELD OVERWRITE created_at ON TABLE context TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS context_source ON TABLE context COLUMNS source_client;
DEFINE INDEX IF NOT EXISTS context_created ON TABLE context COLUMNS created_at;
DEFINE INDEX IF NOT EXISTS context_embedding ON TABLE context COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
"#;

pub const DEFINE_LABEL: &str = r#"
DEFINE TABLE IF NOT EXISTS label SCHEMAFULL;
DEFINE FIELD OVERWRITE task_id ON TABLE label TYPE string;
DEFINE FIELD OVERWRITE name ON TABLE label TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE created_at ON TABLE label TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS label_task_name ON TABLE label COLUMNS task_id, name UNIQUE;
DEFINE INDEX IF NOT EXISTS label_name ON TABLE label COLUMNS name;
"#;

pub const DEFINE_COMMENT: &str = r#"
DEFINE TABLE IF NOT EXISTS comment SCHEMAFULL;
DEFINE FIELD OVERWRITE task_id ON TABLE comment TYPE string;
DEFINE FIELD OVERWRITE content ON TABLE comment TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE author ON TABLE comment TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE created_at ON TABLE comment TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS comment_task ON TABLE comment COLUMNS task_id;
"#;

pub const DEFINE_RELATIONSHIPS: &str = r#"
DEFINE TABLE IF NOT EXISTS depends_on SCHEMALESS TYPE RELATION;
DEFINE FIELD OVERWRITE type ON TABLE depends_on TYPE string ASSERT $value INSIDE ['hard_blocker','soft_dependency','child_of','blocked_by','duplicate_of','related_to','predecessor','successor'];

DEFINE TABLE IF NOT EXISTS implements SCHEMALESS TYPE RELATION;
DEFINE TABLE IF NOT EXISTS relates_to SCHEMALESS TYPE RELATION;
"#;

/// Code file node — a source file tracked in the code graph.
pub const DEFINE_CODE_FILE: &str = r#"
DEFINE TABLE IF NOT EXISTS code_file SCHEMAFULL;
DEFINE FIELD OVERWRITE path ON TABLE code_file TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE language ON TABLE code_file TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE size_bytes ON TABLE code_file TYPE int ASSERT $value > 0;
DEFINE FIELD OVERWRITE content_hash ON TABLE code_file TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE last_indexed_at ON TABLE code_file TYPE datetime DEFAULT time::now();
DEFINE INDEX IF NOT EXISTS code_file_path ON TABLE code_file COLUMNS path UNIQUE;
DEFINE INDEX IF NOT EXISTS code_file_language ON TABLE code_file COLUMNS language;
"#;

/// Function node — a callable code unit extracted via AST parsing.
pub const DEFINE_FUNCTION: &str = r#"
DEFINE TABLE IF NOT EXISTS function SCHEMAFULL;
DEFINE FIELD OVERWRITE name ON TABLE function TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE file_path ON TABLE function TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE line_start ON TABLE function TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE line_end ON TABLE function TYPE int ASSERT $value >= 1;
DEFINE FIELD OVERWRITE signature ON TABLE function TYPE string;
DEFINE FIELD OVERWRITE docstring ON TABLE function TYPE option<string>;
DEFINE FIELD OVERWRITE body_hash ON TABLE function TYPE string ASSERT $value != '';
DEFINE FIELD OVERWRITE token_count ON TABLE function TYPE int ASSERT $value >= 0;
DEFINE FIELD OVERWRITE embed_type ON TABLE function TYPE string ASSERT $value INSIDE ['explicit_code', 'summary_pointer'];
DEFINE FIELD OVERWRITE embedding ON TABLE function TYPE array<float>;
DEFINE FIELD OVERWRITE summary ON TABLE function TYPE string;
DEFINE INDEX IF NOT EXISTS function_name ON TABLE function COLUMNS name;
DEFINE INDEX IF NOT EXISTS function_file ON TABLE function COLUMNS file_path;
DEFINE INDEX IF NOT EXISTS function_embedding ON TABLE function COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
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
