//! Task entity and status enum.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Task status values following the state machine defined in `data-model.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
}

impl TaskStatus {
    /// Return the canonical snake_case string for this status.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Blocked => "blocked",
        }
    }
}

/// An actionable unit of work within a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_item_id: Option<String>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_summary: Option<String>,

    // --- Enhanced fields (002-enhanced-task-management) ---
    /// Priority level string (e.g., `"p0"`, `"p2"`, `"p10"`). Defaults to `"p2"`.
    #[serde(default = "default_priority")]
    pub priority: String,
    /// Derived numeric sort key from `priority` suffix. Lower = higher priority.
    #[serde(default = "default_priority_order")]
    pub priority_order: u32,
    /// Classification: task, bug, spike, decision, milestone, or custom.
    #[serde(default = "default_issue_type")]
    pub issue_type: String,
    /// Claimant identity string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    /// When the task becomes eligible for ready-work.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_until: Option<DateTime<Utc>>,
    /// Whether task floats to top of ready-work queue.
    #[serde(default)]
    pub pinned: bool,
    /// Number of times this task has been compacted.
    #[serde(default)]
    pub compaction_level: u32,
    /// Timestamp of last compaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compacted_at: Option<DateTime<Utc>>,

    // --- Reserved fields (v1 workflow engine) ---
    /// Reserved for v1 workflow engine state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_state: Option<String>,
    /// Reserved for v1 workflow engine identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_priority() -> String {
    "p2".to_owned()
}

const fn default_priority_order() -> u32 {
    2
}

fn default_issue_type() -> String {
    "task".to_owned()
}

/// Compute the numeric sort key from a priority string.
///
/// Parses the trailing digit(s) from the priority (e.g., `"p0"` → `0`,
/// `"p10"` → `10`). Returns `u32::MAX` if no numeric suffix is found.
pub fn compute_priority_order(priority: &str) -> u32 {
    let digits: String = priority.chars().rev().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return u32::MAX;
    }
    let digits: String = digits.chars().rev().collect();
    digits.parse().unwrap_or(u32::MAX)
}
