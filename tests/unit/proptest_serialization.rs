//! Property-based tests for markdown serialization round-trip (T060).
//!
//! Verifies that tasks serialized to tasks.md format and parsed back
//! produce equivalent structured data — SC-008 compliance.
//! Also includes code graph model serde round-trips (T029).

use std::collections::HashMap;

use chrono::Utc;
use proptest::prelude::*;

use engram::models::class::Class;
use engram::models::code_edge::{CodeEdge, CodeEdgeType};
use engram::models::code_file::CodeFile;
use engram::models::function::Function;
use engram::models::interface::Interface;
use engram::models::task::{Task, TaskStatus, compute_priority_order};
use engram::services::dehydration::serialize_tasks_md;
use engram::services::hydration::parse_tasks_md;

fn arb_status() -> impl Strategy<Value = TaskStatus> {
    prop_oneof![
        Just(TaskStatus::Todo),
        Just(TaskStatus::InProgress),
        Just(TaskStatus::Done),
        Just(TaskStatus::Blocked),
    ]
}

fn arb_task() -> impl Strategy<Value = Task> {
    (
        "[a-z][a-z0-9]{2,8}",      // id suffix (alphanumeric, no colons)
        "[A-Za-z][A-Za-z ]{0,49}", // title (starts with letter, avoids whitespace-only)
        arb_status(),
        prop::option::of("[A-Z]{2}#[0-9]{1,5}"), // work_item_id
        "[A-Za-z0-9 .]{0,100}",                  // description (safe chars)
        prop::sample::select(vec!["p0", "p1", "p2", "p3", "p4"]),
    )
        .prop_map(
            |(id_suffix, title, status, work_item_id, description, priority)| {
                let now = Utc::now();
                let priority_order = compute_priority_order(priority);
                Task {
                    id: id_suffix,
                    title,
                    status,
                    work_item_id,
                    description,
                    context_summary: None,
                    priority: priority.to_owned(),
                    priority_order,
                    issue_type: "task".to_owned(),
                    assignee: None,
                    defer_until: None,
                    pinned: false,
                    compaction_level: 0,
                    compacted_at: None,
                    workflow_state: None,
                    workflow_id: None,
                    created_at: now,
                    updated_at: now,
                }
            },
        )
}

