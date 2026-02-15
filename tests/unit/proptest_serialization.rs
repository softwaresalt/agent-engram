//! Property-based tests for markdown serialization round-trip (T060).
//!
//! Verifies that tasks serialized to tasks.md format and parsed back
//! produce equivalent structured data — SC-008 compliance.

use std::collections::HashMap;

use chrono::Utc;
use proptest::prelude::*;

use t_mem::models::task::{Task, TaskStatus, compute_priority_order};
use t_mem::services::dehydration::serialize_tasks_md;
use t_mem::services::hydration::parse_tasks_md;

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
        .prop_map(|(id_suffix, title, status, work_item_id, description, priority)| {
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
        })
}

proptest! {
    #[test]
    fn round_trip_single_task(task in arb_task()) {
        let tasks = vec![task.clone()];
        let serialized = serialize_tasks_md(&tasks, &HashMap::new(), "");
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
        let serialized = serialize_tasks_md(&tasks, &HashMap::new(), "");
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
        let serialized = serialize_tasks_md(std::slice::from_ref(&task), &HashMap::new(), "");
        let parsed = parse_tasks_md(&serialized);
        prop_assert_eq!(parsed[0].task.status, status);
    }
}
