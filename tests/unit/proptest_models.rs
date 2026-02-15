use chrono::Utc;
use proptest::prelude::*;

use t_mem::models::comment::Comment;
use t_mem::models::config::{BatchConfig, CompactionConfig, WorkspaceConfig};
use t_mem::models::graph::DependencyType;
use t_mem::models::label::Label;
use t_mem::models::task::{Task, TaskStatus};

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
            |(id, title, status, work_item_id, description, context_summary, priority, issue_type, assignee, pinned, compaction_level)| {
                let now = Utc::now();
                let priority_order = t_mem::models::task::compute_priority_order(priority);
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
    (
        "label:[a-z0-9]{6}",
        "task:[a-z0-9]{6}",
        "[a-z]{1,20}",
    )
        .prop_map(|(id, task_id, name)| Label {
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
        .prop_map(|(priority, threshold, max_cand, trunc, batch)| WorkspaceConfig {
            default_priority: priority.to_owned(),
            compaction: CompactionConfig {
                threshold_days: threshold,
                max_candidates: max_cand,
                truncation_length: trunc,
            },
            batch: BatchConfig { max_size: batch },
            allowed_labels: vec![],
            allowed_types: vec![],
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
}
