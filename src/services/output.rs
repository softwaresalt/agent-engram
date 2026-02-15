//! MCP output filtering utilities.
//!
//! Provides field filtering for `brief` and `fields` parameters
//! on read tool responses.

use serde_json::{Value, json};

use crate::models::task::Task;

/// Essential fields returned in brief mode (FR-055).
const BRIEF_FIELDS: &[&str] = &["id", "title", "status", "priority", "assignee"];

/// Serialize a task to JSON, optionally applying brief mode or field filtering.
///
/// When `brief` is true, only essential fields are returned.
/// When `fields` is provided, only those fields are included.
/// Otherwise, the full task is serialized.
pub fn serialize_task(task: &Task, brief: bool, fields: Option<&[String]>) -> Value {
    if brief {
        return json!({
            "id": task.id,
            "title": task.title,
            "status": task.status.as_str(),
            "priority": task.priority,
            "assignee": task.assignee,
        });
    }

    let full = full_task_json(task);

    if let Some(fields) = fields {
        filter_fields(full, fields)
    } else {
        full
    }
}

/// Filter a JSON object to keep only the specified fields.
pub fn filter_fields(value: Value, fields: &[String]) -> Value {
    if let Value::Object(obj) = value {
        let filtered: serde_json::Map<String, Value> = obj
            .into_iter()
            .filter(|(k, _)| fields.iter().any(|f| f == k))
            .collect();
        Value::Object(filtered)
    } else {
        value
    }
}

/// Apply brief or fields filtering to an arbitrary JSON value.
///
/// For task-graph nodes and check_status results, this filters
/// at the value level rather than requiring a Task struct.
pub fn filter_value(value: Value, brief: bool, fields: Option<&[String]>) -> Value {
    if brief {
        if let Value::Object(obj) = &value {
            let filtered: serde_json::Map<String, Value> = obj
                .iter()
                .filter(|(k, _)| BRIEF_FIELDS.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            return Value::Object(filtered);
        }
    }

    if let Some(fields) = fields {
        filter_fields(value, fields)
    } else {
        value
    }
}

/// Serialize a task to its full JSON representation.
fn full_task_json(task: &Task) -> Value {
    json!({
        "id": task.id,
        "title": task.title,
        "status": task.status.as_str(),
        "priority": task.priority,
        "priority_order": task.priority_order,
        "issue_type": task.issue_type,
        "assignee": task.assignee,
        "description": task.description,
        "context_summary": task.context_summary,
        "pinned": task.pinned,
        "defer_until": task.defer_until.map(|d| d.to_rfc3339()),
        "compaction_level": task.compaction_level,
        "created_at": task.created_at.to_rfc3339(),
        "updated_at": task.updated_at.to_rfc3339(),
    })
}
