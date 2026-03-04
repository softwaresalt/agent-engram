use chrono::Utc;
use proptest::prelude::*;

use engram::models::class::Class;
use engram::models::code_edge::{CodeEdge, CodeEdgeType};
use engram::models::code_file::CodeFile;
use engram::models::comment::Comment;
use engram::models::config::{BatchConfig, CodeGraphConfig, CompactionConfig, WorkspaceConfig};
use engram::models::function::Function;
use engram::models::graph::DependencyType;
use engram::models::interface::Interface;
use engram::models::label::Label;
use engram::models::task::{Task, TaskStatus};

fn arb_status() -> impl Strategy<Value = TaskStatus> {
    prop_oneof![
        Just(TaskStatus::Todo),
        Just(TaskStatus::InProgress),
        Just(TaskStatus::Done),
        Just(TaskStatus::Blocked),
    ]
}

fn arb_dependency_type() -> impl Strategy<Value = DependencyType> {
    prop_oneof![
        Just(DependencyType::HardBlocker),
        Just(DependencyType::SoftDependency),
        Just(DependencyType::ChildOf),
        Just(DependencyType::BlockedBy),
        Just(DependencyType::DuplicateOf),
        Just(DependencyType::RelatedTo),
        Just(DependencyType::Predecessor),
        Just(DependencyType::Successor),
    ]
}

fn arb_task() -> impl Strategy<Value = Task> {
    (
        "task:[a-z0-9]{6}".prop_map(|s| s),
        ".{1,40}",
        arb_status(),
        prop::option::of("[A-Za-z0-9#:_-]{3,20}"),
        ".{0,120}",
        prop::option::of(".{0,80}"),
        prop::sample::select(vec!["p0", "p1", "p2", "p3", "p4"]),
        prop::sample::select(vec!["task", "bug", "spike", "decision", "milestone"]),
        prop::option::of("[a-z]{3,10}"),
        prop::bool::ANY,
        0..5u32,
    )
        .prop_map(
            |(
                id,
                title,
                status,
                work_item_id,
                description,
                context_summary,
                priority,
                issue_type,
                assignee,
                pinned,
                compaction_level,
            )| {
                let now = Utc::now();
                let priority_order = engram::models::task::compute_priority_order(priority);
                Task {
                    id,
                    title,
                    status,
                    work_item_id,
                    description,
                    context_summary,
                    priority: priority.to_owned(),
                    priority_order,
                    issue_type: issue_type.to_owned(),
                    assignee,
                    defer_until: None,
                    pinned,
                    compaction_level,
                    compacted_at: None,
                    workflow_state: None,
                    workflow_id: None,
                    created_at: now,
                    updated_at: now,
                }
            },
        )
}

fn arb_label() -> impl Strategy<Value = Label> {
    ("label:[a-z0-9]{6}", "task:[a-z0-9]{6}", "[a-z]{1,20}").prop_map(|(id, task_id, name)| Label {
        id,
        task_id,
        name,
        created_at: Utc::now(),
    })
}

fn arb_comment() -> impl Strategy<Value = Comment> {
    (
        "comment:[a-z0-9]{6}",
        "task:[a-z0-9]{6}",
        ".{1,100}",
        "[a-z]{3,15}",
    )
        .prop_map(|(id, task_id, content, author)| Comment {
            id,
            task_id,
            content,
            author,
            created_at: Utc::now(),
        })
}

fn arb_workspace_config() -> impl Strategy<Value = WorkspaceConfig> {
    (
        prop::sample::select(vec!["p0", "p1", "p2", "p3", "p4"]),
        1..30u32,
        1..100u32,
        50..1000u32,
        1..500u32,
    )
        .prop_map(
            |(priority, threshold, max_cand, trunc, batch)| WorkspaceConfig {
                default_priority: priority.to_owned(),
                compaction: CompactionConfig {
                    threshold_days: threshold,
                    max_candidates: max_cand,
                    truncation_length: trunc,
                },
                batch: BatchConfig { max_size: batch },
                allowed_labels: vec![],
                allowed_types: vec![],
                code_graph: CodeGraphConfig::default(),
            },
        )
}

fn arb_code_file() -> impl Strategy<Value = CodeFile> {
    (
        "code_file:[a-f0-9]{8}",
        "src/[a-z]{3,10}\\.rs",
        Just("rust".to_owned()),
        100..100_000u64,
        "[a-f0-9]{64}",
    )
        .prop_map(|(id, path, language, size_bytes, content_hash)| CodeFile {
            id,
            path,
            language,
            size_bytes,
            content_hash,
            last_indexed_at: Utc::now().to_rfc3339(),
        })
}

fn arb_embedding() -> impl Strategy<Value = Vec<f32>> {
    prop::collection::vec(-1.0f32..1.0f32, 384..=384)
}

fn arb_function() -> impl Strategy<Value = Function> {
    (
        (
            "function:[a-z0-9]{8}",
            "[a-z_]{3,20}",
            "src/[a-z]{3,10}\\.rs",
            1..500u32,
            1..500u32,
            "fn [a-z_]{3,20}\\(\\)",
            prop::option::of(".{5,60}"),
        ),
        (
            ".{10,200}",
            "[a-f0-9]{64}",
            1..500u32,
            prop::sample::select(vec!["explicit_code", "summary_pointer"]),
            arb_embedding(),
            ".{10,100}",
        ),
    )
        .prop_map(
            |(
                (id, name, file_path, line_start, line_end_offset, signature, docstring),
                (body, body_hash, token_count, embed_type, embedding, summary),
            )| {
                Function {
                    id,
                    name,
                    file_path,
                    line_start,
                    line_end: line_start + line_end_offset,
                    signature,
                    docstring,
                    body,
                    body_hash,
                    token_count,
                    embed_type: embed_type.to_owned(),
                    embedding,
                    summary,
                }
            },
        )
}

