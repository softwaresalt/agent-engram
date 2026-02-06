#![allow(dead_code)]

/// SurrealDB schema definitions (DEFINE TABLE statements)
pub const DEFINE_SPEC: &str = r#"
DEFINE TABLE spec SCHEMAFULL;
DEFINE FIELD title ON TABLE spec TYPE string ASSERT $value != '';
DEFINE FIELD content ON TABLE spec TYPE string;
DEFINE FIELD embedding ON TABLE spec TYPE array<float>;
DEFINE FIELD file_path ON TABLE spec TYPE string;
DEFINE FIELD created_at ON TABLE spec TYPE datetime VALUE time::now();
DEFINE FIELD updated_at ON TABLE spec TYPE datetime VALUE time::now();
DEFINE INDEX spec_file_path ON TABLE spec COLUMNS file_path UNIQUE;
DEFINE INDEX spec_embedding ON TABLE spec COLUMNS embedding MTREE DIMENSION 384 TYPE COSINE;
"#;

pub const DEFINE_TASK: &str = r#"
DEFINE TABLE task SCHEMAFULL;
DEFINE FIELD title ON TABLE task TYPE string ASSERT $value != '';
DEFINE FIELD status ON TABLE task TYPE string ASSERT $value INSIDE ['todo','in_progress','done','blocked'];
DEFINE FIELD work_item_id ON TABLE task TYPE option<string>;
DEFINE FIELD description ON TABLE task TYPE string;
DEFINE FIELD context_summary ON TABLE task TYPE option<string>;
DEFINE FIELD created_at ON TABLE task TYPE datetime VALUE time::now();
DEFINE FIELD updated_at ON TABLE task TYPE datetime VALUE time::now();
DEFINE INDEX task_status ON TABLE task COLUMNS status;
DEFINE INDEX task_work_item ON TABLE task COLUMNS work_item_id;
DEFINE INDEX task_updated ON TABLE task COLUMNS updated_at;
"#;

pub const DEFINE_CONTEXT: &str = r#"
DEFINE TABLE context SCHEMAFULL;
DEFINE FIELD content ON TABLE context TYPE string ASSERT $value != '';
DEFINE FIELD embedding ON TABLE context TYPE array<float>;
DEFINE FIELD source_client ON TABLE context TYPE string;
DEFINE FIELD created_at ON TABLE context TYPE datetime VALUE time::now();
DEFINE INDEX context_source ON TABLE context COLUMNS source_client;
DEFINE INDEX context_created ON TABLE context COLUMNS created_at;
DEFINE INDEX context_embedding ON TABLE context COLUMNS embedding MTREE DIMENSION 384 TYPE COSINE;
"#;

pub const DEFINE_RELATIONSHIPS: &str = r#"
DEFINE TABLE depends_on SCHEMALESS TYPE RELATION;
DEFINE FIELD type ON TABLE depends_on TYPE string ASSERT $value INSIDE ['hard_blocker','soft_dependency'];

DEFINE TABLE implements SCHEMALESS TYPE RELATION;
DEFINE TABLE relates_to SCHEMALESS TYPE RELATION;
"#;
