use chrono::Utc;
use proptest::prelude::*;

use t_mem::models::task::{Task, TaskStatus};

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
        "task:[a-z0-9]{6}".prop_map(|s| s),
        ".{1,40}",
        arb_status(),
        prop::option::of("[A-Za-z0-9#:_-]{3,20}"),
        ".{0,120}",
        prop::option::of(".{0,80}"),
    )
        .prop_map(|(id, title, status, work_item_id, description, context_summary)| {
            let now = Utc::now();
            Task {
                id,
                title,
                status,
                work_item_id,
                description,
                context_summary,
                created_at: now,
                updated_at: now,
            }
        })
}

proptest! {
    #[test]
    fn task_roundtrip(task in arb_task()) {
        let json = serde_json::to_string(&task).unwrap();
        let decoded: Task = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(task, decoded);
    }
}