fn arb_class() -> impl Strategy<Value = Class> {
    (
        "class:[a-z0-9]{8}",
        "[A-Z][a-z]{2,15}",
        "src/[a-z]{3,10}\\.rs",
        1..500u32,
        1..500u32,
        prop::option::of(".{5,60}"),
        ".{10,200}",
        "[a-f0-9]{64}",
        1..500u32,
        prop::sample::select(vec!["explicit_code", "summary_pointer"]),
        arb_embedding(),
        ".{10,100}",
    )
        .prop_map(
            |(
                id,
                name,
                file_path,
                line_start,
                line_end_offset,
                docstring,
                body,
                body_hash,
                token_count,
                embed_type,
                embedding,
                summary,
            )| {
                Class {
                    id,
                    name,
                    file_path,
                    line_start,
                    line_end: line_start + line_end_offset,
                    docstring,
                    body,
                    body_hash,
                    token_count,
                    embed_type: embed_type.to_owned(),
                    embedding,
                    summary,
                }
            },
        )
}

fn arb_interface() -> impl Strategy<Value = Interface> {
    (
        "interface:[a-z0-9]{8}",
        "[A-Z][a-z]{2,15}",
        "src/[a-z]{3,10}\\.rs",
        1..500u32,
        1..500u32,
        prop::option::of(".{5,60}"),
        ".{10,200}",
        "[a-f0-9]{64}",
        1..500u32,
        prop::sample::select(vec!["explicit_code", "summary_pointer"]),
        arb_embedding(),
        ".{10,100}",
    )
        .prop_map(
            |(
                id,
                name,
                file_path,
                line_start,
                line_end_offset,
                docstring,
                body,
                body_hash,
                token_count,
                embed_type,
                embedding,
                summary,
            )| {
                Interface {
                    id,
                    name,
                    file_path,
                    line_start,
                    line_end: line_start + line_end_offset,
                    docstring,
                    body,
                    body_hash,
                    token_count,
                    embed_type: embed_type.to_owned(),
                    embedding,
                    summary,
                }
            },
        )
}

fn arb_code_edge_type() -> impl Strategy<Value = CodeEdgeType> {
    prop_oneof![
        Just(CodeEdgeType::Calls),
        Just(CodeEdgeType::Imports),
        Just(CodeEdgeType::InheritsFrom),
        Just(CodeEdgeType::Defines),
        Just(CodeEdgeType::Concerns),
    ]
}

fn arb_code_edge() -> impl Strategy<Value = CodeEdge> {
    (
        arb_code_edge_type(),
        "function:[a-z0-9]{8}",
        "function:[a-z0-9]{8}",
        prop::option::of("[a-z:]{5,30}"),
        prop::option::of("[a-z_]{3,15}"),
    )
        .prop_map(|(edge_type, from, to, import_path, linked_by)| CodeEdge {
            edge_type,
            from,
            to,
            import_path,
            linked_by,
            created_at: Utc::now().to_rfc3339(),
        })
}

proptest! {
    #[test]
    fn task_roundtrip(task in arb_task()) {
        let json = serde_json::to_string(&task).unwrap();
        let decoded: Task = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(task, decoded);
    }

    #[test]
    fn dependency_type_roundtrip(dt in arb_dependency_type()) {
        let json = serde_json::to_string(&dt).unwrap();
        let decoded: DependencyType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dt, decoded);
    }

    #[test]
    fn label_roundtrip(label in arb_label()) {
        let json = serde_json::to_string(&label).unwrap();
        let decoded: Label = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(label, decoded);
    }

    #[test]
    fn comment_roundtrip(comment in arb_comment()) {
        let json = serde_json::to_string(&comment).unwrap();
        let decoded: Comment = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(comment, decoded);
    }

    #[test]
    fn workspace_config_roundtrip(config in arb_workspace_config()) {
        let json = serde_json::to_string(&config).unwrap();
        let decoded: WorkspaceConfig = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(config, decoded);
    }

    #[test]
    fn code_file_roundtrip(cf in arb_code_file()) {
        let json = serde_json::to_string(&cf).unwrap();
        let decoded: CodeFile = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(cf, decoded);
    }

    #[test]
    fn function_roundtrip(f in arb_function()) {
        let json = serde_json::to_string(&f).unwrap();
        let decoded: Function = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(f, decoded);
    }

    #[test]
    fn class_roundtrip(c in arb_class()) {
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Class = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(c, decoded);
    }

    #[test]
    fn interface_roundtrip(i in arb_interface()) {
        let json = serde_json::to_string(&i).unwrap();
        let decoded: Interface = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(i, decoded);
    }

    #[test]
    fn code_edge_type_roundtrip(et in arb_code_edge_type()) {
        let json = serde_json::to_string(&et).unwrap();
        let decoded: CodeEdgeType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(et, decoded);
    }

    #[test]
    fn code_edge_roundtrip(edge in arb_code_edge()) {
        let json = serde_json::to_string(&edge).unwrap();
        let decoded: CodeEdge = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(edge, decoded);
    }
}
