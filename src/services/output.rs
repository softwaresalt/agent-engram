//! MCP output filtering utilities.
//!
//! Provides field filtering for `brief` and `fields` parameters
//! on read tool responses.

use serde_json::Value;

/// Essential fields returned in brief mode (FR-055).
const BRIEF_FIELDS: &[&str] = &["id", "title", "status", "priority", "assignee"];

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
