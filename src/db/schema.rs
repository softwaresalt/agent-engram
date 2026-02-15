/// SurrealDB schema definitions (DEFINE TABLE statements)
pub const DEFINE_SPEC: &str = r#"
DEFINE TABLE spec SCHEMAFULL;
DEFINE FIELD title ON TABLE spec TYPE string ASSERT $value != '';
DEFINE FIELD content ON TABLE spec TYPE string;
DEFINE FIELD embedding ON TABLE spec TYPE option<array<float>>;
DEFINE FIELD file_path ON TABLE spec TYPE string;
DEFINE FIELD created_at ON TABLE spec TYPE datetime VALUE time::now();
DEFINE FIELD updated_at ON TABLE spec TYPE datetime VALUE time::now();
DEFINE INDEX spec_file_path ON TABLE spec COLUMNS file_path UNIQUE;
DEFINE INDEX spec_embedding ON TABLE spec COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
"#;

pub const DEFINE_TASK: &str = r#"
DEFINE TABLE task SCHEMAFULL;
DEFINE FIELD title ON TABLE task TYPE string ASSERT $value != '';
DEFINE FIELD status ON TABLE task TYPE string ASSERT $value INSIDE ['todo','in_progress','done','blocked'];
DEFINE FIELD work_item_id ON TABLE task TYPE option<string>;
DEFINE FIELD description ON TABLE task TYPE string;
DEFINE FIELD context_summary ON TABLE task TYPE option<string>;
DEFINE FIELD priority ON TABLE task TYPE string DEFAULT 'p2';
DEFINE FIELD priority_order ON TABLE task TYPE int DEFAULT 2;
DEFINE FIELD issue_type ON TABLE task TYPE string DEFAULT 'task';
DEFINE FIELD assignee ON TABLE task TYPE option<string>;
DEFINE FIELD defer_until ON TABLE task TYPE option<datetime>;
DEFINE FIELD pinned ON TABLE task TYPE bool DEFAULT false;
DEFINE FIELD compaction_level ON TABLE task TYPE int DEFAULT 0;
DEFINE FIELD compacted_at ON TABLE task TYPE option<datetime>;
DEFINE FIELD workflow_state ON TABLE task TYPE option<string>;
DEFINE FIELD workflow_id ON TABLE task TYPE option<string>;
DEFINE FIELD created_at ON TABLE task TYPE datetime VALUE time::now();
DEFINE FIELD updated_at ON TABLE task TYPE datetime VALUE time::now();
DEFINE INDEX task_status ON TABLE task COLUMNS status;
DEFINE INDEX task_work_item ON TABLE task COLUMNS work_item_id;
DEFINE INDEX task_updated ON TABLE task COLUMNS updated_at;
DEFINE INDEX task_priority ON TABLE task COLUMNS priority_order;
DEFINE INDEX task_assignee ON TABLE task COLUMNS assignee;
DEFINE INDEX task_defer_until ON TABLE task COLUMNS defer_until;
DEFINE INDEX task_issue_type ON TABLE task COLUMNS issue_type;
DEFINE INDEX task_pinned ON TABLE task COLUMNS pinned;
DEFINE INDEX task_compaction ON TABLE task COLUMNS compaction_level;
"#;

pub const DEFINE_CONTEXT: &str = r#"
DEFINE TABLE context SCHEMAFULL;
DEFINE FIELD content ON TABLE context TYPE string ASSERT $value != '';
DEFINE FIELD embedding ON TABLE context TYPE option<array<float>>;
DEFINE FIELD source_client ON TABLE context TYPE string;
DEFINE FIELD created_at ON TABLE context TYPE datetime VALUE time::now();
DEFINE INDEX context_source ON TABLE context COLUMNS source_client;
DEFINE INDEX context_created ON TABLE context COLUMNS created_at;
DEFINE INDEX context_embedding ON TABLE context COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
"#;

pub const DEFINE_LABEL: &str = r#"
DEFINE TABLE label SCHEMAFULL;
DEFINE FIELD task_id ON TABLE label TYPE string;
DEFINE FIELD name ON TABLE label TYPE string ASSERT $value != '';
DEFINE FIELD created_at ON TABLE label TYPE datetime VALUE time::now();
DEFINE INDEX label_task_name ON TABLE label COLUMNS task_id, name UNIQUE;
DEFINE INDEX label_name ON TABLE label COLUMNS name;
"#;

pub const DEFINE_COMMENT: &str = r#"
DEFINE TABLE comment SCHEMAFULL;
DEFINE FIELD task_id ON TABLE comment TYPE string;
DEFINE FIELD content ON TABLE comment TYPE string ASSERT $value != '';
DEFINE FIELD author ON TABLE comment TYPE string ASSERT $value != '';
DEFINE FIELD created_at ON TABLE comment TYPE datetime VALUE time::now();
DEFINE INDEX comment_task ON TABLE comment COLUMNS task_id;
"#;

pub const DEFINE_RELATIONSHIPS: &str = r#"
DEFINE TABLE depends_on SCHEMALESS TYPE RELATION;
DEFINE FIELD type ON TABLE depends_on TYPE string ASSERT $value INSIDE ['hard_blocker','soft_dependency','child_of','blocked_by','duplicate_of','related_to','predecessor','successor'];

DEFINE TABLE implements SCHEMALESS TYPE RELATION;
DEFINE TABLE relates_to SCHEMALESS TYPE RELATION;
"#;

/// Schema version constant for `.tmem/.version` file.
pub const SCHEMA_VERSION: &str = "2.0.0";