proptest! {
    #[test]
    fn round_trip_single_task(task in arb_task()) {
        let tasks = vec![task.clone()];
        let serialized = serialize_tasks_md(&tasks, &HashMap::new(), "", &HashMap::new());
        let parsed = parse_tasks_md(&serialized);

        prop_assert_eq!(parsed.len(), 1, "should parse exactly 1 task");
        let rt = &parsed[0].task;
        prop_assert_eq!(&rt.id, &task.id);
        prop_assert_eq!(rt.title.trim(), task.title.trim());
        prop_assert_eq!(rt.status, task.status);
        prop_assert_eq!(&rt.work_item_id, &task.work_item_id);
        // Description round-trip: trimming is acceptable
        prop_assert_eq!(rt.description.trim(), task.description.trim());
    }

    #[test]
    fn round_trip_multiple_tasks(
        tasks in prop::collection::vec(arb_task(), 1..5)
    ) {
        let serialized = serialize_tasks_md(&tasks, &HashMap::new(), "", &HashMap::new());
        let parsed = parse_tasks_md(&serialized);

        prop_assert_eq!(
            parsed.len(),
            tasks.len(),
            "parsed count should match input count"
        );

        for (original, parsed_task) in tasks.iter().zip(parsed.iter()) {
            prop_assert_eq!(&parsed_task.task.id, &original.id);
            prop_assert_eq!(parsed_task.task.status, original.status);
        }
    }

    #[test]
    fn round_trip_preserves_status(status in arb_status()) {
        let now = Utc::now();
        let task = Task {
            id: "test".to_string(),
            title: "Test task".to_string(),
            status,
            work_item_id: None,
            description: "Description".to_string(),
            context_summary: None,
            priority: "p2".to_owned(),
            priority_order: 2,
            issue_type: "task".to_owned(),
            assignee: None,
            defer_until: None,
            pinned: false,
            compaction_level: 0,
            compacted_at: None,
            workflow_state: None,
            workflow_id: None,
            created_at: now,
            updated_at: now,
        };
        let serialized = serialize_tasks_md(std::slice::from_ref(&task), &HashMap::new(), "", &HashMap::new());
        let parsed = parse_tasks_md(&serialized);
        prop_assert_eq!(parsed[0].task.status, status);
    }

    // ── T090: enhanced field round-trip ──────────────────────────

    #[test]
    fn round_trip_preserves_priority(
        priority in prop::sample::select(vec!["p0", "p1", "p2", "p3", "p4"])
    ) {
        let now = Utc::now();
        let task = Task {
            id: "pri".to_string(),
            title: "Priority test".to_string(),
            status: TaskStatus::Todo,
            work_item_id: None,
            description: "Desc".to_string(),
            context_summary: None,
            priority: priority.to_owned(),
            priority_order: compute_priority_order(priority),
            issue_type: "task".to_owned(),
            assignee: None,
            defer_until: None,
            pinned: false,
            compaction_level: 0,
            compacted_at: None,
            workflow_state: None,
            workflow_id: None,
            created_at: now,
            updated_at: now,
        };
        let serialized = serialize_tasks_md(
            std::slice::from_ref(&task),
            &HashMap::new(), "", &HashMap::new(),
        );
        let parsed = parse_tasks_md(&serialized);
        prop_assert_eq!(&parsed[0].task.priority, priority);
    }

    #[test]
    fn round_trip_preserves_issue_type(
        issue_type in prop::sample::select(vec!["task", "bug", "spike"])
    ) {
        let now = Utc::now();
        let task = Task {
            id: "typ".to_string(),
            title: "Type test".to_string(),
            status: TaskStatus::Todo,
            work_item_id: None,
            description: "Desc".to_string(),
            context_summary: None,
            priority: "p2".to_owned(),
            priority_order: 2,
            issue_type: issue_type.to_owned(),
            assignee: None,
            defer_until: None,
            pinned: false,
            compaction_level: 0,
            compacted_at: None,
            workflow_state: None,
            workflow_id: None,
            created_at: now,
            updated_at: now,
        };
        let serialized = serialize_tasks_md(
            std::slice::from_ref(&task),
            &HashMap::new(), "", &HashMap::new(),
        );
        let parsed = parse_tasks_md(&serialized);
        prop_assert_eq!(&parsed[0].task.issue_type, issue_type);
    }

    #[test]
    fn round_trip_preserves_pinned(pinned in proptest::bool::ANY) {
        let now = Utc::now();
        let task = Task {
            id: "pin".to_string(),
            title: "Pinned test".to_string(),
            status: TaskStatus::Todo,
            work_item_id: None,
            description: "Desc".to_string(),
            context_summary: None,
            priority: "p2".to_owned(),
            priority_order: 2,
            issue_type: "task".to_owned(),
            assignee: None,
            defer_until: None,
            pinned,
            compaction_level: 0,
            compacted_at: None,
            workflow_state: None,
            workflow_id: None,
            created_at: now,
            updated_at: now,
        };
        let serialized = serialize_tasks_md(
            std::slice::from_ref(&task),
            &HashMap::new(), "", &HashMap::new(),
        );
        let parsed = parse_tasks_md(&serialized);
        prop_assert_eq!(parsed[0].task.pinned, pinned);
    }

    #[test]
    fn round_trip_preserves_assignee(
        assignee in prop::option::of("[a-z]{3,10}")
    ) {
        let now = Utc::now();
        let task = Task {
            id: "asg".to_string(),
            title: "Assignee test".to_string(),
            status: TaskStatus::Todo,
            work_item_id: None,
            description: "Desc".to_string(),
            context_summary: None,
            priority: "p2".to_owned(),
            priority_order: 2,
            issue_type: "task".to_owned(),
            assignee: assignee.clone(),
            defer_until: None,
            pinned: false,
            compaction_level: 0,
            compacted_at: None,
            workflow_state: None,
            workflow_id: None,
            created_at: now,
            updated_at: now,
        };
        let serialized = serialize_tasks_md(
            std::slice::from_ref(&task),
            &HashMap::new(), "", &HashMap::new(),
        );
        let parsed = parse_tasks_md(&serialized);
        prop_assert_eq!(&parsed[0].task.assignee, &assignee);
    }

    #[test]
    fn round_trip_preserves_compaction_level(
        level in 0u32..5
    ) {
        let now = Utc::now();
        let task = Task {
            id: "cmp".to_string(),
            title: "Compaction test".to_string(),
            status: TaskStatus::Done,
            work_item_id: None,
            description: "Desc".to_string(),
            context_summary: None,
            priority: "p2".to_owned(),
            priority_order: 2,
            issue_type: "task".to_owned(),
            assignee: None,
            defer_until: None,
            pinned: false,
            compaction_level: level,
            compacted_at: if level > 0 { Some(now) } else { None },
            workflow_state: None,
            workflow_id: None,
            created_at: now,
            updated_at: now,
        };
        let serialized = serialize_tasks_md(
            std::slice::from_ref(&task),
            &HashMap::new(), "", &HashMap::new(),
        );
        let parsed = parse_tasks_md(&serialized);
        prop_assert_eq!(parsed[0].task.compaction_level, level);
    }

    #[test]
    fn round_trip_preserves_labels(
        labels in prop::collection::vec("[a-z]{2,8}", 0..4)
    ) {
        let now = Utc::now();
        let task = Task {
            id: "lbl".to_string(),
            title: "Labels test".to_string(),
            status: TaskStatus::Todo,
            work_item_id: None,
            description: "Desc".to_string(),
            context_summary: None,
            priority: "p2".to_owned(),
            priority_order: 2,
            issue_type: "task".to_owned(),
            assignee: None,
            defer_until: None,
            pinned: false,
            compaction_level: 0,
            compacted_at: None,
            workflow_state: None,
            workflow_id: None,
            created_at: now,
            updated_at: now,
        };
        let mut task_labels = HashMap::new();
        task_labels.insert("lbl".to_string(), labels.clone());
        let serialized = serialize_tasks_md(
            std::slice::from_ref(&task),
            &HashMap::new(), "", &task_labels,
        );
        let parsed = parse_tasks_md(&serialized);
        prop_assert_eq!(&parsed[0].labels, &labels);
    }
}

