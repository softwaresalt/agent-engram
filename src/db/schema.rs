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

/// Schema version constant for `.tmem/.version` file.
pub const SCHEMA_VERSION: &str = "2.0.0";