/// Non-proptest T090: full enhanced task round-trip with all new fields set.
#[test]
fn t090_round_trip_all_enhanced_fields() {
    let now = Utc::now();
    let task = Task {
        id: "enhanced".to_string(),
        title: "Full enhanced round-trip".to_string(),
        status: TaskStatus::Done,
        work_item_id: Some("WI#12345".to_string()),
        description: "Complete enhanced task for testing".to_string(),
        context_summary: None,
        priority: "p1".to_owned(),
        priority_order: 1,
        issue_type: "bug".to_owned(),
        assignee: Some("agent-test".to_string()),
        defer_until: None,
        pinned: true,
        compaction_level: 2,
        compacted_at: Some(now),
        workflow_state: None,
        workflow_id: None,
        created_at: now,
        updated_at: now,
    };
    let mut task_labels = HashMap::new();
    task_labels.insert(
        "enhanced".to_string(),
        vec!["frontend".to_string(), "urgent".to_string()],
    );

    let serialized = serialize_tasks_md(
        std::slice::from_ref(&task),
        &HashMap::new(),
        "",
        &task_labels,
    );
    let parsed = parse_tasks_md(&serialized);

    assert_eq!(parsed.len(), 1);
    let rt = &parsed[0].task;
    assert_eq!(rt.id, "enhanced");
    assert_eq!(rt.title, "Full enhanced round-trip");
    assert_eq!(rt.status, TaskStatus::Done);
    assert_eq!(rt.work_item_id.as_deref(), Some("WI#12345"));
    assert_eq!(rt.priority, "p1");
    assert_eq!(rt.issue_type, "bug");
    assert_eq!(rt.assignee.as_deref(), Some("agent-test"));
    assert!(rt.pinned);
    assert_eq!(rt.compaction_level, 2);
    assert!(rt.compacted_at.is_some());
    assert_eq!(parsed[0].labels, vec!["frontend", "urgent"]);
}

// --- Code graph model serde round-trip tests (T029) ---

fn arb_code_file_serde() -> impl Strategy<Value = CodeFile> {
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

fn arb_embedding_serde() -> impl Strategy<Value = Vec<f32>> {
    prop::collection::vec(-1.0f32..1.0f32, 384..=384)
}

fn arb_function_serde() -> impl Strategy<Value = Function> {
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
            arb_embedding_serde(),
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

fn arb_class_serde() -> impl Strategy<Value = Class> {
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
        arb_embedding_serde(),
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

fn arb_interface_serde() -> impl Strategy<Value = Interface> {
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
        arb_embedding_serde(),
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

fn arb_code_edge_type_serde() -> impl Strategy<Value = CodeEdgeType> {
    prop_oneof![
        Just(CodeEdgeType::Calls),
        Just(CodeEdgeType::Imports),
        Just(CodeEdgeType::InheritsFrom),
        Just(CodeEdgeType::Defines),
        Just(CodeEdgeType::Concerns),
    ]
}

fn arb_code_edge_serde() -> impl Strategy<Value = CodeEdge> {
    (
        arb_code_edge_type_serde(),
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
    fn code_file_serde_roundtrip(cf in arb_code_file_serde()) {
        let json = serde_json::to_string(&cf).unwrap();
        let decoded: CodeFile = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(cf, decoded);
    }

    #[test]
    fn function_serde_roundtrip(f in arb_function_serde()) {
        let json = serde_json::to_string(&f).unwrap();
        let decoded: Function = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(f, decoded);
    }

    #[test]
    fn class_serde_roundtrip(c in arb_class_serde()) {
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Class = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(c, decoded);
    }

    #[test]
    fn interface_serde_roundtrip(i in arb_interface_serde()) {
        let json = serde_json::to_string(&i).unwrap();
        let decoded: Interface = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(i, decoded);
    }

    #[test]
    fn code_edge_serde_roundtrip(edge in arb_code_edge_serde()) {
        let json = serde_json::to_string(&edge).unwrap();
        let decoded: CodeEdge = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(edge, decoded);
    }

    #[test]
    fn code_edge_type_serde_roundtrip(et in arb_code_edge_type_serde()) {
        let json = serde_json::to_string(&et).unwrap();
        let decoded: CodeEdgeType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(et, decoded);
    }
}
